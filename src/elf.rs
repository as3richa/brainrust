/*
    The ELF binary is layed out as follows:
    - ELF header (64 bytes)
    - .text program header (56 bytes)
    - .bss program header (56 bytes)
    - Dummy section header (64 bytes, offset 0xb0)
    - .text section header (64 bytes)
    - .bss section header (64 bytes)
    - String table section header (64 bytes)
    - String table contents (22 bytes, offset 0x01b0)
    - Code (variable length, offset 0x01c6)

    The user-space virtual address space spans from  0x0000000000000000
    to 0x00007fffffffffff.

    ELF dictates that "loadable process segments must have congruent values for
    p_vaddr and p_offset, modulo the page size", that is to say that the virtual
    address of the .text segment must be at an offset 0x01c6 bytes past a page
    boundary. The most obvious choice is to map .text to 0x00000000000001c6, but
    Linux doesn't like this; the resultant executable immediately segfaults. I
    imagine this is because of some special casing of the first page of virtual
    memory, but I haven't been able to find a reference. Instead, we just map .text
    to 0x00001000000001c6. This address apears in the ELF header as the entry point,
    and as the virtual address in both program and section headers for the .text
    segment.
*/

pub const ELF_HEADER: [u8; 64] = [
    0x7f, 0x45, 0x4c, 0x46, // Magic numbers
    0x02, 0x01, 0x01, 0x00, // 64-bit encoding; little-endian encoding; version 1; System V ABI
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Padding
    0x02, 0x00, // Type (executable file)
    0x3e, 0x00, // Architecture (AMD64)
    0x01, 0x00, 0x00, 0x00, // Version (1, again)
    0xc6, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, // Entry point
    0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Program header table offset
    0xb0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Section header table offset
    0x00, 0x00, 0x00, 0x00, // Flags (unused)
    0x40, 0x00, // Size of ELF header
    0x38, 0x00, // Size of program header
    0x02, 0x00, // Number of program headers (.text, .bss)
    0x40, 0x00, // Size of section header
    0x04, 0x00, // Number of sections headers (dummy section, .text, .bss, name table)
    0x03, 0x00, // Index of name table section
];

// Leading portion of the .text program header, up until the size fields
pub const TEXT_PROGRAM_HEADER_START: [u8; 32] = [
    0x01, 0x00, 0x00, 0x00, // Type (loadable segment)
    0x05, 0x00, 0x00, 0x00, // Flags (readable, executable),
    0xc6, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Offset
    0xc6, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, // Virtual address (same as entry point)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Physical address (unused)
];

// Trailing portion of the .text program header after the size fields
pub const TEXT_PROGRAM_HEADER_END: [u8; 8] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Alignment (unused)
];

// Leading portion of the .bss program header, up until the size field
pub const BSS_PROGRAM_HEADER_START: [u8; 40] = [
    0x01, 0x00, 0x00, 0x00, // Type (loadable segment)
    0x06, 0x00, 0x00, 0x00, // Flags (readable, writeable),
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Offset (unused)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, // Virtual address
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Physical address (unused)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Size on disk (zero)
];

// Trailing portion of the .bss program header after the size fields
pub const BSS_PROGRAM_HEADER_END: [u8; 8] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Alignment (unused)
];

pub const DUMMY_SECTION_HEADER: [u8; 64] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

// Leading portion of .text section header, up until the size field
pub const TEXT_SECTION_HEADER_START: [u8; 32] = [
    0x01, 0x00, 0x00, 0x00, // Name offset (1 byte into the table)
    0x01, 0x00, 0x00, 0x00, // Type (PROGBITS, i.e. program data)
    0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Flags (ALLOC [occupies memory], EXECINSTR [executable])
    0xc6, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, // Virtual address (same as entry point)
    0xc6, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Offset
];

// Trailing portion of .text section header after the size field
pub const TEXT_SECTION_HEADER_END: [u8; 24] = [
    0x00, 0x00, 0x00, 0x00, // Linked section (unused)
    0x00, 0x00, 0x00, 0x00, // Info field (unused)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Alignment (unused)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Entity size (unused)
];

// Leading portion of .bss section header, up until the size field
pub const BSS_SECTION_HEADER_START: [u8; 32] = [
    0x07, 0x00, 0x00, 0x00, // Name offset (7 bytes into the table)
    0x08, 0x00, 0x00, 0x00, // Type (NOBITS, i.e. occupies no space on disk)
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Flags (WRITE, ALLOC)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, // Virtual address
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Offset (unused)
];

// Trailing portion of the .bss section header after the size field
pub const BSS_SECTION_HEADER_END: [u8; 24] = [
    0x00, 0x00, 0x00, 0x00, // Linked section (unused)
    0x00, 0x00, 0x00, 0x00, // Info field (unused)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Alignment (unused)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Entity size (unused)
];

pub const STRING_TABLE_SECTION_HEADER: [u8; 64] = [
    0x0c, 0x00, 0x00, 0x00, // Name offset (12 bytes into the table)
    0x03, 0x00, 0x00, 0x00, // Type (string table)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Flags (none)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Virtual address (unused)
    0xb0, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Offset
    0x16, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Size (22 bytes)
    0x00, 0x00, 0x00, 0x00, // Linked section (unused)
    0x00, 0x00, 0x00, 0x00, // Info field (unused)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Alignment (unused)
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Entity size (unused)
];

pub const STRING_TABLE_CONTENTS: [u8; 22] = [
    0x00, // Unused index
    b'.', b't', b'e', b'x', b't', 0x00, // .text (offset 1)
    b'.', b'b', b's', b's', 0x00, // .bss (offset 7)
    b'.', b's', b'h', b's', b't', b'r', b't', b'a', b'b', 0x00, // .shstrtab (offset 12)
];

pub const TEXT_VIRTUAL_ADDRESS: u64 = 0x1000000001c6;
pub const BSS_VIRTUAL_ADDRESS: u64 = 0x600000000000;
pub const MAX_VIRTUAL_ADDRESS: u64 = 0x7fffffffffff;

pub const MAX_TEXT_SIZE: u64 = BSS_VIRTUAL_ADDRESS - TEXT_VIRTUAL_ADDRESS;
pub const MAX_BSS_SIZE: u64 = (1 + 0x7fffffffffff) - BSS_VIRTUAL_ADDRESS;
