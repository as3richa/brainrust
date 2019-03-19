use std::io;

pub trait Assembler<'a> {
    type Memory: 'a + Copy;
    type Label: 'a + Copy;

    fn allocate_memory(&mut self, size: u64) -> Self::Memory;
    fn allocate_label(&mut self) -> Self::Label;

    fn add_byte_ptr_r8_plus_r9_u8(&mut self, operand: u8);
    fn add_r8_r9(&mut self);
    fn add_r8_u32(&mut self, operand: u32);
    fn cmovge_r8_r15(&mut self);
    fn je(&mut self, label: Self::Label);
    fn jg(&mut self, label: Self::Label);
    fn jmp(&mut self, label: Self::Label);
    fn jnc(&mut self, label: Self::Label);
    fn jne(&mut self, label: Self::Label);
    fn mov_r15_r8(&mut self);
    fn mov_rax_u32(&mut self, operand: u32);
    fn sub_r15_r9(&mut self);
    fn sub_r8_r9(&mut self);
    fn sub_r8_u32(&mut self, operand: u32);
    fn syscall(&mut self);
    fn xor_rax_rax(&mut self);
    fn xor_rdi_rdi(&mut self);

    fn label(&mut self, label: Self::Label);

    fn assemble<W: io::Write>(self, output: &mut W) -> Result<(), io::Error>;
}
