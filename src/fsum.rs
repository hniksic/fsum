use std::path::PathBuf;
use std::fs;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashSet;
use std::io::Write;
use std;

enum QItem<T> {
    Item(T),
    Done
}

#[derive(Debug, Clone)]
struct Queue<T> {
    tx: Sender<QItem<T>>,
    rx: Arc<Mutex<Receiver<QItem<T>>>>,
    qsize: Arc<AtomicUsize>,
}

impl<T> Queue<T> {
    pub fn new() -> Queue<T> {
        let (tx, rx) = mpsc::channel::<QItem<T>>();
        Queue {
            rx: Arc::new(Mutex::new(rx)),
            tx: tx,
            qsize: Arc::new(AtomicUsize::new(0)),
        }
    }

    // Note: we own the receiver and know it won't hang up, so
    // self.tx.send(...).unwrap() should be safe.

    pub fn append(&self, item: T) {
        self.qsize.fetch_add(1, Ordering::SeqCst);
        self.tx.send(QItem::Item(item)).unwrap();
    }

    pub fn task_done(&self) {
        match self.qsize.fetch_sub(1, Ordering::SeqCst) {
            0 => panic!("task_done called on empty queue"),
            1 => self.tx.send(QItem::Done).unwrap(),
            _ => ()
        }
    }
}

impl<T> Iterator for Queue<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.qsize.load(Ordering::SeqCst) == 0 {
            return None
        }
        match self.rx.lock().unwrap().recv().unwrap() {
            QItem::Item(item) => Some(item),
            QItem::Done => {
                self.tx.send(QItem::Done).unwrap();
                None
            }
        }
    }
}


#[derive(Clone)]
enum Job {
    Path(PathBuf),
    Dir(PathBuf),
}

struct State {
    seen: Mutex<HashSet<(u64, u64)>>,
}

fn log_error<E: std::fmt::Display>(path: &PathBuf, e: E) {
    writeln!(std::io::stderr(), "{}: {}", path.display(), e).unwrap();
}

fn path_size(path: PathBuf, queue: &Queue<Job>, state: &State)
    -> u64
{
    (|| -> std::io::Result<u64> {
        let meta = try!(fs::symlink_metadata(&path));

        let st = &meta as &std::os::unix::fs::MetadataExt;
        {
            let mut seen = state.seen.lock().unwrap();
            if !seen.insert((st.dev(), st.ino())) {
                return Ok(0);
            }
        }

        if meta.is_dir() {
            queue.append(Job::Dir(path.clone()));
            Ok(0)
        } else {
            Ok(meta.len())
        }
    })().unwrap_or_else(|e| { log_error(&path, e); 0 })
}


fn worker(queue: Queue<Job>, state: Arc<State>) -> u64 {
    let q2 = queue.clone();
    queue.map(
        |job| match job {
            Job::Path(path) => path_size(path, &q2, &state),
            Job::Dir(dir)
                => (|| -> std::io::Result<u64> {
                    Ok(try!(fs::read_dir(&dir))
                        .filter_map(|res| res.map_err(|e| log_error(&dir, e)).ok())
                        .map(|dirent|
                             path_size(dirent.path(), &q2, &state))
                        .sum())
                })().unwrap_or_else(|e| { log_error(&dir, e); 0 }),
        }).inspect(|_| q2.task_done())
        .sum()
}

pub fn fsum(args: &mut Iterator<Item=PathBuf>) -> u64
{
    const THREADS_CNT: usize = 8;

    let queue = Queue::<Job>::new();
    let args: Vec<_> = args.collect();
    let state = Arc::new(State {
        seen: Mutex::new(HashSet::new()),
    });
    for path in args {
        queue.append(Job::Path(path));
    }
    let threads: Vec<_> = (0..THREADS_CNT).map(|_| {
        let queue = queue.clone();
        let state = state.clone();
        thread::spawn(move || worker(queue, state))
    }).collect();

    threads.into_iter()
        .map(thread::JoinHandle::join).map(Result::unwrap)
        .sum()
}
