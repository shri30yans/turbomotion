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


/// The typeface in `path`; `index` picks a face of a `.ttc` collection (0 for a plain font file).
pub fn from_file(path: &Path, index: u32) -> Result<Typeface> {
    let bytes = fs::read(path).with_context(|| format!("reading font {}", path.display()))?;
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
