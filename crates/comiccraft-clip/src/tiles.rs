//! 256x256 tiled raster compression and decompression for Clip Studio Paint (.clip).
//!
//! Real CSP stores pixel data in external (`CHNKExta`) chunks divided into 256x256 subblocks.
//! Each subblock is wrapped in `BlockDataBeginChunk` and `BlockDataEndChunk` UTF-16BE tags.
//! Non-empty blocks are compressed with zlib and have 5 channels:
//! 1 byte Alpha channel (65,536 bytes) followed by 4 bytes BGRX (262,144 bytes).

use crate::error::ClipError;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{Read, Write};

pub const TILE_SIZE: u32 = 256;

// UTF-16BE: "BlockDataBeginChunk" (19 chars = 38 bytes)
pub const BLOCK_DATA_BEGIN_CHUNK: [u8; 38] = [
    0x00, b'B', 0x00, b'l', 0x00, b'o', 0x00, b'c', 0x00, b'k', 0x00, b'D', 0x00, b'a', 0x00,
    b't', 0x00, b'a', 0x00, b'B', 0x00, b'e', 0x00, b'g', 0x00, b'i', 0x00, b'n', 0x00, b'C',
    0x00, b'h', 0x00, b'u', 0x00, b'n', 0x00, b'k',
];

// UTF-16BE: "BlockDataEndChunk" (17 chars = 34 bytes)
pub const BLOCK_DATA_END_CHUNK: [u8; 34] = [
    0x00, b'B', 0x00, b'l', 0x00, b'o', 0x00, b'c', 0x00, b'k', 0x00, b'D', 0x00, b'a', 0x00,
    b't', 0x00, b'a', 0x00, b'E', 0x00, b'n', 0x00, b'd', 0x00, b'C', 0x00, b'h', 0x00, b'u',
    0x00, b'n', 0x00, b'k',
];

// UTF-16BE: "BlockStatus" (11 chars = 22 bytes)
pub const BLOCK_STATUS: [u8; 22] = [
    0x00, b'B', 0x00, b'l', 0x00, b'o', 0x00, b'c', 0x00, b'k', 0x00, b'S', 0x00, b't', 0x00,
    b'a', 0x00, b't', 0x00, b'u', 0x00, b's',
];

// UTF-16BE: "BlockCheckSum" (13 chars = 26 bytes)
pub const BLOCK_CHECKSUM: [u8; 26] = [
    0x00, b'B', 0x00, b'l', 0x00, b'o', 0x00, b'c', 0x00, b'k', 0x00, b'C', 0x00, b'h', 0x00,
    b'e', 0x00, b'c', 0x00, b'k', 0x00, b'S', 0x00, b'u', 0x00, b'm',
];

