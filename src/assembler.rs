use std::io;

pub trait Assembler<'a> {
    type Memory: 'a + Copy;
    type Label: 'a + Copy;

    fn allocate_memory(&mut self, name: &str, size: u64) -> Self::Memory;
    fn allocate_label(&mut self, name: &str) -> Self::Label;
    fn emit_code(&mut self, code: &[Instruction<Self::Memory, Self::Label>]);
    fn assemble<W: io::Write>(self, output: &mut W) -> Result<(), io::Error>;
}

#[derive(Clone, Copy)]
pub enum Instruction<M: Copy, L: Copy> {
    Syscall,
    IncRbx,
    AddRbxImmediate(u64),
    AndRbxImmediate(u64),
    MovRaxImmediate(u64),
    XorRaxRax,
    XorRbxRbx,
    JmpLeq(L),
    Label(L),
    Nop(M),
}
