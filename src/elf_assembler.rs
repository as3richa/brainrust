use std::io;

use crate::assembler::Instruction::*;
use crate::assembler::{Assembler, Instruction};
use crate::elf::*;

type Memory = u64;
type Label = usize;

pub struct ElfAssembler {
    memory_allocated: u64,
    labels_allocated: usize,
    code: Vec<Instruction<Memory, Label>>,
}

impl ElfAssembler {
    pub fn new() -> Self {
        Self {
            memory_allocated: 0,
            labels_allocated: 0,
            code: vec![],
        }
    }
}

impl<'a> Assembler<'a> for ElfAssembler {
    type Memory = Memory;
    type Label = Label;

    fn allocate_memory(&mut self, name: &str, size: u64) -> Self::Memory {
        let label_offset = self.memory_allocated;
        self.memory_allocated += size;
        label_offset
    }

    fn allocate_label(&mut self, name: &str) -> Self::Label {
        let index = self.labels_allocated;
        self.labels_allocated += 1;
        index
    }

    fn emit_code(&mut self, code: &[Instruction<Memory, Label>]) {
        self.code.extend(code.iter().cloned());
    }

    fn assemble<W: io::Write>(self, output: &mut W) -> Result<(), io::Error> {
        enum LabelState {
            Unpopulated(Vec<usize>),
            Populated(usize),
        }

        let mut label_states = Vec::with_capacity(self.labels_allocated);

        for index in 0..self.labels_allocated {
            label_states.push(LabelState::Unpopulated(vec![]));
        }

        let mut machine_code: Vec<u8> = vec![];

        for instruction in self.code {
            match instruction {
                Label(index) => {
                    let label_state = &mut label_states[index];

                    match label_state {
                        LabelState::Unpopulated(branch_offsets) => {
                            let label_offset = machine_code.len();

                            for branch_offset in branch_offsets {
                                let le_relative_offset = i32_to_le32(relative_offset(label_offset, *branch_offset + 6));
                                let patch_location = &mut machine_code[(*branch_offset + 2)..(*branch_offset + 6)];
                                assert!(patch_location == &[0x00, 0x00, 0x00, 0x00]);
                                patch_location.copy_from_slice(&le_relative_offset);
                            }

                            *label_state = LabelState::Populated(label_offset)
                        }
                        LabelState::Populated(_) => assert!(false),
                    }
                }

                JmpLeq(index) => {
                    let branch_offset = machine_code.len();
                    machine_code.extend(&[0x0f, 0x8e]);

                    match &mut label_states[index] {
                        LabelState::Unpopulated(ref mut branch_offsets) => {
                            branch_offsets.push(branch_offset);
                            machine_code.extend(&[0x00, 0x00, 0x00, 0x00]);
                        }
                        LabelState::Populated(label_offset) => {
                            let le_relative_offset = i32_to_le32(relative_offset(*label_offset, branch_offset + 6));
                            machine_code.extend(&le_relative_offset);
                        }
                    }
                }

                _ => machine_code.extend(&[0x90]),
            }
        }

        let le_machine_code_size = usize_to_le64(machine_code.len());
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
        output.write_all(&machine_code)?;
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

fn i32_to_le32(value: i32) -> [u8; 4] {
    [
        (value & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        ((value >> 16) & 0xff) as u8,
        ((value >> 24) & 0xff) as u8,
    ]
}

fn relative_offset(to: usize, from: usize) -> i32 {
    if (to <= from) {
        assert!(from - to <= (i32::max_value() as usize));
        -((from - to) as i32)
    } else {
        assert!(to - from <= (i32::max_value() as usize));
        (to - from) as i32
    }
}
