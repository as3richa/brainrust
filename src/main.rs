mod instruction;
mod lexer;

use std::io;

use instruction::Instruction;
use lexer::Lexer;

fn main() {
    let stdin = io::stdin();
    let mut lexer = Lexer::new(stdin.lock());

    loop {
        let (instruction, line, column) = lexer.lex().unwrap();
        println!("{}:{}: {:?}", line, column, instruction);
        if let Instruction::ProgramEnd = instruction {
            break;
        }
    }
}
