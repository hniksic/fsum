# fsum

Utility that prints the total size of all files under the provided
directory.

Symbolic links are followed.  Recursive directory structure is
detected and ignored.  A file is counted only once, even if it is
pointed to by multiple symlinks or hard links.

This was my first attempt at a Rust program, a port of an older
[script](`http:fsum`).  Tested with my home directory that contains
~300k files, the Rust version is 7-8 times faster than the original
Python.  Part of the speedup is just Rust being more efficient, but a
larger part is due the Rust version using Rayon to distribute work
among threads.  Although one would expect the majority of time to be
spent waiting for disk, the directory metadata is in practice already
cached and served from RAM, so it's possible to get better performance
just by using more cores.  A single-threaded version can be obtained
for testing by changing `par_iter()` to an ordinary `iter()`.

## License

`fsum` is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).  See [LICENSE-APACHE](LICENSE-APACHE)
and [LICENSE-MIT](LICENSE-MIT) for details.  Contributing changes is
assumed to signal agreement with these licensing terms.
