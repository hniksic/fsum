use std::fs;
use std::path::Path;

use dashmap::DashMap;
use rayon::prelude::*;

#[derive(Debug, Default)]
struct State {
    seen: DashMap<(u64, u64), ()>,
}

impl State {
    pub fn seen(&self, meta: &fs::Metadata) -> bool {
        use std::os::unix::fs::MetadataExt;
        self.seen.insert((meta.dev(), meta.ino()), ()).is_some()
    }
}

fn log_error<E: std::fmt::Display>(path: &Path, e: E) {
    eprintln!("{}: {}", path.display(), e);
}

fn dir_size(dir: &Path, state: &State) -> u128 {
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

fn path_size_1(path: &Path, state: &State) -> std::io::Result<u128> {
    let mut metadata = path.symlink_metadata()?;
    if metadata.file_type().is_symlink() {
        if !path.exists() {
            // just ignore dangling symlinks
            return Ok(0);
        }
        metadata = path.metadata()?;
    }
    Ok(if state.seen(&metadata) {
        0
    } else if metadata.is_dir() {
        dir_size(&path, state)
    } else {
        metadata.len() as u128
    })
}

fn path_size(path: &Path, state: &State) -> u128 {
    path_size_1(path, state).unwrap_or_else(|e| {
        log_error(&path, e);
        0
    })
}

pub fn fsum<T>(args: impl IntoIterator<Item = T>) -> u128
where
    T: AsRef<Path>,
{
    let state = State::default();
    args.into_iter()
        .map(|p| path_size(p.as_ref(), &state))
        .sum()
}
