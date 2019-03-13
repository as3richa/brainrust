#[derive(Debug)]
pub enum Instruction {
    Move(isize),
    Add(isize),
    ReadChar,
    WriteChar,
    LoopStart,
    LoopEnd,
    ProgramEnd,
}
