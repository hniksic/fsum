use std::collections::HashSet;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use std::fs;
use std::env;

fn fsum<'a, T>(args: T) -> u64
    where T: Iterator<Item=&'a Path>
{
    let mut todo: VecDeque<PathBuf> = args.map(|x| x.to_path_buf()).collect();

    let mut seen: HashSet<(u64, u64)> = HashSet::new();
    let mut total = 0u64;

    while let Some(fl) = todo.pop_front() {
        let meta = fs::symlink_metadata(&fl).unwrap();
        let st = &meta as &std::os::unix::fs::MetadataExt;
        let file_id = (st.dev(), st.ino());
        if !seen.insert(file_id) {
            continue
        }

        if meta.is_dir() {
            todo.extend(fs::read_dir(&fl)
                        .unwrap()
                        .map(|dirent| dirent.unwrap().path()));
        } else {
            total += meta.len();
        }
    }

    total
}

fn main()
{
    let paths: Vec<PathBuf> = env::args_os().skip(1).map(PathBuf::from).collect();
    let size = fsum(paths.iter().map(|x| x.as_path()));
    println!("{}", size);
    for &(power, digits, letter) in [(1<<10, 0, "K"), (1<<20, 2, "M"), (1<<30, 2, "G")].iter() {
        if size >= power {
            println!("{:.*} {}", digits, size as f64 / power as f64, letter)
        }
    }
}
