mod native;
mod parser;
mod stream;
mod tree;

use std::fs::File;
use std::io;

use crate::stream::Stream;

fn main() {
    let stdin = io::stdin();
    let stream = Stream::new(stdin.lock());
    let output = File::create("a.out").unwrap();
    native::compile(output, stream).unwrap();
}
