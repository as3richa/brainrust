mod parser;
mod stream;
mod tree;

use std::io;

use crate::parser::parse;
use crate::stream::Stream;

fn main() {
    let stdin = io::stdin();
    let mut stream = Stream::new(stdin.lock());
    println!("{:?}", parse(&mut stream));
}
