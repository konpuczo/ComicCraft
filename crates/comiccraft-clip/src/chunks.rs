//! Outer container of a Clip Studio Paint `.clip` file.
//!
//! Layout (all integers big-endian), as documented by independent reverse-engineering work
//! (e.g. dobrokot/clip_to_psd):
//!
//! ```text
//! 24-byte file header : "CSFCHUNK" | u64 file size | u64 offset of first chunk (24)
//! repeated chunks     : "CHNK" | 4-byte name | u64 payload size | payload
//!                       names: "Head", "Exta" (external data, many), "SQLi" (SQLite DB), "Foot"
//! "Exta" payload      : u64 id length (40) | id ("extrnlid" + 32 hex) | u64 data size | data
//! ```

use crate::error::ClipError;

pub const MAGIC: &[u8; 8] = b"CSFCHUNK";
pub const FILE_HEADER_LEN: usize = 24;
pub const EXTA_ID_LEN: usize = 40;

#[derive(Debug, Clone, Copy)]
pub struct RawChunk<'a> {
    pub name: [u8; 4],
    /// Offset of the "CHNK" tag in the file.
    pub offset: usize,
    pub data: &'a [u8],
}

fn be_u64(d: &[u8], at: usize) -> Option<u64> {
    d.get(at..at + 8)
        .map(|b| u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
}

/// Splits a `.clip` file into its chunks without interpreting them.
pub fn parse_container(data: &[u8]) -> Result<Vec<RawChunk<'_>>, ClipError> {
    if data.len() < FILE_HEADER_LEN || &data[0..8] != MAGIC {
        return Err(ClipError::NotClip("missing CSFCHUNK header".into()));
    }
    let mut offset = FILE_HEADER_LEN;
    let mut chunks = Vec::new();
    while offset + 16 <= data.len() {
        if &data[offset..offset + 4] != b"CHNK" {
            return Err(ClipError::Malformed(format!("expected CHNK tag at offset {offset}")));
        }
        let name = [data[offset + 4], data[offset + 5], data[offset + 6], data[offset + 7]];
        let size = be_u64(data, offset + 8).unwrap_or(0) as usize;
        let start = offset + 16;
        let end = start
            .checked_add(size)
            .filter(|e| *e <= data.len())
            .ok_or_else(|| ClipError::Malformed(format!("chunk at offset {offset} overruns the file")))?;
        chunks.push(RawChunk { name, offset, data: &data[start..end] });
        offset = end;
    }
    if chunks.is_empty() {
        return Err(ClipError::Malformed("file contains no chunks".into()));
    }
    Ok(chunks)
}

/// Splits an "Exta" payload into (external id, data).
pub fn parse_exta(data: &[u8]) -> Result<(Vec<u8>, &[u8]), ClipError> {
    let bad = || ClipError::Malformed("truncated Exta chunk".into());
    let id_len = be_u64(data, 0).ok_or_else(bad)? as usize;
    let id = data.get(8..8 + id_len).ok_or_else(bad)?.to_vec();
    let size = be_u64(data, 8 + id_len).ok_or_else(bad)? as usize;
    let payload = data.get(16 + id_len..).ok_or_else(bad)?;
    if payload.len() != size {
        return Err(ClipError::Malformed(format!(
            "Exta size mismatch: header says {size}, found {}",
            payload.len()
        )));
    }
    Ok((id, payload))
}

pub fn build_exta(id: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + id.len() + payload.len());
    out.extend_from_slice(&(id.len() as u64).to_be_bytes());
    out.extend_from_slice(id);
    out.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Builds a container chunk by chunk and patches the header at the end.
pub struct ContainerWriter {
    buf: Vec<u8>,
}

impl ContainerWriter {
    pub fn new() -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);
        buf.extend_from_slice(&0u64.to_be_bytes()); // file size, patched in finish()
        buf.extend_from_slice(&(FILE_HEADER_LEN as u64).to_be_bytes());
        Self { buf }
    }

    /// Appends a chunk and returns the offset of its "CHNK" tag.
    pub fn add_chunk(&mut self, name: &[u8; 4], payload: &[u8]) -> usize {
        let offset = self.buf.len();
        self.buf.extend_from_slice(b"CHNK");
        self.buf.extend_from_slice(name);
        self.buf.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        self.buf.extend_from_slice(payload);
        offset
    }

    /// Overwrites 8 bytes at `pos` (used to back-patch the Head chunk).
    pub fn patch_u64(&mut self, pos: usize, value: u64) {
        self.buf[pos..pos + 8].copy_from_slice(&value.to_be_bytes());
    }

    pub fn finish(mut self) -> Vec<u8> {
        let len = self.buf.len() as u64;
        self.buf[8..16].copy_from_slice(&len.to_be_bytes());
        self.buf
    }
}

impl Default for ContainerWriter {
    fn default() -> Self {
        Self::new()
    }
}
