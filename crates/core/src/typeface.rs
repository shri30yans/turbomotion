//! Loading typefaces: from a file (including one face of a TrueType collection) or by family name.
//! Every renderer that draws text starts here; layout and styling are theirs.

use std::{fs, path::Path};

use anyhow::{anyhow, Context, Result};
use skia_safe::{font_style::{Slant, Weight, Width}, FontMgr, FontStyle, Typeface};

/// Face `index` of a TrueType collection as a standalone sfnt (macOS' font manager only opens the first face of a
/// collection from memory). Plain `.ttf` files pass through for index 0.
fn extract_face(bytes: &[u8], index: u32) -> Result<Vec<u8>> {
    let be32 = |at: usize| -> Result<u32> {
        let b = bytes.get(at..at + 4).context("truncated font file")?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    let be16 = |at: usize| -> Result<u16> {
        let b = bytes.get(at..at + 2).context("truncated font file")?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    };
    let dir = if bytes.starts_with(b"ttcf") {
        anyhow::ensure!(index < be32(8)?, "collection has {} faces", be32(8)?);
        be32(12 + 4 * index as usize)? as usize
    } else {
        anyhow::ensure!(index == 0, "not a collection");
        return Ok(bytes.to_vec());
    };
    let tables = be16(dir + 4)? as usize;
    let mut out = bytes.get(dir..dir + 12 + 16 * tables).context("truncated table directory")?.to_vec();
    let mut data = Vec::new();
    let data_start = out.len();
    for t in 0..tables {
        let rec = dir + 12 + 16 * t;
        let (offset, length) = (be32(rec + 8)? as usize, be32(rec + 12)? as usize);
        let body = bytes.get(offset..offset + length).context("table outside the file")?;
        let new_offset = (data_start + data.len()) as u32;
        out[12 + 16 * t + 8..12 + 16 * t + 12].copy_from_slice(&new_offset.to_be_bytes());
        data.extend_from_slice(body);
        data.resize(data.len().next_multiple_of(4), 0);
    }
    out.extend_from_slice(&data);
    Ok(out)
}


/// A WOFF 1.0 web font as a plain sfnt: each table is zlib-compressed when its stored length is shorter than its
/// original length. (WOFF2 needs a brotli decoder and a glyf transform; convert those to `.woff` or `.ttf` first.)
pub fn woff_to_sfnt(bytes: &[u8]) -> Result<Vec<u8>> {
    use std::io::Read;
    let be32 = |at: usize| -> Result<u32> {
        let b = bytes.get(at..at + 4).context("truncated WOFF file")?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    anyhow::ensure!(bytes.starts_with(b"wOFF"), "not a WOFF 1.0 file (WOFF2 is not supported; convert it to .woff or .ttf)");
    let flavor = be32(4)?;
    let tables = u16::from_be_bytes([*bytes.get(12).context("truncated WOFF header")?, *bytes.get(13).context("truncated WOFF header")?]) as usize;
    let dir_len = 12 + 16 * tables;
    let mut out = Vec::with_capacity(be32(16)? as usize);
    out.extend_from_slice(&flavor.to_be_bytes());
    out.extend_from_slice(&(tables as u16).to_be_bytes());
    let entry_selector = (usize::BITS - 1 - tables.max(1).leading_zeros()) as u16;
    let search_range = (1u16 << entry_selector) * 16;
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&entry_selector.to_be_bytes());
    out.extend_from_slice(&((tables as u16) * 16 - search_range).to_be_bytes());
    out.resize(dir_len, 0);
    for t in 0..tables {
        let rec = 44 + 20 * t;
        let (tag, offset, comp_len, orig_len, checksum) = (be32(rec)?, be32(rec + 4)? as usize, be32(rec + 8)? as usize, be32(rec + 12)? as usize, be32(rec + 16)?);
        let stored = bytes.get(offset..offset + comp_len).context("WOFF table outside the file")?;
        let table = if comp_len < orig_len {
            let mut raw = Vec::with_capacity(orig_len);
            flate2::read::ZlibDecoder::new(stored).read_to_end(&mut raw).context("inflating a WOFF table")?;
            anyhow::ensure!(raw.len() == orig_len, "WOFF table inflated to {} bytes, header says {orig_len}", raw.len());
            raw
        } else {
            stored.to_vec()
        };
        let at = out.len() as u32;
        out[12 + 16 * t..12 + 16 * t + 4].copy_from_slice(&tag.to_be_bytes());
        out[12 + 16 * t + 4..12 + 16 * t + 8].copy_from_slice(&checksum.to_be_bytes());
        out[12 + 16 * t + 8..12 + 16 * t + 12].copy_from_slice(&at.to_be_bytes());
        out[12 + 16 * t + 12..12 + 16 * t + 16].copy_from_slice(&(orig_len as u32).to_be_bytes());
        out.extend_from_slice(&table);
        out.resize(out.len().next_multiple_of(4), 0);
    }
    Ok(out)
}

/// The typeface in `path`; `index` picks a face of a `.ttc` collection (0 for a plain font file).
pub fn from_file(path: &Path, index: u32) -> Result<Typeface> {
    let bytes = fs::read(path).with_context(|| format!("reading font {}", path.display()))?;
    let bytes = if bytes.starts_with(b"wOFF") { woff_to_sfnt(&bytes).with_context(|| format!("decoding WOFF {}", path.display()))? } else { bytes };
    let single = extract_face(&bytes, index).with_context(|| format!("reading face {index} of {}", path.display()))?;
    FontMgr::new().new_from_data(&single, None).with_context(|| format!("decoding font {} (collection index {index})", path.display()))
}

/// An installed family at a weight (100..900) and slant.
pub fn from_family(family: &str, weight: u16, italic: bool) -> Result<Typeface> {
    let style = FontStyle::new(Weight::from(i32::from(weight)), Width::NORMAL, if italic { Slant::Italic } else { Slant::Upright });
    FontMgr::new().match_family_style(family, style).ok_or_else(|| anyhow!("no installed font family {family:?}"))
}

/// Names of the installed font families, sorted.
pub fn families() -> Vec<String> {
    let mgr = FontMgr::new();
    let mut names: Vec<String> = (0..mgr.count_families()).map(|i| mgr.family_name(i)).collect();
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-table WOFF built by hand: the table is stored uncompressed, so the sfnt must carry it verbatim.
    #[test]
    fn woff_header_and_stored_table_become_an_sfnt_directory() {
        let body = b"abcdefgh";
        let mut w = Vec::new();
        w.extend_from_slice(b"wOFF");
        w.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        w.extend_from_slice(&0u32.to_be_bytes());
        w.extend_from_slice(&1u16.to_be_bytes());
        w.extend_from_slice(&0u16.to_be_bytes());
        w.extend_from_slice(&(12 + 16 + body.len() as u32).to_be_bytes());
        w.resize(44, 0);
        w.extend_from_slice(b"head");
        w.extend_from_slice(&64u32.to_be_bytes());
        w.extend_from_slice(&(body.len() as u32).to_be_bytes());
        w.extend_from_slice(&(body.len() as u32).to_be_bytes());
        w.extend_from_slice(&7u32.to_be_bytes());
        w.extend_from_slice(body);
        let sfnt = woff_to_sfnt(&w).unwrap();
        assert_eq!(&sfnt[0..4], &[0, 1, 0, 0]);
        assert_eq!(&sfnt[12..16], b"head");
        let at = u32::from_be_bytes(sfnt[20..24].try_into().unwrap()) as usize;
        assert_eq!(&sfnt[at..at + 8], body);
    }

    #[test]
    fn a_file_that_is_not_woff_is_refused_with_the_reason() {
        let e = woff_to_sfnt(b"wOF2....").unwrap_err().to_string();
        assert!(e.contains("not a WOFF 1.0"), "{e}");
    }
}
