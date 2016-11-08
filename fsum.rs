use std::collections::{HashSet, VecDeque};
use std::path::{PathBuf};
use std::fs;
use std::env;
use std::io;
use std::io::Write;
use std::fmt;

struct FileError {
    err: io::Error,
    filename: PathBuf,
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.filename.display(), self.err)
    }
}

trait Contextify<T> {
    fn imbue_err(self, filename: &PathBuf) -> Result<T, FileError>;
}

impl<T> Contextify<T> for Result<T, io::Error> {
    fn imbue_err(self, filename: &PathBuf) -> Result<T, FileError> {
        match self {
            Err(err) => Err(FileError {err: err, filename: filename.clone()}),
            Ok(v) => Ok(v),
        }
    }
}

fn fsum(args: &mut Iterator<Item=PathBuf>) -> u64
{
    let mut todo: VecDeque<PathBuf> = args.collect();

    let mut seen: HashSet<(u64, u64)> = HashSet::new();
    let mut total = 0u64;

    fn log_error(e: FileError) {
        writeln!(&mut std::io::stderr(), "{}", e).unwrap()
    }

    while let Some(fl) = todo.pop_front() {
        (|| {
            let mut meta = try!(fs::symlink_metadata(&fl).imbue_err(&fl));
            if meta.file_type().is_symlink() {
                let follow = fs::metadata(&fl);
                if !follow.is_ok() {
                    return Ok(());  // don't log broken symlinks
                }
                meta = try!(follow.imbue_err(&fl));
            }
            let st = &meta as &std::os::unix::fs::MetadataExt;
            let file_id = (st.dev(), st.ino());
            if !seen.insert(file_id) {
                return Ok(());
            }

            if meta.is_dir() {
                todo.extend(try!(fs::read_dir(&fl).imbue_err(&fl))
                            .filter_map(|res| res.imbue_err(&fl).map_err(log_error).ok())
                            .map(|dirent| dirent.path()));
            } else {
                total += meta.len();
            }
            Ok(())
        })().map_err(log_error).ok();
    }

    total
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