fn be_u32(d: &[u8], at: usize) -> Option<u32> {
    d.get(at..at + 4)
        .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// Encodes an RGBA pixel buffer into the authentic CSP 256x256 tiled subblock stream.
pub fn encode_tiles(width: u32, height: u32, rgba_pixels: &[u8]) -> Result<Vec<u8>, ClipError> {
    let grid_w = (width + TILE_SIZE - 1) / TILE_SIZE;
    let grid_h = (height + TILE_SIZE - 1) / TILE_SIZE;
    let mut out = Vec::new();

    let mut tile_rgba = vec![0u8; (TILE_SIZE * TILE_SIZE * 4) as usize];
    let k = (TILE_SIZE * TILE_SIZE) as usize;
    let mut uncompressed_5ch = vec![0u8; k * 5];

    for gy in 0..grid_h {
        for gx in 0..grid_w {
            let block_idx = gy * grid_w + gx;
            tile_rgba.fill(0);

            // Copy layer pixels into 256x256 tile
            let mut has_non_zero_alpha = false;
            for ty in 0..TILE_SIZE {
                let cy = gy * TILE_SIZE + ty;
                if cy >= height {
                    continue;
                }
                for tx in 0..TILE_SIZE {
                    let cx = gx * TILE_SIZE + tx;
                    if cx >= width {
                        continue;
                    }
                    let src_idx = ((cy * width + cx) * 4) as usize;
                    let dst_idx = ((ty * TILE_SIZE + tx) * 4) as usize;

                    if src_idx + 4 <= rgba_pixels.len() {
                        let a = rgba_pixels[src_idx + 3];
                        if a > 0 {
                            has_non_zero_alpha = true;
                        }
                        tile_rgba[dst_idx..dst_idx + 4].copy_from_slice(&rgba_pixels[src_idx..src_idx + 4]);
                    }
                }
            }

            // Build inner block
            let mut block = Vec::new();
            block.extend_from_slice(&block_idx.to_be_bytes());
            block.extend_from_slice(&0u32.to_be_bytes());
            block.extend_from_slice(&0u32.to_be_bytes());
            block.extend_from_slice(&0u32.to_be_bytes());

            if !has_non_zero_alpha {
                // Empty block: has_data = 0
                block.extend_from_slice(&0u32.to_be_bytes());
            } else {
                // Non-empty block: has_data = 1, packing into 5 channels (1 alpha + 4 BGRX)
                block.extend_from_slice(&1u32.to_be_bytes());

                for i in 0..k {
                    let r = tile_rgba[i * 4];
                    let g = tile_rgba[i * 4 + 1];
                    let b = tile_rgba[i * 4 + 2];
                    let a = tile_rgba[i * 4 + 3];

                    // Alpha in first k bytes
                    uncompressed_5ch[i] = a;
                    // BGRX in k..5*k bytes
                    uncompressed_5ch[k + i * 4] = b;
                    uncompressed_5ch[k + i * 4 + 1] = g;
                    uncompressed_5ch[k + i * 4 + 2] = r;
                    uncompressed_5ch[k + i * 4 + 3] = 0;
                }

                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&uncompressed_5ch)?;
                let compressed = encoder.finish()?;

                let subblock_len = (compressed.len() + 4) as u32;
                block.extend_from_slice(&subblock_len.to_be_bytes()); // BE
                block.extend_from_slice(&(compressed.len() as u32).to_le_bytes()); // LE!
                block.extend_from_slice(&compressed);
            }

            // Wrap with BlockDataBeginChunk and BlockDataEndChunk
            let total_size = (4 + 4 + BLOCK_DATA_BEGIN_CHUNK.len() + block.len() + 4 + BLOCK_DATA_END_CHUNK.len()) as u32;

            out.extend_from_slice(&total_size.to_be_bytes());
            out.extend_from_slice(&19u32.to_be_bytes());
            out.extend_from_slice(&BLOCK_DATA_BEGIN_CHUNK);
            out.extend_from_slice(&block);
            out.extend_from_slice(&17u32.to_be_bytes());
            out.extend_from_slice(&BLOCK_DATA_END_CHUNK);
        }
    }

    Ok(out)
}

