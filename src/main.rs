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
const HASH_SCAN_MAX: usize = 0x1000;          // only scan first 4KB of a segment
const MAX_SEGMENT_SIZE: u64 = 20 * 1024 * 1024; // 20 MB safety cap

// Sanity ranges for header fields
const MIN_VERSION: u32 = 1;
const MAX_VERSION: u32 = 1000;
const MAX_COMMON_SZ: usize = 0x1000;
const MAX_QTI_SZ: usize = 0x1000;
const MAX_OEM_SZ: usize = 0x4000;
const MAX_HASH_TBL_SZ: usize = 0x10000;       // 64KB upper bound
const MAX_ARB: u32 = 127;                      // ARB values are typically small

fn read_le_u16(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes(buf[off..off + 2].try_into().unwrap())
}

fn read_le_u32(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(buf[off..off + 4].try_into().unwrap())
}

fn read_le_u64(buf: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(buf[off..off + 8].try_into().unwrap())
}

fn find_hash_header(seg: &[u8], debug: bool, _seg_idx: usize, seg_off: u64) -> Option<usize> {
    let seg_len = seg.len();
    for off in (0..HASH_SCAN_MAX.min(seg_len)).step_by(4) {
        if off + HASH_HDR_SIZE > seg_len {
            break;
        }

        let version = read_le_u32(seg, off);
        let common_sz = read_le_u32(seg, off + 4) as usize;
        let qti_sz = read_le_u32(seg, off + 8) as usize;
        let oem_sz = read_le_u32(seg, off + 12) as usize;
        let hash_tbl_sz = read_le_u32(seg, off + 16) as usize;

        // version should be within known range
        if !(MIN_VERSION..=MAX_VERSION).contains(&version) {
            continue;
        }
        // individual region sizes must not be excessive
        if common_sz > MAX_COMMON_SZ || qti_sz > MAX_QTI_SZ || oem_sz > MAX_OEM_SZ {
            continue;
        }
        // hash table size must be non‑zero and not implausibly large
        if hash_tbl_sz == 0 || hash_tbl_sz > MAX_HASH_TBL_SZ {
            continue;
        }
        // all described regions must fit inside the segment
        if off + HASH_HDR_SIZE + common_sz + qti_sz + oem_sz > seg_len {
            continue;
        }

        if debug {
            eprintln!(
                "[DEBUG] Segment at file offset 0x{:x}: possible header at offset +0x{:x} (file 0x{:x})",
                seg_off, off, seg_off + off as u64
            );
        }

        return Some(off);
    }
    None
}

struct HashInfo {
    oem_major: u32,
    oem_minor: u32,
    oem_arb: u32,
    used_seg_off: u64,
    used_header_off: usize,
}

fn try_extract_hash_info(seg_data: &[u8], seg_off: u64, debug: bool, seg_idx: usize) -> Option<HashInfo> {
    let header_off = find_hash_header(seg_data, debug, seg_idx, seg_off)?;

    let common_sz = read_le_u32(seg_data, header_off + 4) as usize;
    let qti_sz = read_le_u32(seg_data, header_off + 8) as usize;

    // OEM metadata starts right after common + QTI areas
    let oem_off = header_off + HASH_HDR_SIZE + common_sz + qti_sz;
    if oem_off + 12 > seg_data.len() {
        return None;
    }

    let major = read_le_u32(seg_data, oem_off);
    let minor = read_le_u32(seg_data, oem_off + 4);
    let arb = read_le_u32(seg_data, oem_off + 8);

    // reject values that are clearly out of range
    if major > MAX_VERSION || minor > MAX_VERSION || arb > MAX_ARB {
        return None;
    }

    if debug {
        eprintln!(
            "[DEBUG]  -> OEM at +0x{:x} (file 0x{:x}): major={}, minor={}, arb={}",
            oem_off,
            seg_off + oem_off as u64,
            major,
            minor,
            arb
        );
    }

    Some(HashInfo {
        oem_major: major,
        oem_minor: minor,
        oem_arb: arb,
        used_seg_off: seg_off,
        used_header_off: header_off,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let (debug, path) = if args.len() == 3 && (args[1] == "--debug" || args[1] == "-d") {
        (true, args[2].clone())
    } else if args.len() == 2 {
        (false, args[1].clone())
    } else {
        eprintln!("Usage: {} [--debug] <xbl_config.img>", args[0]);
        std::process::exit(1);
    };

    let mut file = File::open(&path)?;
    let file_size = file.metadata()?.len();

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

    let e_phoff = read_le_u64(&ehdr, 0x20);
    let e_phentsz = read_le_u16(&ehdr, 0x36) as usize;
    let e_phnum = read_le_u16(&ehdr, 0x38) as usize;

    if e_phentsz < 56 || e_phnum == 0 {
        return Err("Invalid program header table".into());
    }

    // Collect candidates, prioritizing PT_NULL (type 0) segments
    let mut null_candidates = Vec::new(); // (file_offset, size, index)
    let mut other_candidates = Vec::new();

    for i in 0..e_phnum {
        let ph_offset = e_phoff + (i as u64) * e_phentsz as u64;
        file.seek(SeekFrom::Start(ph_offset))?;
        let mut ph_buf = [0u8; 56];
        file.read_exact(&mut ph_buf)?;

        let p_type = read_le_u32(&ph_buf, 0);
        let p_offset = read_le_u64(&ph_buf, 8);
        let p_filesz = read_le_u64(&ph_buf, 32);

        if p_filesz == 0 {
            continue;
        }
        if p_offset + p_filesz > file_size {
            eprintln!("Warning: segment {} exceeds file size, skipping", i);
            continue;
        }
        if p_filesz > MAX_SEGMENT_SIZE {
            eprintln!("Warning: segment {} too large ({} bytes), skipping", i, p_filesz);
            continue;
        }

        let candidate = (p_offset, p_filesz, i);
        if p_type == 0 {
            null_candidates.push(candidate);
        } else {
            other_candidates.push(candidate);
        }
    }

    // Helper to scan a list of candidates
    fn scan_candidates(
        file: &mut File,
        candidates: &[(u64, u64, usize)],
        debug: bool,
    ) -> Result<Option<HashInfo>, std::io::Error> {
        for &(off, size, idx) in candidates {
            if debug {
                eprintln!("[DEBUG] Scanning segment {} at file offset 0x{:x} (size 0x{:x})", idx, off, size);
            }
            let mut seg_data = vec![0u8; size as usize];
            file.seek(SeekFrom::Start(off))?;
            file.read_exact(&mut seg_data)?;

            if let Some(info) = try_extract_hash_info(&seg_data, off, debug, idx) {
                if debug {
                    eprintln!(
                        "[DEBUG] >>> SELECTED segment {} (offset 0x{:x}) with header at +0x{:x}",
                        idx, info.used_seg_off, info.used_header_off
                    );
                }
                return Ok(Some(info));
            }
        }
        Ok(None)
    }

    // Try PT_NULL segments first, fall back to others if none work
    let hash_info = if let Some(info) = scan_candidates(&mut file, &null_candidates, debug)? {
        info
    } else if let Some(info) = scan_candidates(&mut file, &other_candidates, debug)? {
        info
    } else {
        return Err("No valid HASH segment with OEM metadata found".into());
    };

    println!("OEM Metadata from {}:", path);
    println!("  Major Version         : {}", hash_info.oem_major);
    println!("  Minor Version         : {}", hash_info.oem_minor);
    println!("  Anti-Rollback Version : {}", hash_info.oem_arb);

    Ok(())
}
