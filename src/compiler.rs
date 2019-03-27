use std::io;

use crate::assembler::Assembler;
use crate::elf_assembler::ElfAssembler;
use crate::parser::Token::*;
use crate::parser::{parse, ParseError};
use crate::stream::Stream;

/*
    We allocate registers as follows:
    - rbx: Pointer to the base of the tape
    - r14: Pointer to the input buffer
    - rsp: Pointer to the output buffer
    - r8: Current tape position
    - r9: Tape length
    - r10: Current position within the input buffer
    - r12: Total number of bytes in the input buffer
    - r13: Current position within the output buffer
    - r15: Scratch space
*/

const TAPE_LENGTH: u64 = 30000;
const INPUT_BUFFER_SIZE: u64 = 16;
const OUTPUT_BUFFER_SIZE: u64 = 16;

pub fn compile<W: io::Write, R: io::Read>(output: &mut W, mut stream: Stream<R>) -> Result<(), ParseError> {
    let mut asm = ElfAssembler::new();

    let tape = asm.allocate_memory(TAPE_LENGTH);
    let input_buffer = asm.allocate_memory(INPUT_BUFFER_SIZE);
    let output_buffer = asm.allocate_memory(OUTPUT_BUFFER_SIZE);

    asm.mov_rbx_addr(tape);
    asm.mov_r14_addr(input_buffer);
    asm.mov_rsp_addr(output_buffer);
    asm.xor_r8_r8();
    asm.mov_r9_u64(TAPE_LENGTH);
    asm.xor_r10_r10();
    asm.xor_r12_r12();
    asm.xor_r13_r13();

    let mut loop_stack = vec![];

    loop {
        let (token, line, column) = parse(&mut stream)?;

        match token {
            Move(shift) => {
                // FIXME: almost definitely have a bug related to sign extension here

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
                        let value = ((-shift) as u64) % TAPE_LENGTH;
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

                match wrapped_value {
                    0 => (),
                    1 => asm.inc_byte_ptr_rbx_plus_r8(),
                    255 => asm.dec_byte_ptr_rbx_plus_r8(),
                    _ => asm.add_byte_ptr_rbx_plus_r8_u8(wrapped_value),
                }
            }
            ReadChar => {
                // FIXME: allow unbuffered input
                assert!(INPUT_BUFFER_SIZE > 0);

                let data_in_buffer = asm.allocate_label();

                asm.cmp_r10_r12();
                asm.jne(data_in_buffer);

                // Flush any buffered output
                {
                    let skip_flush = asm.allocate_label();
                    asm.cmp_r13_u32(0);
                    asm.je(skip_flush);
                    emit_flush(&mut asm);
                    asm.label(skip_flush);
                }

                // Read into the input buffer
                {
                    asm.xor_rax_rax(); // sys_read
                    asm.xor_rdi_rdi(); // Standard input
                    asm.mov_rsi_r14(); // Input buffer
                    asm.mov_rdx_u32(INPUT_BUFFER_SIZE as u32); // Input buffer size
                    asm.syscall();

                    // FIXME: distinguish errors from EOF
                    let okay = asm.allocate_label();
                    asm.cmp_rax_u32(0);
                    asm.jg(okay);
                    emit_exit(&mut asm, 2);
                    asm.label(okay);

                    // Record the number of bytes in the input buffer
                    asm.mov_r12_rax();

                    // Rest input buffer cursor to zero
                    asm.xor_r10_r10();
                }

                asm.label(data_in_buffer);

                // Copy a byte from the input buffer to the tape
                asm.mov_r15b_byte_ptr_r14_plus_r10();
                asm.mov_byte_ptr_rbx_plus_r8_r15b();

                // Increment input buffer index
                asm.inc_r10();
            }
            WriteChar => {
                // FIXME: allow unbuffered output
                assert!(OUTPUT_BUFFER_SIZE > 0);

                // Copy a byte from the tape to the output buffer
                asm.mov_r15b_byte_ptr_rbx_plus_r8();
                asm.mov_byte_ptr_rsp_plus_r13_r15b();

                // Increment output buffer index
                asm.inc_r13();

                let flush = asm.allocate_label();
                let done = asm.allocate_label();

                // Flush output buffer if character was a newline
                asm.cmp_r15b_u8(b'\n');
                asm.je(flush);

                // Skip flush if the character was not a newline and the buffer isn't full
                asm.cmp_r13_u32(OUTPUT_BUFFER_SIZE as u32);
                asm.jne(done);

                asm.label(flush);

                emit_flush(&mut asm);

                // Flush is complete, or no flush was necessary
                asm.label(done);
            }
            LoopStart => {
                let start_label = asm.allocate_label();
                let end_label = asm.allocate_label();
                loop_stack.push((start_label, end_label));
                asm.cmp_byte_ptr_rbx_plus_r8_u8(0);
                asm.je(end_label);
                asm.label(start_label);
            }
            LoopEnd => {
                // FIXME: error handling
                let (start_label, end_label) = loop_stack.pop().unwrap();
                asm.cmp_byte_ptr_rbx_plus_r8_u8(0);
                asm.jne(start_label);
                asm.label(end_label);
            }
            EndOfFile => break,
        }
    }

    // FIXME: error handling
    assert!(loop_stack.len() == 0);

    // Flush any remaining output
    {
        let skip_flush = asm.allocate_label();
        asm.cmp_r13_u32(0);
        asm.je(skip_flush);
        emit_flush(&mut asm);
        asm.label(skip_flush);
    }

    emit_exit(&mut asm, 0);

    asm.assemble(output)?;

    Ok(())
}

fn emit_exit(asm: &mut ElfAssembler, code: u32) {
    // sys_exit
    asm.mov_rax_u32(0x3c);

    // Exit code
    if code == 0 {
        asm.xor_rdi_rdi();
    } else {
        asm.mov_rdi_u32(code);
    }

    asm.syscall();
}

fn emit_flush(asm: &mut ElfAssembler) {
    // Let r15 represent the number of bytes written thus far
    asm.xor_r15_r15();

    // Start of flush loop
    let loop_start = asm.allocate_label();
    asm.label(loop_start);

    // sys_write
    asm.mov_rax_u32(0x01);

    // fd 1, i.e. stdout
    asm.mov_rdi_u32(0x01);

    // Output buffer, excluding the already-written bytes
    asm.mov_rsi_rsp();
    asm.add_rsi_r15();

    // Number of bytes remaining
    asm.mov_rdx_r13();
    asm.sub_rdx_r15();

    asm.syscall();

    let okay = asm.allocate_label();

    // Check for errors (rax <= 0, signed)
    asm.cmp_rax_u32(0);
    asm.jg(okay);
    emit_exit(asm, 1);
    asm.label(okay);

    // Count the number of bytes written; if there remain bytes to be written, jump
    // to the top of the loop
    asm.add_r15_rax();
    asm.cmp_r15_r13();
    asm.jne(loop_start);

    // Mark the buffer as empty
    asm.xor_r13_r13();
}
