use std::io;

use crate::assembler::Assembler;
use crate::elf::*;

type Address = u64;
type Label = usize;

pub struct ElfAssembler {
    allocation_pointer: u64,
    label_states: Vec<LabelState>,
    machine_code: Vec<u8>,
}

enum LabelState {
    Unpopulated(Vec<usize>),
    Populated(usize),
}

impl ElfAssembler {
    pub fn new() -> Self {
        Self {
            allocation_pointer: BSS_VIRTUAL_ADDRESS,
            label_states: vec![],
            machine_code: vec![],
        }
    }

    fn generate_branch(&mut self, label: Label, code: &[u8]) {
        let state = &mut self.label_states[label];

        self.machine_code.extend(code);

        match state {
            LabelState::Unpopulated(ref mut patch_offsets) => {
                patch_offsets.push(self.machine_code.len());
                self.machine_code.extend(&[0x00, 0x00, 0x00, 0x00]);
            }
            LabelState::Populated(destination) => {
                let origin = self.machine_code.len() + 4;

                assert!(*destination < origin);

                let relative_offset = {
                    let difference = origin - *destination;
                    assert!(difference <= (i32::max_value() as usize)); // FIXME?
                    -(difference as i32)
                };

                self.machine_code.extend(&relative_offset.to_le_bytes());
            }
        }
    }
}

macro_rules! instr {
    ($name:ident, $code:expr) => {
        fn $name(&mut self) {
            self.machine_code.extend(&$code);
        }
    };

    ($name:ident, $operand_type:ty, $code:expr) => {
        fn $name(&mut self, operand: $operand_type) {
            self.machine_code.extend(&$code);
            self.machine_code.extend(&operand.to_le_bytes());
        }
    };
}

macro_rules! instr_branch {
    ($name:ident, $code:expr) => {
        fn $name(&mut self, label: Self::Label) {
            self.generate_branch(label, &$code);
        }
    };
}

impl<'a> Assembler<'a> for ElfAssembler {
    type Address = Address;
    type Label = Label;

    fn allocate_memory(&mut self, size: u64) -> Self::Address {
        assert!(self.allocation_pointer + size <= MAX_VIRTUAL_ADDRESS + 1); // FIXME: overflow
        let address = self.allocation_pointer;
        self.allocation_pointer += size;
        address
    }

    fn allocate_label(&mut self) -> Self::Label {
        let index = self.label_states.len();
        self.label_states.push(LabelState::Unpopulated(vec![]));
        index
    }

    fn label(&mut self, label: Self::Label) {
        let state = &mut self.label_states[label];
        let destination = self.machine_code.len();

        let patch_offsets = match state {
            LabelState::Unpopulated(ref offsets) => offsets,
            LabelState::Populated(_) => panic!("label was defined multiple times"),
        };

        for patch_offset in patch_offsets {
            let origin = *patch_offset + 4;
            assert!(origin <= destination);

            let patch_slice = &mut self.machine_code[*patch_offset..*patch_offset + 4];
            assert!(patch_slice == [0x00, 0x00, 0x00, 0x00]);

            let relative_offset = {
                let difference = destination - origin;
                assert!(difference <= (i32::max_value() as usize)); // FIXME?
                difference as i32
            };

            patch_slice.copy_from_slice(&relative_offset.to_le_bytes());
        }

        self.label_states[label] = LabelState::Populated(destination);
    }

    fn assemble<W: io::Write>(self, output: &mut W) -> Result<(), io::Error> {
        assert!((self.machine_code.len() as u64) <= MAX_TEXT_SIZE); // FIXME

        let le_text_size = self.machine_code.len().to_le_bytes();
        let le_bss_size = (self.allocation_pointer - BSS_VIRTUAL_ADDRESS).to_le_bytes();

        output.write_all(&ELF_HEADER)?;
        output.write_all(&TEXT_PROGRAM_HEADER_START)?;
        output.write_all(&le_text_size)?;
        output.write_all(&le_text_size)?;
        output.write_all(&TEXT_PROGRAM_HEADER_END)?;
        output.write_all(&BSS_PROGRAM_HEADER_START)?;
        output.write_all(&le_bss_size)?;
        output.write_all(&BSS_PROGRAM_HEADER_END)?;
        output.write_all(&DUMMY_SECTION_HEADER)?;
        output.write_all(&TEXT_SECTION_HEADER_START)?;
        output.write_all(&le_text_size)?;
        output.write_all(&TEXT_SECTION_HEADER_END)?;
        output.write_all(&BSS_SECTION_HEADER_START)?;
        output.write_all(&le_bss_size)?;
        output.write_all(&BSS_SECTION_HEADER_END)?;
        output.write_all(&STRING_TABLE_SECTION_HEADER)?;
        output.write_all(&STRING_TABLE_CONTENTS)?;
        output.write_all(&self.machine_code)?;
        Ok(())
    }

