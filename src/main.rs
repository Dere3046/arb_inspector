use std::env;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

// ELF64 constants
const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;

// HASH segment header scan limits
const HASH_HDR_SIZE: usize = 36;
const HASH_SCAN_MAX: usize = 0x1000;      // scan only first 4KB of a segment
const MAX_SEGMENT_SIZE: u64 = 20 * 1024 * 1024; // 20 MB safety cap

// Sanity ranges for version numbers (loose, to catch obvious garbage)
const MIN_VERSION: u32 = 1;
const MAX_VERSION: u32 = 1000;
const MAX_COMMON_SZ: usize = 0x1000;
const MAX_QTI_SZ: usize = 0x1000;
const MAX_OEM_SZ: usize = 0x4000;
const MAX_ARB: u32 = 127; // ARB values are typically small

/// Read a little‑endian u16 from a byte slice.
fn read_le_u16(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes(buf[off..off + 2].try_into().unwrap())
}

/// Read a little‑endian u32 from a byte slice.
fn read_le_u32(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(buf[off..off + 4].try_into().unwrap())
}

/// Read a little‑endian u64 from a byte slice.
fn read_le_u64(buf: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(buf[off..off + 8].try_into().unwrap())
}

/// Try to locate a HASH segment header inside a candidate segment.
/// Returns the offset (relative to segment start) if a plausible header is found.
fn find_hash_header(seg: &[u8]) -> Option<usize> {
    let seg_len = seg.len();
    // Scan in 4‑byte steps (the header is 4‑byte aligned in practice)
    for off in (0..HASH_SCAN_MAX.min(seg_len)).step_by(4) {
        if off + HASH_HDR_SIZE > seg_len {
            break;
        }

        let version = read_le_u32(seg, off);
        let common_sz = read_le_u32(seg, off + 4) as usize;
        let qti_sz = read_le_u32(seg, off + 8) as usize;
        let oem_sz = read_le_u32(seg, off + 12) as usize;
        let hash_tbl_sz = read_le_u32(seg, off + 16) as usize;

        // Validate plausible header fields
        if !(MIN_VERSION..=MAX_VERSION).contains(&version) {
            continue;
        }
        if common_sz > MAX_COMMON_SZ || qti_sz > MAX_QTI_SZ || oem_sz > MAX_OEM_SZ {
            continue;
        }
        // Hash table size should be non‑zero and a multiple of 32 (common for hash entries)
        if hash_tbl_sz == 0 || (hash_tbl_sz & 0x1F) != 0 {
            continue;
        }
        // Ensure the described regions fit inside the segment
        if off + HASH_HDR_SIZE + common_sz + qti_sz + oem_sz > seg_len {
            continue;
        }

        // All checks passed – this looks like a valid HASH header
        return Some(off);
    }
    None
}

/// Represents the data extracted from a valid HASH segment (only OEM metadata).
struct HashInfo {
    pub oem_major: u32,
    pub oem_minor: u32,
    pub oem_arb: u32,
}

/// Parse a candidate segment, verify its header, and extract OEM metadata if valid.
fn try_extract_hash_info(seg_data: &[u8]) -> Option<HashInfo> {
    let header_off = find_hash_header(seg_data)?;

    let common_sz = read_le_u32(seg_data, header_off + 4) as usize;
    let qti_sz = read_le_u32(seg_data, header_off + 8) as usize;

    // OEM metadata starts after common + QTI areas
    let oem_off = header_off + HASH_HDR_SIZE + common_sz + qti_sz;
    if oem_off + 12 > seg_data.len() {
        return None; // not enough room for three u32s
    }

    let major = read_le_u32(seg_data, oem_off);
    let minor = read_le_u32(seg_data, oem_off + 4);
    let arb = read_le_u32(seg_data, oem_off + 8);

    // Basic sanity on OEM numbers (avoid obviously corrupted data)
    if major > MAX_VERSION || minor > MAX_VERSION || arb > MAX_ARB {
        return None;
    }

    Some(HashInfo {
        oem_major: major,
        oem_minor: minor,
        oem_arb: arb,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <xbl_config.img>", args[0]);
        std::process::exit(1);
    }
    let path = &args[1];

    let mut file = File::open(path)?;
    let file_size = file.metadata()?.len();

    // 1. Read and verify ELF header
    let mut ehdr = [0u8; 64];
    file.read_exact(&mut ehdr)?;

    if &ehdr[0..4] != ELF_MAGIC {
        return Err("Not an ELF file".into());
    }
    if ehdr[EI_CLASS] != ELFCLASS64 {
        return Err("Not a 64‑bit ELF file".into());
    }
    if ehdr[EI_DATA] != ELFDATA2LSB {
        return Err("Not a little‑endian ELF file".into());
    }

    // Extract ELF header fields needed to locate program headers
    let e_phoff = read_le_u64(&ehdr, 0x20);
    let e_phentsz = read_le_u16(&ehdr, 0x36) as usize;
    let e_phnum = read_le_u16(&ehdr, 0x38) as usize;

    if e_phentsz < 56 /* typical size of Elf64_Phdr */ || e_phnum == 0 {
        return Err("Invalid program header table".into());
    }

    // 2. Collect candidate segments (those that could contain the HASH data)
    //    We will scan all program headers and keep those with non‑zero file size,
    //    that fit in the file, and are not larger than MAX_SEGMENT_SIZE.
    let mut candidates = Vec::new();
    for i in 0..e_phnum {
        let ph_offset = e_phoff + (i as u64) * e_phentsz as u64;
        file.seek(SeekFrom::Start(ph_offset))?;
        let mut ph_buf = [0u8; 56]; // read enough for a standard Elf64_Phdr
        file.read_exact(&mut ph_buf)?;

        let p_offset = read_le_u64(&ph_buf, 8);
        let p_filesz = read_le_u64(&ph_buf, 32);

        if p_filesz == 0 {
            continue;
        }
        // Basic bounds checks
        if p_offset + p_filesz > file_size {
            eprintln!("Warning: segment {} exceeds file size, skipping", i);
            continue;
        }
        if p_filesz > MAX_SEGMENT_SIZE {
            eprintln!("Warning: segment {} is too large ({} bytes), skipping", i, p_filesz);
            continue;
        }
        // Record the offset and size
        candidates.push((p_offset, p_filesz));
    }

    // 3. Scan candidates to locate the HASH segment and extract OEM metadata
    let mut hash_info = None;
    for (off, size) in candidates {
        let mut seg_data = vec![0u8; size as usize];
        file.seek(SeekFrom::Start(off))?;
        file.read_exact(&mut seg_data)?;

        if let Some(info) = try_extract_hash_info(&seg_data) {
            hash_info = Some(info);
            break;
        }
    }

    let hash_info = hash_info.ok_or("No valid HASH segment with OEM metadata found")?;

    // 4. Output only the OEM metadata (Major, Minor, Anti-Rollback Version)
    println!("OEM Metadata from {}:", path);
    println!("  Major Version         : {}", hash_info.oem_major);
    println!("  Minor Version         : {}", hash_info.oem_minor);
    println!("  Anti-Rollback Version : {}", hash_info.oem_arb);

    Ok(())
}