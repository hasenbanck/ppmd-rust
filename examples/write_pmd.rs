//! Writes a `.pmd` file (the header of Shkarin's `ppmd` tool followed by a
//! PPMd var.H stream) from standard input, so the output can be checked
//! against another reader such as 7-Zip's PPMd handler.
//!
//! Usage: write_pmd OUT.pmd NAME ORDER MEM_MB < input
use std::io::{Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = &args[1];
    let name = args[2].as_bytes();
    let order: u32 = args[3].parse().unwrap();
    let mem_mb: u32 = args[4].parse().unwrap();
    let mut data = Vec::new();
    std::io::stdin().read_to_end(&mut data).unwrap();
    let mut file = Vec::new();
    file.extend_from_slice(&0x84AC_AF8Fu32.to_le_bytes());
    file.extend_from_slice(&0x20u32.to_le_bytes()); // attributes
    let info: u16 = ((order - 1) as u16) | (((mem_mb - 1) as u16) << 4) | (7 << 12);
    file.extend_from_slice(&info.to_le_bytes());
    file.extend_from_slice(&(name.len() as u16).to_le_bytes());
    file.extend_from_slice(&0x5A21_0000u32.to_le_bytes()); // a DOS time
    file.extend_from_slice(name);
    let mut encoder = ppmd_rust::Ppmd7aEncoder::new(file, order, mem_mb << 20).unwrap();
    encoder.write_all(&data).unwrap();
    let file = encoder.finish(true).unwrap();
    std::fs::write(out, file).unwrap();
}
