use std::io;

pub trait Assembler<'a> {
    type Memory: 'a + Copy;
    type Label: 'a + Copy;

    fn allocate_memory(&mut self, size: u64) -> Self::Memory;
    fn allocate_label(&mut self) -> Self::Label;

    fn add_byte_ptr_rbx_plus_r8_u8(&mut self, operand: u8);
    fn add_r15_rax(&mut self);
    fn add_r8_r9(&mut self);
    fn add_r8_u32(&mut self, operand: u32);
    fn add_rsi_r15(&mut self);
    fn cmovge_r8_r15(&mut self);
    fn cmp_byte_ptr_rbx_plus_r8_u8(&mut self, operand: u8);
    fn cmp_r13_r14(&mut self);
    fn cmp_r15_r13(&mut self);
    fn cmp_r15b_u8(&mut self, operand: u8);
    fn cmp_rax_u32(&mut self, operand: u32);
    fn dec_byte_ptr_rbx_plus_r8(&mut self);
    fn inc_byte_ptr_rbx_plus_r8(&mut self);
    fn inc_r13(&mut self);
    fn je(&mut self, label: Self::Label);
    fn jg(&mut self, label: Self::Label);
    fn jge(&mut self, label: Self::Label);
    fn jmp(&mut self, label: Self::Label);
    fn jnc(&mut self, label: Self::Label);
    fn jne(&mut self, label: Self::Label);
    fn mov_byte_ptr_rsp_plus_r13_r15b(&mut self);
    fn mov_r12_u64(&mut self, operand: u64);
    fn mov_r14_u64(&mut self, operand: u64);
    fn mov_r15_r8(&mut self);
    fn mov_r15b_byte_ptr_rbx_plus_r8(&mut self);
    fn mov_r9_u64(&mut self, operand: u64);
    fn mov_rax_u32(&mut self, operand: u32);
    fn mov_rbx_addr(&mut self, address: Self::Memory);
    fn mov_rcx_addr(&mut self, address: Self::Memory);
    fn mov_rdi_u32(&mut self, operand: u32);
    fn mov_rdx_r13(&mut self);
    fn mov_rsi_rsp(&mut self);
    fn mov_rsp_addr(&mut self, address: Self::Memory);
    fn sub_r15_r9(&mut self);
    fn sub_r8_r9(&mut self);
    fn sub_r8_u32(&mut self, operand: u32);
    fn sub_rdx_r15(&mut self);
    fn syscall(&mut self);
    fn xor_r10_r10(&mut self);
    fn xor_r11_r11(&mut self);
    fn xor_r13_r13(&mut self);
    fn xor_r15_r15(&mut self);
    fn xor_r8_r8(&mut self);
    fn xor_rax_rax(&mut self);
    fn xor_rdi_rdi(&mut self);

    fn label(&mut self, label: Self::Label);

    fn assemble<W: io::Write>(self, output: &mut W) -> Result<(), io::Error>;
}
