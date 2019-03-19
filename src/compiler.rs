use std::io;

use crate::assembler::Assembler;
use crate::elf_assembler::ElfAssembler;
use crate::parser::Token::*;
use crate::parser::{parse, ParseError};
use crate::stream::Stream;

/*
    We allocate registers as follows:
    - rax, rdi, rsi, rdx: Reserved for syscall invocation
    - rbx: Pointer to the base of the tape
    - rcx: Pointer to the input buffer
    - rsp: Pointer to the output buffer
    - r8: Current tape position
    - r9: Tape length
    - r10: Current position within the input buffer
    - r11: Total number of bytes in the input buffer
    - r12: Input buffer size
    - r13: Current position within the output buffer
    - r14: Output buffer size
    - r15: Scratch space

    There are a lot of potential optimizations to be had by choosing smaller
    registers where applicable, but I decided it wasn't worth the effort to
    implement the necessary plumbing (in particular, writing a general-purpose
    assembler is hard). Constant values are stored in registers too,
    because x64 doesn't support 64-bit immediate values for most instructions
    (and even if that wasn't the case, it would be a waste of space).
*/

const TAPE_LENGTH: u64 = 256;

pub fn compile<W: io::Write, R: io::Read>(output: &mut W, mut stream: Stream<R>) -> Result<(), ParseError> {
    let mut asm = ElfAssembler::new();

    let tape = asm.allocate_memory(TAPE_LENGTH);

    let mut loop_stack = vec![];

    loop {
        let (token, line, column) = parse(&mut stream)?;

        match token {
            Move(shift) => {
                if shift >= 0 {
                    if shift > (u32::max_value() as i64) {
                        panic!("FIXME");
                    }

                    let right_shift = {
                        let value = (shift as u64) % TAPE_LENGTH;
                        assert!(value <= (u32::max_value() as u64));
                        value as u32
                    };

                    if right_shift != 0 {
                        asm.add_r8_u32(right_shift);

                        // If the previous addition overflowed r8, then the move went past the right
                        // boundary of the tape (because the tape length is strictly less than the
                        // maximum value of r8). Alternatively, if the addition did not overflow, then
                        // the move went past the right boundary of the tape if and only if r8 >= r9.
                        // The former case is only possible for tape lengths with a 1 in their MSB.
                        // In either case, we can recover the correctly-wrapped position by subtracting
                        // the tape length from r8

                        let position_wrapped = asm.allocate_label();

                        if (TAPE_LENGTH >> 63) == 1 {
                            let did_not_overflow = asm.allocate_label();

                            // If r8 did not overflow, we still to handle the second case
                            asm.jnc(did_not_overflow);

                            // If it did overflow, we can just correct it in place and skip the
                            // handling of the second case
                            asm.sub_r8_r9();
                            asm.jmp(position_wrapped);

                            asm.label(did_not_overflow);
                        }

                        // Copy the tape position to scratch and subtract the tape length; copy the result
                        // back only if the position was in fact greater than or equal to the length (i.e.
                        // if the right boundary was exceeded)
                        asm.mov_r15_r8();
                        asm.sub_r15_r9();
                        asm.cmovge_r8_r15();

                        asm.label(position_wrapped);
                    }
                } else {
                    if shift < -(u32::max_value() as i64) {
                        panic!("FIXME");
                    }

                    let left_shift = {
                        let value = ((-shift).abs() as u64) % TAPE_LENGTH;
                        assert!(value <= (u32::max_value() as u64));
                        value as u32
                    };

                    if left_shift != 0 {
                        asm.sub_r8_u32(left_shift);

                        // Suppose the move takes us past the left boundary of the tape. Then, after
                        // the previous subtraction, r8 contains a two's complement negative integer
                        // indicating the magnitude of the underflow. In this case, we can implement
                        // the correct wrapping semantics by simply adding the tape length (i.e. r9)
                        // to r8
                        let position_wrapped = asm.allocate_label();
                        asm.jg(position_wrapped);
                        asm.add_r8_r9();
                        asm.label(position_wrapped);
                    }
                }
            }
            Add(value) => {
                let wrapped_value = ((value % 256 + 256) % 256) as u8;
                asm.add_byte_ptr_r8_plus_r9_u8(wrapped_value);
            }
            ReadChar => asm.xor_rax_rax(),
            WriteChar => asm.xor_rax_rax(),
            LoopStart => {
                let start_label = asm.allocate_label();
                let end_label = asm.allocate_label();
                loop_stack.push((start_label, end_label));
                asm.je(end_label);
                asm.label(start_label);
            }
            LoopEnd => {
                // FIXME: error handling
                let (start_label, end_label) = loop_stack.pop().unwrap();
                asm.jne(start_label);
                asm.label(end_label);
            }
            EndOfFile => break,
        }
    }

    // FIXME: error handling
    assert!(loop_stack.len() == 0);

    // 0x3c => sys_exit; exit code is in edi
    asm.mov_rax_u32(0x3c);
    asm.xor_rdi_rdi();
    asm.syscall();

    asm.assemble(output)?;
    Ok(())
}
