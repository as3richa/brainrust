mod assembler;
mod compiler;
mod elf;
mod elf_assembler;
mod parser;
mod stream;

use std::fs::File;
use std::io;

use crate::compiler::compile;
use crate::stream::Stream;

fn main() {
    let stdin = io::stdin();
    let stream = Stream::new(stdin.lock());
    let mut output = File::create("a.out").unwrap();
    compile(&mut output, stream).unwrap();
}
