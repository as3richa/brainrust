#[derive(Debug)]
pub enum Tree {
    Move(i64),
    Add(i64),
    ReadChar,
    WriteChar,
    Loop(Vec<Tree>),
    EndOfFile,
}