    instr!(add_byte_ptr_rbx_plus_r8_u8, u8, [0x42, 0x80, 0x04, 0x03]);
    instr!(add_r15_rax, [0x49, 0x01, 0xc7]);
    instr!(add_r8_i32, i32, [0x49, 0x81, 0xc0]);
    instr!(add_r8_i8, i8, [0x49, 0x83, 0xc0]);
    instr!(add_r8_r9, [0x4d, 0x01, 0xc8]);
    instr!(add_rsi_r15, [0x4c, 0x01, 0xfe]);
    instr!(cmovae_r8_r15, [0x4d, 0x0f, 0x43, 0xc7]);
    instr!(cmp_byte_ptr_rbx_plus_r8_u8, u8, [0x42, 0x80, 0x3c, 0x03]);
    instr!(cmp_r10_r11, [0x4d, 0x39, 0xda]);
    instr!(cmp_r10_r12, [0x4d, 0x39, 0xe2]);
    instr!(cmp_r13_u32, u32, [0x49, 0x81, 0xfd]);
    instr!(cmp_r13_rbp, [0x49, 0x39, 0xed]);
    instr!(cmp_r15b_u8, u8, [0x41, 0x80, 0xff]);
    instr!(cmp_r15_r13, [0x4d, 0x39, 0xef]);
    instr!(cmp_rax_u32, u32, [0x48, 0x3d]);
    instr!(dec_byte_ptr_rbx_plus_r8, [0x42, 0xfe, 0x0c, 0x03]);
    instr!(inc_byte_ptr_rbx_plus_r8, [0x42, 0xfe, 0x04, 0x03]);
    instr!(inc_r10, [0x49, 0xff, 0xc2]);
    instr!(inc_r13, [0x49, 0xff, 0xc5]);
    instr_branch!(je, [0x0f, 0x84]);
    instr_branch!(jg, [0x0f, 0x8f]);
    instr_branch!(jge, [0x0f, 0x8d]);
    instr_branch!(jmp, [0xe9]);
    instr_branch!(jne, [0x0f, 0x85]);
    instr_branch!(jns, [0x0f, 0x89]);
    instr!(mov_byte_ptr_rbx_plus_r8_r15b, [0x46, 0x88, 0x3c, 0x03]);
    instr!(mov_byte_ptr_rsp_plus_r13_r15b, [0x46, 0x88, 0x3c, 0x2c]);
    instr!(mov_r11_rax, [0x49, 0x89, 0xc3]);
    instr!(mov_r12_u64, u64, [0x49, 0xbc]);
    instr!(mov_r12_rax, [0x49, 0x89, 0xc4]);
    instr!(mov_r13_u32, u32, [0x41, 0xbd]);
    instr!(mov_r14_addr, Self::Address, [0x49, 0xbe]);
    instr!(mov_r15b_byte_ptr_r14_plus_r10, [0x47, 0x8a, 0x3c, 0x16]);
    instr!(mov_r15b_byte_ptr_rbx_plus_r8, [0x46, 0x8a, 0x3c, 0x03]);
    instr!(mov_r15_r8, [0x4d, 0x89, 0xc7]);
    instr!(mov_r9_u64, u64, [0x49, 0xb9]);
    instr!(mov_rax_u32, u32, [0xb8]);
    instr!(mov_rbp_u64, u64, [0x48, 0xbd]);
    instr!(mov_rbx_addr, Self::Address, [0x48, 0xbb]);
    instr!(mov_rdi_u32, u32, [0xbf]);
    instr!(mov_rdx_u32, u32, [0xba]);
    instr!(mov_rdx_r12, [0x4c, 0x89, 0xe2]);
    instr!(mov_rdx_r13, [0x4c, 0x89, 0xea]);
    instr!(mov_rsi_r14, [0x4c, 0x89, 0xf6]);
    instr!(mov_rsi_rsp, [0x48, 0x89, 0xe6]);
    instr!(mov_rsp_addr, Self::Address, [0x48, 0xbc]);
    instr!(sub_r15_r9, [0x4d, 0x29, 0xcf]);
    instr!(sub_r8_r9, [0x4d, 0x29, 0xc8]);
    instr!(sub_rdx_r15, [0x4c, 0x29, 0xfa]);
    instr!(syscall, [0x0f, 0x05]);
    instr!(xor_r10_r10, [0x4d, 0x31, 0xd2]);
    instr!(xor_r11_r11, [0x4d, 0x31, 0xdb]);
    instr!(xor_r12_r12, [0x4d, 0x31, 0xe4]);
    instr!(xor_r13_r13, [0x4d, 0x31, 0xed]);
    instr!(xor_r15_r15, [0x4d, 0x31, 0xff]);
    instr!(xor_r8_r8, [0x4d, 0x31, 0xc0]);
    instr!(xor_rax_rax, [0x48, 0x31, 0xc0]);
    instr!(xor_rdi_rdi, [0x48, 0x31, 0xff]);
}
