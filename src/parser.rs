use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::io;

use crate::stream::Stream;
use crate::tree::Tree;
use crate::tree::Tree::*;

type ParseResult = Result<Tree, ParseError>;

pub fn parse<R: io::Read>(stream: &mut Stream<R>) -> ParseResult {
    let tree = match stream.peek()? {
        Some(byte) => {
            let (line, column) = (stream.line, stream.column);
            stream.forward();

            match byte {
                b'>' | b'<' => parse_move(stream, byte)?,
                b'+' | b'-' => parse_increment(stream, byte)?,
                b'[' => parse_loop(stream, line, column)?,

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

    Ok(tree)
}

fn parse_move<R: io::Read>(stream: &mut Stream<R>, byte: u8) -> ParseResult {
    let mut shift = {
        if byte == b'>' {
            1
        } else {
            -1
        }
    };

    while let Some(byte) = stream.peek()? {
        match byte {
            b'>' => shift += 1,
            b'<' => shift -= 1,
            _ => break,
        }
        stream.forward();
    }

    Ok(Move(shift))
}

fn parse_increment<R: io::Read>(stream: &mut Stream<R>, byte: u8) -> ParseResult {
    let mut value = {
        if byte == b'+' {
            1
        } else {
            -1
        }
    };

    while let Some(byte) = stream.peek()? {
        match byte {
            b'+' => value += 1,
            b'-' => value -= 1,
            _ => break,
        }
        stream.forward();
    }

    Ok(Add(value))
}

fn parse_loop<R: io::Read>(stream: &mut Stream<R>, line: usize, column: usize) -> ParseResult {
    let mut children = vec![];

    loop {
        match stream.peek()? {
            Some(b']') => {
                stream.forward();
                break;
            }
            _ => match parse(stream)? {
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
        Self { line, column, message }
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
