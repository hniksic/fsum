use std::path::PathBuf;
use std::fs;
use std::env;
use std::thread;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashSet;

pub mod shared_channel {
    use std::sync::{Arc, Mutex};
    use std::sync::mpsc::{channel, Sender, Receiver, RecvError};

    /// A thread-safe wrapper around a `Receiver`.
    #[derive(Clone)]
    pub struct SharedReceiver<T>(Arc<Mutex<Receiver<T>>>);
    impl<T> Iterator for SharedReceiver<T> {
        type Item = T;
        /// Get the next item from the wrapped receiver.
        fn next(&mut self) -> Option<T> {
            self.recv().ok()
        }
    }
    impl<T> SharedReceiver<T> {
        pub fn recv(&mut self) -> Result<T, RecvError> {
            let guard = self.0.lock().unwrap();
            guard.recv()
        }
    }

    pub fn shared_channel<T>() -> (Sender<T>, SharedReceiver<T>) {
        let (sender, receiver) = channel();
        (sender, SharedReceiver(Arc::new(Mutex::new(receiver))))
    }
}


#[derive(Clone)]
enum Job {
    Path(PathBuf),
    Dir(PathBuf),
    Done,
}

struct State {
    seen: Mutex<HashSet<(u64, u64)>>,
    qsize: AtomicUsize,
}

fn path_size(path: PathBuf,
             schedule_work: &mpsc::Sender<Job>,
	     state: &State)
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
        state.qsize.fetch_add(1, Ordering::SeqCst);
        schedule_work.send(Job::Dir(path)).unwrap();
        0
    } else {
        meta.len()
    }
}

fn worker(schedule_work: mpsc::Sender<Job>,
          get_work: shared_channel::SharedReceiver<Job>,
	  state: Arc<State>) -> u64 {
    get_work.into_iter().map(
        |job| {
            let total = match job {
                Job::Path(path) => {
                    Some(path_size(path, &schedule_work, &state))
                }
                Job::Dir(dir) => {
                    Some(fs::read_dir(&dir).unwrap()
                         .map(|dirent|
                              path_size(dirent.unwrap().path(), &schedule_work, &state))
                         .sum())
                }
                Job::Done => {
                    schedule_work.send(Job::Done).unwrap();
                    None
                }
            };
            if state.qsize.fetch_sub(1, Ordering::SeqCst) == 1 {
                schedule_work.send(Job::Done).unwrap();
            }
            total
        }).take_while(Option::is_some).map(Option::unwrap).sum()
}

fn fsum(args: &mut Iterator<Item=PathBuf>) -> u64
{
    const THREADS_CNT: usize = 8;

    let (schedule_work, get_work) = shared_channel::shared_channel::<Job>();
    let args: Vec<_> = args.collect();
    let state = Arc::new(State {
        qsize: AtomicUsize::new(args.len()),
        seen: Mutex::new(HashSet::new()),
    });
    for path in args {
        schedule_work.send(Job::Path(path)).unwrap();
    }
    let threads: Vec<_> = (0..THREADS_CNT).map(|_| {
        let schedule_work = schedule_work.clone();
        let get_work = get_work.clone();
        let state = state.clone();
        thread::spawn(move || {
            worker(schedule_work, get_work, state)
        })
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
