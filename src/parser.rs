use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::io;

use crate::stream::Stream;
use crate::tree::Tree;
use crate::tree::Tree::*;

pub struct Parser<R: io::Read> {
    stream: Stream<R>,
}

impl<R: io::Read> Parser<R> {
    pub fn new(read: R) -> Self {
        let stream = Stream::new(read);
        Self { stream }
    }

    pub fn parse(&mut self) -> Result<Tree, ParseError> {
        let node = match self.stream.peek()? {
            Some(byte) => {
                let line = self.stream.line;
                let column = self.stream.column;
                self.stream.forward();

                match byte {
                    b'>' | b'<' => self.parse_move(byte)?,
                    b'+' | b'-' => self.parse_increment(byte)?,
                    b'[' => self.parse_loop(line, column)?,

                    b'.' => WriteChar,
                    b',' => ReadChar,

                    b']' => {
                        let error = SyntaxError::new(line, column, "unmatched closing bracket `]`");
                        return Err(ParseError::from(error));
                    }

                    _ => unreachable!(),
                }
            }
            None => EndOfFile,
        };

        Ok(node)
    }

    fn parse_move(&mut self, byte: u8) -> Result<Tree, io::Error> {
        let mut shift = {
            if byte == b'>' {
                1
            } else {
                -1
            }
        };

        while let Some(byte) = self.stream.peek()? {
            match byte {
                b'>' => shift += 1,
                b'<' => shift -= 1,
                _ => break,
            }
            self.stream.forward();
        }

        Ok(Move(shift))
    }

    fn parse_increment(&mut self, byte: u8) -> Result<Tree, io::Error> {
        let mut value = {
            if byte == b'+' {
                1
            } else {
                -1
            }
        };

        while let Some(byte) = self.stream.peek()? {
            match byte {
                b'+' => value += 1,
                b'-' => value -= 1,
                _ => break,
            }
            self.stream.forward();
        }

        Ok(Add(value))
    }

    fn parse_loop(&mut self, line: usize, column: usize) -> Result<Tree, ParseError> {
        let mut children = vec![];

        loop {
            match self.stream.peek()? {
                Some(b']') => break,
                _ => match self.parse()? {
                    EndOfFile => {
                        let error = SyntaxError::new(line, column, "unmatched opening bracket `[`");
                        return Err(ParseError::from(error));
                    }
                    child => children.push(child),
                },
            }
        }

        Ok(Loop(children))
    }
}

pub enum ParseError {
    Io(io::Error),
    Syntax(SyntaxError),
}

impl Display for ParseError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            ParseError::Io(error) => Display::fmt(error, formatter),
            ParseError::Syntax(error) => Display::fmt(error, formatter),
        }
    }
}

impl Debug for ParseError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            ParseError::Io(error) => Debug::fmt(error, formatter),
            ParseError::Syntax(error) => Debug::fmt(error, formatter),
        }
    }
}

impl From<io::Error> for ParseError {
    fn from(error: io::Error) -> Self {
        ParseError::Io(error)
    }
}

impl From<SyntaxError> for ParseError {
    fn from(error: SyntaxError) -> Self {
        ParseError::Syntax(error)
    }
}

pub struct SyntaxError {
    line: usize,
    column: usize,
    message: &'static str,
}

impl SyntaxError {
    fn new(line: usize, column: usize, message: &'static str) -> Self {
        Self {
            line,
            column,
            message,
        }
    }
}

impl Display for SyntaxError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl Debug for SyntaxError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        Display::fmt(self, formatter)
    }
}
