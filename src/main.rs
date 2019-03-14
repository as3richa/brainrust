mod parser;
mod stream;
mod tree;

use std::io;

use crate::parser::Parser;

fn main() {
    let stdin = io::stdin();
    let mut parser = Parser::new(stdin.lock());
    println!("{:?}", parser.parse());
}
