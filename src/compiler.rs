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
                // FIXME: clean up casting mess
                let wrapped_shift = shift % (TAPE_LENGTH as i64);
                assert!(-(TAPE_LENGTH as i64) < wrapped_shift && wrapped_shift < (TAPE_LENGTH as i64));

                // If the shift would bring us back to the same cell, it's a no-op
                if wrapped_shift == 0 {
                    continue;
                }

                // Implement the shift as a sign-extended addition to r8 with an 8- or 32-bit immediate;
                // we can't use inc/dec here because the wraparound logic depends on the flags being updated
                if i64::from(i8::min_value()) <= wrapped_shift && wrapped_shift <= i64::from(i8::max_value()) {
                    asm.add_r8_i8(wrapped_shift as i8);
                } else if i64::from(i32::min_value()) <= wrapped_shift && wrapped_shift <= i64::from(i32::max_value()) {
                    asm.add_r8_i32(wrapped_shift as i32);
                } else {
                    panic!("shift too big (FIXME)")
                }

                if wrapped_shift > 0 {
                    // Assume that the addition didn't overflow r8 (this would only be possible for TAPE_LENGTH >= 2**63)
                    // FIXME: surface this assertion in a saner way
                    assert!(TAPE_LENGTH < (1u64 << 63));

                    // Given the previous assumption, we know that the shift exceeded the right
                    // boundary of the tape if and only if r8 is greater than or equal to r9
                    // (unsigned). In this case we can recover the correctly-wrapped value of the
                    // tape pointer by simply subtracting r9 from r8

                    // Using r15 as scratch, compute r8 - r9, and copy the result back to r8 if
                    // in fact r8 >= r9 (unsigned)
                    asm.mov_r15_r8();
                    asm.sub_r15_r9();
                    asm.cmovae_r8_r15();
                } else {
                    // Assume that TAPE_LENGTH isn't huge, again (FIXME)
                    assert!(TAPE_LENGTH < (1u64 << 63));

                    // Given the previous assertion, we exceeded the left boundary of the tape if
                    // and only if the previous addition resulted in a negative integer. Moreover,
                    // in this case we can recover the correctly-wrapped value of the tape pointer
                    // by simply adding r9 to r8 (because r8 contains a signed negative integer
                    // indicating the magnitude of the underflow)

                    let done = asm.allocate_label();
                    asm.jns(done);
                    asm.add_r8_r9();
                    asm.label(done);
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
    assert!(loop_stack.is_empty());

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
