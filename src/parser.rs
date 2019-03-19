use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::io;

use crate::stream::Stream;

pub enum Token {
    Move(i64),
    Add(i64),
    ReadChar,
    WriteChar,
    LoopStart,
    LoopEnd,
    EndOfFile,
}

use Token::*;

pub fn parse<R: io::Read>(stream: &mut Stream<R>) -> Result<(Token, usize, usize), ParseError> {
    while let Some(byte) = stream.peek()? {
        let (line, column) = (stream.line, stream.column);
        stream.forward();

        let token = match byte {
            b'>' | b'<' => parse_move(stream, byte)?,
            b'+' | b'-' => parse_increment(stream, byte)?,
            b'[' => LoopStart,
            b']' => LoopEnd,
            b'.' => WriteChar,
            b',' => ReadChar,
            _ => continue,
        };

        return Ok((token, line, column));
    }

    Ok((EndOfFile, stream.line, stream.column))
}

fn parse_move<R: io::Read>(stream: &mut Stream<R>, byte: u8) -> Result<Token, ParseError> {
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
            b'+' | b'-' | b'[' | b']' | b'.' | b',' => break,
            _ => (),
        }
        stream.forward();
    }

    Ok(Move(shift))
}

fn parse_increment<R: io::Read>(stream: &mut Stream<R>, byte: u8) -> Result<Token, ParseError> {
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
            b'>' | b'<' | b'[' | b']' | b'.' | b',' => break,
            _ => (),
        }
        stream.forward();
    }

    Ok(Add(value))
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
