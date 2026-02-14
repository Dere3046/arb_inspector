# arb_inspector

[English](README.md) | [中文](README_zh.md)

`arb_inspector` is a command-line tool for extracting OEM metadata from Qualcomm `xbl_config.img` firmware images, including the major version, minor version, and anti-rollback version.

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
arb_inspector <xbl_config.img>
```

- `<xbl_config.img>`: Path to the input firmware image file.

### Example

```bash
$ arb_inspector xbl_config.img
OEM Metadata from xbl_config.img:
  Major Version         : 3
  Minor Version         : 0
  Anti-Rollback Version : 0
```

## About Parsing Coverage

`arb_inspector` is based on analysis of known firmware samples and uses heuristic rules to locate the HASH segment and extract OEM metadata. While it works correctly on most devices, **it cannot guarantee parsing all versions of `xbl_config.img` files** for the following reasons:

- Firmware formats may change with vendor updates.
- Some customized firmwares may use non‑standard segment structures or encryption/compression.
- Heuristic rules need to balance accuracy and coverage, potentially missing some variants.

## If the Tool Fails to Parse Your File

If you encounter a parsing failure, feel free to submit an **Issue** on the GitHub repository, and provide the following information to help improve the tool:

- Device model and firmware source
- Attach the `xbl_config.img` file (if permitted)
- Expected ARB value (if known)
- Full output of the tool when run

We will continuously optimize the rules based on feedback to support more firmware versions. Thank you for your understanding and support!

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contact & Copyright Notice

For any questions or suggestions, please contact: **fine4trn@163.com**  
This tool is intended for learning and research purposes only. If any copyright infringement is found, modifications will be made as required.
