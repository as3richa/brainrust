use std::io;

use crate::assembler::Assembler;
use crate::elf::*;

type Memory = u32;
type Label = usize;

pub struct ElfAssembler {
    memory_allocated: u32,
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
}

impl<'a> Assembler<'a> for ElfAssembler {
    type Memory = Memory;
    type Label = Label;

    fn allocate_memory(&mut self, name: &str, size: u32) -> Self::Memory {
        let offset = self.memory_allocated;
        self.memory_allocated += size;
        offset
    }

    fn allocate_label(&mut self, name: &str) -> Self::Label {
        let index = self.label_states.len();
        self.label_states.push(LabelState::Unpopulated(vec![]));
        index
    }

    fn add_ebx_u32(&mut self, addend: u32) {
        self.machine_code.extend(&[0x81, 0xc3]);
        self.machine_code.extend(&u32_to_le(addend));
    }

    fn and_ebx_u32(&mut self, mask: u32) {
        self.machine_code.extend(&[0x81, 0xe1]);
        self.machine_code.extend(&u32_to_le(mask));
    }

    fn inc_ebx(&mut self) {
        self.machine_code.extend(&[0xff, 0xc3]);
    }

    fn jle(&mut self, label: Self::Label) {
        let state = &mut self.label_states[label];

        let branch_offset = self.machine_code.len();
        self.machine_code.extend(&[0x0f, 0x8e]);

        match state {
            LabelState::Unpopulated(ref mut branch_offsets) => {
                branch_offsets.push(branch_offset);
                self.machine_code.extend(&[0x00, 0x00, 0x00, 0x00]);
            }
            LabelState::Populated(label_offset) => {
                let le_relative_offset = i32_to_le(relative_offset(*label_offset, branch_offset + 6));
                self.machine_code.extend(&le_relative_offset);
            }
        }
    }

    fn label(&mut self, label: Self::Label) {
        let state = &mut self.label_states[label];
        let label_offset = self.machine_code.len();

        let branch_offsets = match state {
            LabelState::Unpopulated(ref branch_offsets) => branch_offsets,
            LabelState::Populated(_) => unreachable!(),
        };

        for branch_offset in branch_offsets {
            let le_relative_offset = i32_to_le(relative_offset(label_offset, *branch_offset + 6));
            let patch_location = &mut self.machine_code[(*branch_offset + 2)..(*branch_offset + 6)];
            assert!(patch_location == &[0x00, 0x00, 0x00, 0x00]);
            patch_location.copy_from_slice(&le_relative_offset);
        }

        self.label_states[label] = LabelState::Populated(label_offset);
    }

    fn mov_eax_u32(&mut self, value: u32) {
        self.machine_code.extend(&[0xb8]);
        self.machine_code.extend(&u32_to_le(value));
    }

    fn syscall(&mut self) {
        self.machine_code.extend(&[0x0f, 0x05]);
    }

    fn xor_eax_eax(&mut self) {
        self.machine_code.extend(&[0x31, 0xc0]);
    }

    fn xor_edi_edi(&mut self) {
        self.machine_code.extend(&[0x31, 0xff]);
    }

    fn assemble<W: io::Write>(self, output: &mut W) -> Result<(), io::Error> {
        let le_machine_code_size = usize_to_le64(self.machine_code.len());
        output.write_all(&ELF_HEADER)?;
        output.write_all(&TEXT_PROGRAM_HEADER_START)?;
        output.write_all(&le_machine_code_size)?;
        output.write_all(&le_machine_code_size)?;
        output.write_all(&TEXT_PROGRAM_HEADER_END)?;
        output.write_all(&BSS_PROGRAM_HEADER)?;
        output.write_all(&DUMMY_SECTION_HEADER)?;
        output.write_all(&TEXT_SECTION_HEADER_START)?;
        output.write_all(&le_machine_code_size)?;
        output.write_all(&TEXT_SECTION_HEADER_END)?;
        output.write_all(&BSS_SECTION_HEADER)?;
        output.write_all(&STRING_TABLE_SECTION_HEADER)?;
        output.write_all(&STRING_TABLE_CONTENTS)?;
        output.write_all(&self.machine_code)?;
        Ok(())
    }
}

fn usize_to_le64(value: usize) -> [u8; 8] {
    [
        (value & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        ((value >> 16) & 0xff) as u8,
        ((value >> 24) & 0xff) as u8,
        ((value >> 32) & 0xff) as u8,
        ((value >> 40) & 0xff) as u8,
        ((value >> 48) & 0xff) as u8,
        ((value >> 56) & 0xff) as u8,
    ]
}

fn u32_to_le(value: u32) -> [u8; 4] {
    [
        (value & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        ((value >> 16) & 0xff) as u8,
        ((value >> 24) & 0xff) as u8,
    ]
}

fn i32_to_le(value: i32) -> [u8; 4] {
    u32_to_le(value as u32)
}

fn relative_offset(to: usize, from: usize) -> i32 {
    if to <= from {
        assert!(from - to <= (i32::max_value() as usize));
        -((from - to) as i32)
    } else {
        assert!(to - from <= (i32::max_value() as usize));
        (to - from) as i32
    }
}
