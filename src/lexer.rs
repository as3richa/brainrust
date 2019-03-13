use std::io;

use crate::instruction::Instruction;
use crate::instruction::Instruction::*;

pub struct Lexer<R: io::Read> {
    pub line: usize,
    pub column: usize,
    peeked: Option<u8>,
    bytes: io::Bytes<R>,
}

impl<R: io::Read> Lexer<R> {
    pub fn new(stream: R) -> Self {
        let bytes = stream.bytes();
        Self {
            line: 1,
            column: 1,
            peeked: None,
            bytes,
        }
    }

    pub fn lex(&mut self) -> Result<(Instruction, usize, usize), io::Error> {
        let mut instruction = loop {
            match self.peek()? {
                Some(byte) => match byte {
                    b'>' => break Move(1),
                    b'<' => break Move(-1),
                    b'+' => break Add(1),
                    b'-' => break Add(-1),
                    b'.' => break WriteChar,
                    b',' => break ReadChar,
                    b'[' => break LoopStart,
                    b']' => break LoopEnd,
                    _ => self.forward(),
                },
                None => return Ok((ProgramEnd, self.line, self.column)),
            }
        };

        let line = self.line;
        let column = self.column;
        self.forward();

        match instruction {
            Move(ref mut delta) => {
                while let Some(byte) = self.peek()? {
                    match byte {
                        b'>' => *delta += 1,
                        b'<' => *delta -= 1,
                        b'+' | b'-' | b'.' | b',' | b'[' | b']' => break,
                        _ => (),
                    }
                    self.forward();
                }
            }
            Add(ref mut increment) => {
                while let Some(byte) = self.peek()? {
                    match byte {
                        b'+' => *increment += 1,
                        b'-' => *increment -= 1,
                        b'>' | b'<' | b'.' | b',' | b'[' | b']' => break,
                        _ => (),
                    }
                    self.forward();
                }
            }
            _ => (),
        }

        Ok((instruction, line, column))
    }

    fn peek(&mut self) -> Result<Option<u8>, io::Error> {
        if self.peeked.is_none() {
            self.peeked = match self.bytes.next() {
                Some(Err(error)) => return Err(error),
                Some(Ok(byte)) => Some(byte),
                None => None,
            };
        }

        Ok(self.peeked)
    }

    fn forward(&mut self) {
        match self.peeked.take().unwrap() {
            b'\n' => {
                self.line += 1;
                self.column = 1;
            }
            _ => {
                self.column += 1;
            }
        }
    }
}
