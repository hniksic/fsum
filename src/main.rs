use std::env;
use std::iter;

mod fsum;
use fsum::fsum;

fn format_large(n: u128) -> impl Iterator<Item = String> {
    iter::once(format!("{}", n)).chain(
        [
            (1 << 10, 0, "KiB"),
            (1 << 20, 2, "MiB"),
            (1 << 30, 2, "GiB"),
            (1 << 40, 2, "TiB"),
            (1 << 50, 2, "PiB"),
            (1 << 60, 2, "EiB"),
            (1 << 70, 2, "ZiB"),
            (1 << 80, 2, "YiB"),
        ]
        .iter()
        .filter_map(move |&(power, digits, letter)| {
            if n >= power {
                Some(format!("{:.*} {}", digits, n as f64 / power as f64, letter))
            } else {
                None
            }
        }),
    )
}

fn main() {
    let size = fsum(env::args_os().skip(1));
    for formatted in format_large(size) {
        println!("{}", formatted);
    }
}
