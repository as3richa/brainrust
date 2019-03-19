use std::io;

use crate::assembler::Assembler;
use crate::elf::*;

type Memory = u64;
type Label = usize;

pub struct ElfAssembler {
    memory_allocated: u64,
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
            memory_allocated: 0,
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
    }
}

macro_rules! instr_imm {
    ($name:ident, $operand_type:ident, $code:expr) => {
        fn $name(&mut self, operand: $operand_type) {
            self.machine_code.extend(&$code);
            self.machine_code.extend(&operand.to_le_bytes());
        }
    }
}

macro_rules! instr_branch {
    ($name:ident, $code:expr) => {
        fn $name(&mut self, label: Self::Label) {
            self.generate_branch(label, &$code);
        }
    }
}

macro_rules! instr_mov_addr {
    ($name:ident, $code:expr) => {
        fn $name(&mut self, address: Memory) {
            self.machine_code.extend(&$code);
            self.machine_code.extend(&(BSS_VIRTUAL_ADDRESS + address).to_le_bytes());
        }
    }
}

impl<'a> Assembler<'a> for ElfAssembler {
    type Memory = Memory;
    type Label = Label;

    fn allocate_memory(&mut self, size: u64) -> Self::Memory {
        assert!(size < (MAX_BSS_SIZE - self.memory_allocated)); // FIXME
        let offset = self.memory_allocated;
        self.memory_allocated += size;
        offset
    }

    fn allocate_label(&mut self) -> Self::Label {
        let index = self.label_states.len();
        self.label_states.push(LabelState::Unpopulated(vec![]));
        index
    }

    instr_mov_addr!(mov_rbx_addr, [0x48, 0xbb]);
    instr_mov_addr!(mov_rcx_addr, [0x48, 0xb9]);
    instr_mov_addr!(mov_rsp_addr, [0x48, 0xbc]);

    instr!(add_r8_r9, [0x41, 0x01, 0xc8]);
    instr!(cmovge_r8_r15, [0x4d, 0x0f, 0x4d, 0xc7]);
    instr!(dec_byte_ptr_rbx_plus_r8, [0x42, 0xfe, 0x0c, 0x03]);
    instr!(inc_byte_ptr_rbx_plus_r8, [0x42, 0xfe, 0x04, 0x03]);
    instr!(mov_r15_r8, [0x4d, 0x89, 0xc7]);
    instr!(sub_r15_r9, [0x4d, 0x29, 0xcf]);
    instr!(sub_r8_r9, [0x4d, 0x29, 0xc8]);
    instr!(syscall, [0x0f, 0x05]);
    instr!(xor_r10_r10, [0x4d, 0x31, 0xd2]);
    instr!(xor_r11_r11, [0x4d, 0x31, 0xdb]);
    instr!(xor_r13_r13, [0x4d, 0x31, 0xed]);
    instr!(xor_r8_r8, [0x4d, 0x31, 0xc0]);
    instr!(xor_rax_rax, [0x48, 0x31, 0xc0]);
    instr!(xor_rdi_rdi, [0x48, 0x31, 0xff]);

    instr_imm!(add_byte_ptr_rbx_plus_r8_u8, u8, [0x42, 0x80, 0x04, 0x03]);
    instr_imm!(add_r8_u32, u32, [0x49, 0x81, 0xc0]);
    instr_imm!(mov_r12_u64, u64, [0x49, 0xbc]);
    instr_imm!(mov_r14_u64, u64, [0x49, 0xbe]);
    instr_imm!(mov_r9_u64, u64, [0x49, 0xb9]);
    instr_imm!(mov_rax_u32, u32, [0xb8]);
    instr_imm!(sub_r8_u32, u32, [0x49, 0x81, 0xe8]);

    instr_branch!(je, [0x0f, 0x84]);
    instr_branch!(jg, [0x0f, 0x8f]);
    instr_branch!(jnc, [0x0f, 0x83]);
    instr_branch!(jne, [0x0f, 0x85]);
    instr_branch!(jmp, [0xe9]);

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
            assert!(patch_slice == &[0x00, 0x00, 0x00, 0x00]);

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
        let le_bss_size = self.memory_allocated.to_le_bytes();

        output.write_all(&ELF_HEADER)?;
        output.write_all(&TEXT_PROGRAM_HEADER_START)?;
        output.write_all(&le_text_size)?;
        output.write_all(&le_text_size)?;
        output.write_all(&TEXT_PROGRAM_HEADER_END)?;
        output.write_all(&BSS_PROGRAM_HEADER_START)?;
        output.write_all(&le_bss_size)?;
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
}
