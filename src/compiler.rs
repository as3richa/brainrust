use std::io;

use crate::assembler::Assembler;
use crate::assembler::Instruction::*;
use crate::elf_assembler::ElfAssembler;
use crate::parser::{parse, ParseError};
use crate::stream::Stream;
use crate::tree::Tree::*;

const tape_length: u64 = 256;

pub fn compile<W: io::Write, R: io::Read>(output: &mut W, mut stream: Stream<R>) -> Result<(), ParseError> {
    let mut assembler = ElfAssembler::new();

    loop {
        let tree = parse(&mut stream)?;

        match tree {
            Move(shift) => {
                let normalized_shift = if shift < 0 {
                    (tape_length - (((-shift) as u64) % tape_length)) % tape_length
                } else {
                    (shift as u64) % tape_length
                };

                assert!(0 <= normalized_shift && normalized_shift < tape_length);

                if normalized_shift != 0 {
                    if normalized_shift == 1 {
                        assembler.emit_code(&[IncRbx])
                    } else {
                        assembler.emit_code(&[AddRbxImmediate(normalized_shift)]);
                    }
                    // FIXME: only works for power of two tape size
                    assembler.emit_code(&[AndRbxImmediate(tape_length - 1)]);
                }
            }
            Add(_value) => assembler.emit_code(&[XorRaxRax]),
            ReadChar => assembler.emit_code(&[XorRaxRax]),
            WriteChar => assembler.emit_code(&[XorRaxRax]),
            Loop(children) => {
                let label = assembler.allocate_label("asdasdsa");
                assembler.emit_code(&[JmpLeq(label), MovRaxImmediate(1), Label(label)]);
            }
            EndOfFile => break,
        }
    }

    assembler.assemble(output)?;
    Ok(())
}
