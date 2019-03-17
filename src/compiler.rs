use std::io;

use crate::assembler::Assembler;
use crate::elf_assembler::ElfAssembler;
use crate::parser::{parse, ParseError};
use crate::stream::Stream;
use crate::tree::Tree::*;

const tape_length: u32 = 256;

pub fn compile<W: io::Write, R: io::Read>(output: &mut W, mut stream: Stream<R>) -> Result<(), ParseError> {
    let mut asm = ElfAssembler::new();

    loop {
        let tree = parse(&mut stream)?;

        match tree {
            Move(shift) => {
                let normalized_shift = if shift < 0 {
                    (tape_length - (((-shift) as u32) % tape_length)) % tape_length
                } else {
                    (shift as u32) % tape_length
                };

                assert!(0 <= normalized_shift && normalized_shift < tape_length);

                if normalized_shift != 0 {
                    if normalized_shift == 1 {
                        asm.inc_ebx();
                    } else {
                        asm.add_ebx_u32(normalized_shift);
                    }
                    // FIXME: only works for power of two tape size
                    asm.and_ebx_u32(tape_length - 1);
                }
            }
            Add(_value) => asm.xor_eax_eax(),
            ReadChar => asm.xor_eax_eax(),
            WriteChar => asm.xor_eax_eax(),
            Loop(children) => asm.xor_eax_eax(),
            EndOfFile => break,
        }
    }

    asm.assemble(output)?;
    Ok(())
}
