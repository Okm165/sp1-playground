use anyhow::Context;
use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;


use curta_core::runtime::instruction::Instruction;
use elf::ElfBytes;

use elf::endian::LittleEndian;
use elf::file::Class;

pub const MAX_MEM: u32 = u32::MAX;
pub const WORD_SIZE: usize = 4;

pub fn parse_elf(input: &[u8]) -> Result<(Vec<Instruction>, u32)> {
    let elf = ElfBytes::<LittleEndian>::minimal_parse(input)
        .map_err(|err| anyhow!("Elf parse error: {err}"))?;
    if elf.ehdr.class != Class::ELF32 {
        bail!("Not a 32-bit ELF");
    }
    if elf.ehdr.e_machine != elf::abi::EM_RISCV {
        bail!("Invalid machine type, must be RISC-V");
    }
    if elf.ehdr.e_type != elf::abi::ET_EXEC {
        bail!("Invalid ELF type, must be executable");
    }
    let entry: u32 = elf
        .ehdr
        .e_entry
        .try_into()
        .map_err(|err| anyhow!("e_entry was larger than 32 bits. {err}"))?;

    if entry >= MAX_MEM || entry % WORD_SIZE as u32 != 0 {
        bail!("Invalid entrypoint");
    }
    let segments = elf.segments().ok_or(anyhow!("Missing segment table"))?;
    if segments.len() > 256 {
        bail!("Too many program headers");
    }
    let mut instructions : Vec<Instruction> = Vec::new();

    let mut first_memory_address_in_segment = None;

    // Only read segments that are executable instructions that are also PT_LOAD.
    for segment in segments.iter().filter(|x| x.p_type == elf::abi::PT_LOAD && ((x.p_flags & elf::abi::PF_X) != 0)) {
        let file_size: u32 = segment
            .p_filesz
            .try_into()
            .map_err(|err| anyhow!("filesize was larger than 32 bits. {err}"))?;
        if file_size >= MAX_MEM {
            bail!("Invalid segment file_size");
        }
        let mem_size: u32 = segment
            .p_memsz
            .try_into()
            .map_err(|err| anyhow!("mem_size was larger than 32 bits {err}"))?;
        if mem_size >= MAX_MEM {
            bail!("Invalid segment mem_size");
        }
        let vaddr: u32 = segment
            .p_vaddr
            .try_into()
            .map_err(|err| anyhow!("vaddr is larger than 32 bits. {err}"))?;
        if vaddr % WORD_SIZE as u32 != 0 {
            bail!("vaddr {vaddr:08x} is unaligned");
        }
        if first_memory_address_in_segment.is_none() {
            first_memory_address_in_segment = Some(vaddr);
        } else if first_memory_address_in_segment.unwrap() > vaddr {
            first_memory_address_in_segment = Some(vaddr);
        }
        let offset: u32 = segment
            .p_offset
            .try_into()
            .map_err(|err| anyhow!("offset is larger than 32 bits. {err}"))?;
        for i in (0..mem_size).step_by(WORD_SIZE) {
            let addr = vaddr.checked_add(i).context("Invalid segment vaddr")?;
            if addr >= MAX_MEM {
                bail!("Address [0x{addr:08x}] exceeds maximum address for guest programs [0x{MAX_MEM:08x}]");
            }
            if i >= file_size {
                // Past the file size, all zeros.
            } else {
                let mut word = 0;
                // Don't read past the end of the file.
                let len = core::cmp::min(file_size - i, WORD_SIZE as u32);
                for j in 0..len {
                    let offset = (offset + i + j) as usize;
                    let byte = input.get(offset).context("Invalid segment offset")?;
                    word |= (*byte as u32) << (j * 8);
                }
                instructions.push(Instruction::decode(word));
            }
        }
    }
    match first_memory_address_in_segment {
        Some(addr) => {
            Ok((instructions, entry - addr))
        }
        None => {
            bail!("No executable segments found");
        }
    }
}