/// Decodes an authentic CSP tiled subblock stream into an RGBA pixel buffer.
pub fn decode_tiles(width: u32, height: u32, exta_data: &[u8]) -> Result<Vec<u8>, ClipError> {
    let grid_w = (width + TILE_SIZE - 1) / TILE_SIZE;
    let grid_h = (height + TILE_SIZE - 1) / TILE_SIZE;
    let total_pixels = (width * height * 4) as usize;
    let mut rgba = vec![0u8; total_pixels];

    let mut offset = 0;
    while offset < exta_data.len() {
        // Skip optional BlockStatus or BlockCheckSum
        if offset + 4 + BLOCK_STATUS.len() <= exta_data.len()
            && &exta_data[offset..offset + 4] == &11u32.to_be_bytes()
            && &exta_data[offset + 4..offset + 4 + BLOCK_STATUS.len()] == &BLOCK_STATUS
        {
            let status_count = be_u32(exta_data, offset + 30).unwrap_or(0) as usize;
            let block_size = status_count * 4 + 12 + (BLOCK_STATUS.len() + 4);
            offset += block_size;
            continue;
        }

        if offset + 4 + BLOCK_CHECKSUM.len() <= exta_data.len()
            && &exta_data[offset..offset + 4] == &13u32.to_be_bytes()
            && &exta_data[offset + 4..offset + 4 + BLOCK_CHECKSUM.len()] == &BLOCK_CHECKSUM
        {
            let block_size = 4 + BLOCK_CHECKSUM.len() + 12 + (grid_w * grid_h * 4) as usize;
            offset += block_size;
            continue;
        }

        // Check for BlockDataBeginChunk
        if offset + 8 + BLOCK_DATA_BEGIN_CHUNK.len() > exta_data.len() {
            break;
        }

        let block_size = match be_u32(exta_data, offset) {
            Some(s) if s >= 84 => s as usize,
            _ => break,
        };

        if offset + block_size > exta_data.len() {
            break;
        }

        if &exta_data[offset + 8..offset + 8 + BLOCK_DATA_BEGIN_CHUNK.len()] != &BLOCK_DATA_BEGIN_CHUNK {
            offset += 4;
            continue;
        }

        let end_tag_len = 4 + BLOCK_DATA_END_CHUNK.len();
        let block_start = offset + 8 + BLOCK_DATA_BEGIN_CHUNK.len();
        let block_end = offset + block_size - end_tag_len;

        if block_start < block_end && block_end <= exta_data.len() {
            let block = &exta_data[block_start..block_end];
            if block.len() >= 20 {
                let subblock_idx = be_u32(block, 0).unwrap_or(0);
                let has_data = be_u32(block, 16).unwrap_or(0) == 1;

                if has_data && block.len() > 28 {
                    let compressed = &block[28..];
                    let mut decoder = ZlibDecoder::new(compressed);
                    let mut decompressed = Vec::new();
                    if decoder.read_to_end(&mut decompressed).is_ok() {
                        let k = (TILE_SIZE * TILE_SIZE) as usize;
                        let gx = subblock_idx % grid_w;
                        let gy = subblock_idx / grid_w;

                        if decompressed.len() == k * 5 {
                            // 5-channel RGBA: 1 alpha + 4 BGRX
                            let alpha = &decompressed[0..k];
                            let bgrx = &decompressed[k..5 * k];

                            for ty in 0..TILE_SIZE {
                                let cy = gy * TILE_SIZE + ty;
                                if cy >= height {
                                    continue;
                                }
                                for tx in 0..TILE_SIZE {
                                    let cx = gx * TILE_SIZE + tx;
                                    if cx >= width {
                                        continue;
                                    }
                                    let tile_i = (ty * TILE_SIZE + tx) as usize;
                                    let b = bgrx[tile_i * 4];
                                    let g = bgrx[tile_i * 4 + 1];
                                    let r = bgrx[tile_i * 4 + 2];
                                    let a = alpha[tile_i];

                                    let dst_idx = ((cy * width + cx) * 4) as usize;
                                    if dst_idx + 4 <= rgba.len() {
                                        rgba[dst_idx] = r;
                                        rgba[dst_idx + 1] = g;
                                        rgba[dst_idx + 2] = b;
                                        rgba[dst_idx + 3] = a;
                                    }
                                }
                            }
                        } else if decompressed.len() == k {
                            // 1-channel Grayscale mask
                            for ty in 0..TILE_SIZE {
                                let cy = gy * TILE_SIZE + ty;
                                if cy >= height {
                                    continue;
                                }
                                for tx in 0..TILE_SIZE {
                                    let cx = gx * TILE_SIZE + tx;
                                    if cx >= width {
                                        continue;
                                    }
                                    let val = decompressed[(ty * TILE_SIZE + tx) as usize];
                                    let dst_idx = ((cy * width + cx) * 4) as usize;
                                    if dst_idx + 4 <= rgba.len() {
                                        rgba[dst_idx] = val;
                                        rgba[dst_idx + 1] = val;
                                        rgba[dst_idx + 2] = val;
                                        rgba[dst_idx + 3] = val;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        offset += block_size;
    }

    Ok(rgba)
}
