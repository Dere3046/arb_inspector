# arb_inspector

`arb_inspector` is a command-line tool for extracting OEM metadata from Qualcomm `xbl_config.img` firmware images, including the major version, minor version, and anti-rollback version.

[中文](README_zh.md)

## Features

- Parses ELF-format `xbl_config.img` files  
- Automatically locates and reads the HASH segment containing OEM metadata  
- Outputs OEM Major, Minor, and Anti-Rollback version information  
- Lightweight, relies only on the Rust standard library, no additional runtime required  

## How It Works

1. **ELF Parsing**  
   The tool first reads the ELF header of the input file, verifies that it is a valid 64-bit little-endian ELF file, and obtains the location and count of the program header table.

2. **Candidate Segment Collection**  
   It iterates through all program headers, selecting segments that exist within the file and have reasonable sizes as candidates (the HASH segment is typically among them).

3. **HASH Segment Identification**  
   For each candidate segment, it scans near the beginning with 4-byte alignment to locate a data structure matching the characteristics of a HASH segment header (containing version numbers and sizes of metadata regions).

4. **OEM Metadata Extraction**  
   Based on the offset from the HASH header, it calculates the start of the OEM metadata region and reads three 32-bit integers: Major, Minor, and Anti-Rollback version.

5. **Result Output**  
   The extracted three values are printed to the console for easy viewing.

## Usage

```bash
arb_inspector [--debug] [--block] <xbl_config.img>
```

- `<xbl_config.img>`: Path to the input firmware image file.  
- `--debug` (or `-d`): Optional flag to enable verbose output, showing which segments are scanned and why a particular segment is selected.
- `--block` (or `-b`): Enable block device mode. Skips file size boundary checks, useful when reading directly from Android partitions where `metadata().len()` may return incorrect values.

### Example

```bash
$ arb_inspector xbl_config.img
OEM Metadata from xbl_config.img:
  Major Version         : 3
  Minor Version         : 0
  Anti-Rollback Version : 0

$ arb_inspector --debug xbl_config.img
[DEBUG] Scanning segment 0 at file offset 0x0 (size 0x1c8)
[DEBUG] Scanning segment 6 at file offset 0x5d000 (size 0x130c)
[DEBUG] Segment at file offset 0x5d000: possible header at offset +0x4 (file 0x5d004)
[DEBUG]  -> OEM at +0x40 (file 0x5d040): major=3, minor=0, arb=0
[DEBUG] >>> SELECTED segment 6 (offset 0x5d000) with header at +0x4
OEM Metadata from xbl_config.img:
  Major Version         : 3
  Minor Version         : 0
  Anti-Rollback Version : 0
```

## Limitations

`arb_inspector` uses heuristic rules to locate the HASH segment based on analysis of known firmware samples. While it works correctly on most devices, **it cannot guarantee parsing all versions of `xbl_config.img` files** due to potential vendor‑specific variations, encryption, or future format changes. If the tool fails to parse your file, please run it with the `--debug` flag and open an issue on GitHub with the output and a copy of your file (if permitted).

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contact & Copyright Notice

For any questions or suggestions, please contact: **fine4trn@163.com**  
This tool is intended for learning and research purposes only. If any copyright infringement is found, modifications will be made as required.