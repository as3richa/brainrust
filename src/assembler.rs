use std::io;

pub trait Assembler<'a> {
    type Memory: 'a + Copy;
    type Label: 'a + Copy;

    fn allocate_memory(&mut self, name: &str, size: u32) -> Self::Memory;
    fn allocate_label(&mut self, name: &str) -> Self::Label;

    fn add_ebx_u32(&mut self, addend: u32);
    fn and_ebx_u32(&mut self, mask: u32);
    fn inc_ebx(&mut self);
    fn jle(&mut self, label: Self::Label);
    fn label(&mut self, label: Self::Label);
    fn mov_eax_u32(&mut self, value: u32);
    fn syscall(&mut self);
    fn xor_eax_eax(&mut self);
    fn xor_edi_edi(&mut self);

    fn assemble<W: io::Write>(self, output: &mut W) -> Result<(), io::Error>;
}
