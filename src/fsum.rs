use std;
use std::path::{PathBuf};
use std::fs;
use std::io::Write;

use rayon::prelude::*;
use concurrent_hashmap::ConcHashMap;

// use u8 as value because ConcHashMap doesn't support zero-sized
// value types (it panics at run-time).
type MyMap = ConcHashMap<(u64, u64), u8>;

struct State {
    seen: MyMap,
}

impl State {
    pub fn seen(&self, meta: &fs::Metadata) -> bool {
        let st = meta as &std::os::unix::fs::MetadataExt;
        return self.seen.insert((st.dev(), st.ino()), 0).is_some();
    }
}

fn log_error<E: std::fmt::Display>(path: &PathBuf, e: E) {
    writeln!(std::io::stderr(), "{}: {}", path.display(), e).unwrap();
}

fn dir_size(dir: &PathBuf, state: &State) -> u64 {
    || -> std::io::Result<u64> {
        let size = try!(fs::read_dir(&dir))
            .filter_map(|res| res.map_err(|e| log_error(&dir, e)).ok())
            .map(|dirent| dirent.path())
            .collect::<Vec<_>>()
            .par_iter()
            .map(|p| path_size(p, state))
            .sum();
        Ok(size)
    }().unwrap_or_else(|e| { log_error(&dir, e); 0 })
 }

fn path_size(path: &PathBuf, state: &State) -> u64 {
    || -> std::io::Result<u64> {
        let meta_maybe = path.metadata();
        if path.read_link().is_ok() && meta_maybe.is_err() {
            // completely ignore dangling symlinks (don't even log error)
            return Ok(0);
        }
        let meta = try!(meta_maybe);
        let size =
            if state.seen(&meta) {
                0
            } else if meta.is_dir() {
                dir_size(&path, state)
            } else {
                meta.len()
            };
        Ok(size)
    }().unwrap_or_else(|e| { log_error(&path, e); 0 })
}

pub fn fsum(args: &mut Iterator<Item=PathBuf>) -> u64
{
    let state = State { seen: MyMap::new() };
    args.map(|p| path_size(&p, &state)).sum()
}
