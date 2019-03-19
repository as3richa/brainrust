use std::io;

pub struct Stream<R: io::Read> {
    pub line: usize,
    pub column: usize,
    peeked: Option<u8>,
    bytes: io::Bytes<R>,
}

impl<R: io::Read> Stream<R> {
    pub fn new(read: R) -> Self {
        Self {
            line: 1,
            column: 1,
            peeked: None,
            bytes: read.bytes(),
        }
    }

    pub fn peek(&mut self) -> Result<Option<u8>, io::Error> {
        if self.peeked.is_none() {
            if let Some(result) = self.bytes.next() {
                let byte = result?;
                self.peeked = Some(byte);
            }
        }
        Ok(self.peeked)
    }

    pub fn forward(&mut self) {
        assert!(self.peeked.is_some());
        self.column += 1;
        self.peeked = None;
    }
}
