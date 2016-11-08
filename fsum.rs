use std::collections::{HashSet, VecDeque};
use std::path::{PathBuf};
use std::fs;
use std::env;
use std::io::Write;

fn fsum(args: &mut Iterator<Item=PathBuf>) -> u64
{
    let mut todo: VecDeque<PathBuf> = args.collect();

    let mut seen: HashSet<(u64, u64)> = HashSet::new();
    let mut total = 0u64;

    while let Some(fl) = todo.pop_front() {
        let log_error = |e| writeln!(std::io::stderr(), "{}: {}",
                                     fl.display(), e).unwrap();
        (|| {
            let mut meta = try!(fs::symlink_metadata(&fl));
            if meta.file_type().is_symlink() {
                let follow = fs::metadata(&fl);
                if !follow.is_ok() {
                    return Ok(());  // don't log broken symlinks
                }
                meta = try!(follow);
            }
            let st = &meta as &std::os::unix::fs::MetadataExt;
            let file_id = (st.dev(), st.ino());
            if !seen.insert(file_id) {
                return Ok(());
            }

            if meta.is_dir() {
                todo.extend(try!(fs::read_dir(&fl))
                            .filter_map(|res| res.map_err(&log_error).ok())
                            .map(|dirent| dirent.path()));
            } else {
                total += meta.len();
            }
            Ok(())
        })().map_err(&log_error).ok();
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
