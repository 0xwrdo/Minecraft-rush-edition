//! Zero-dependency pure-Rust PNG encoder for dumping 2D noise slices.
//!
//! Produces standard RFC 2083 PNG images using uncompressed Deflate blocks (RFC 1951).
//! Contains NO graphics/GPU dependencies and runs completely headless.

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Standard CRC-32 (IEEE 802.3)
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Standard Adler-32 (RFC 1950)
fn adler32(data: &[u8]) -> u32 {
    let mut s1 = 1u32;
    let mut s2 = 0u32;
    for &byte in data {
        s1 = (s1 + byte as u32) % 65521;
        s2 = (s2 + s1) % 65521;
    }
    (s2 << 16) | s1
}

fn write_chunk<W: Write>(w: &mut W, chunk_type: &[u8; 4], data: &[u8]) -> io::Result<()> {
    w.write_all(&(data.len() as u32).to_be_bytes())?;
    w.write_all(chunk_type)?;
    w.write_all(data)?;

    let mut crc_buf = Vec::with_capacity(4 + data.len());
    crc_buf.extend_from_slice(chunk_type);
    crc_buf.extend_from_slice(data);
    let checksum = crc32(&crc_buf);
    w.write_all(&checksum.to_be_bytes())?;
    Ok(())
}

/// Save an 8-bit grayscale image buffer to a PNG file.
pub fn save_grayscale_png(path: &Path, width: u32, height: u32, pixels: &[u8]) -> io::Result<()> {
    assert_eq!(
        pixels.len(),
        (width * height) as usize,
        "Pixel buffer size must match width * height"
    );

    let mut file = File::create(path)?;

    // 1. PNG Signature
    file.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])?;

    // 2. IHDR Chunk (13 bytes)
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // Bit depth: 8
    ihdr.push(0); // Color type: 0 (Grayscale)
    ihdr.push(0); // Compression method: 0 (Deflate)
    ihdr.push(0); // Filter method: 0
    ihdr.push(0); // Interlace: 0 (None)
    write_chunk(&mut file, b"IHDR", &ihdr)?;

    // 3. Raw Scanline Data (Each row prefixed with filter byte 0 = None)
    let row_len = (width + 1) as usize;
    let mut raw_data = Vec::with_capacity(row_len * height as usize);
    for y in 0..height as usize {
        raw_data.push(0); // Filter byte: None
        let row_start = y * width as usize;
        let row_end = row_start + width as usize;
        raw_data.extend_from_slice(&pixels[row_start..row_end]);
    }

    // 4. Wrap raw data in RFC 1950 Zlib container with uncompressed RFC 1951 Deflate blocks
    let mut idat = Vec::new();
    idat.push(0x78); // CMF: Deflate, 32K window
    idat.push(0x01); // FLG: Check bits, no preset dictionary

    // Split raw scanlines into chunks of at most 65535 bytes
    let chunks = raw_data.chunks(65535).collect::<Vec<_>>();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_last = i == chunks.len() - 1;
        idat.push(if is_last { 0x01 } else { 0x00 }); // BFINAL and BTYPE=00 (uncompressed)
        let len = chunk.len() as u16;
        let nlen = !len;
        idat.extend_from_slice(&len.to_le_bytes());
        idat.extend_from_slice(&nlen.to_le_bytes());
        idat.extend_from_slice(chunk);
    }

    // Adler-32 checksum of uncompressed raw_data
    let checksum = adler32(&raw_data);
    idat.extend_from_slice(&checksum.to_be_bytes());

    write_chunk(&mut file, b"IDAT", &idat)?;

    // 5. IEND Chunk
    write_chunk(&mut file, b"IEND", &[])?;

    file.flush()?;
    Ok(())
}
