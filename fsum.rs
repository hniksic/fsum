use std::path::PathBuf;
use std::fs;
use std::env;
use std::thread;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashSet;

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

fn path_size(path: PathBuf, queue: &Queue<Job>, state: &State)
    -> u64
{
    let meta = fs::symlink_metadata(&path).unwrap();

    let st = &meta as &std::os::unix::fs::MetadataExt;
    {
        let mut seen = state.seen.lock().unwrap();
        if !seen.insert((st.dev(), st.ino())) {
            return 0;
        }
    }

    if meta.is_dir() {
        queue.append(Job::Dir(path));
        0
    } else {
        meta.len()
    }
}

fn worker(queue: Queue<Job>, state: Arc<State>) -> u64 {
    let q2 = queue.clone();
    queue.map(
        |job| match job {
            Job::Path(path) => path_size(path, &q2, &state),
            Job::Dir(dir)   => (fs::read_dir(&dir).unwrap()
                                .map(|dirent|
                                     path_size(dirent.unwrap().path(), &q2, &state))
                                .sum())
        }).inspect(|_| q2.task_done())
        .sum()
}

fn fsum(args: &mut Iterator<Item=PathBuf>) -> u64
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

    threads.into_iter().map(|t| t.join().unwrap()).sum()
}


fn main()
{
    let size = fsum(&mut env::args_os().skip(1).map(PathBuf::from));
    println!("{}", size);
    for &(power, digits, letter) in [(1<<10, 0, "K"), (1<<20, 2, "M"), (1<<30, 2, "G")].iter() {
        if size >= power {
            println!("{:.*} {}", digits, size as f64 / power as f64, letter)
        }
    }
}
