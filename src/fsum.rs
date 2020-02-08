use std::fs;
use std::path::PathBuf;

use chashmap::CHashMap;
use rayon::prelude::*;

#[derive(Debug, Default)]
struct State {
    seen: CHashMap<(u64, u64), ()>,
}

impl State {
    pub fn seen(&self, meta: &fs::Metadata) -> bool {
        use std::os::unix::fs::MetadataExt;
        self.seen.insert((meta.dev(), meta.ino()), ()).is_some()
    }
}

fn log_error<E: std::fmt::Display>(path: &PathBuf, e: E) {
    eprintln!("{}: {}", path.display(), e);
}

fn dir_size(dir: &PathBuf, state: &State) -> u64 {
    match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|res| res.map_err(|e| log_error(&dir, e)).ok())
            .map(|dirent| dirent.path())
            .collect::<Vec<_>>()
            .par_iter()
            .map(|p| path_size(p, state))
            .sum(),
        Err(e) => {
            log_error(&dir, e);
            0
        }
    }
}

fn path_size(path: &PathBuf, state: &State) -> u64 {
    match path.metadata() {
        Ok(metadata) => {
            if state.seen(&metadata) {
                0
            } else if metadata.is_dir() {
                dir_size(&path, state)
            } else {
                metadata.len()
            }
        }
        Err(e) => {
            let is_symlink = path.read_link().is_ok();
            // don't log errors for symlinks, which are likely dangling --
            // just ignore them completely
            if !is_symlink {
                log_error(&path, e);
            }
            0
        }
    }
}

pub fn fsum(args: impl Iterator<Item = PathBuf>) -> u64 {
    args.map(|p| path_size(&p, &State::default())).sum()
}
