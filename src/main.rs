extern crate rayon;

use std::env;
use std::path::PathBuf;

mod fsum;
use fsum::fsum;

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
