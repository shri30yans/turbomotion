//! Photographs and cut-out characters exported next to the film (`plates/<name>.png`, `chars/<pose>_<height>.png`),
//! decoded on first use and kept while recent.

use std::{fs, path::{Path, PathBuf}, rc::Rc};

use anyhow::{anyhow, Context, Result};
use skia_safe::{Data, Image};

use super::cache::Cache;

pub struct Media {
    dir: PathBuf,
    plates: Cache<String, Image>,
    characters: Cache<(String, i32), Image>,
}

impl Media {
    pub fn new(dir: &Path) -> Self {
        Self { dir: dir.to_path_buf(), plates: Cache::new(8), characters: Cache::new(16) }
    }

    /// The full-frame photograph called `name`.
    pub fn plate(&self, name: &str) -> Result<Rc<Image>> {
        self.plates.get_or_try_insert(name.to_string(), || self.decode(&format!("plates/{name}.png")))
    }

    /// The character in `pose`, rendered `height` pixels tall.
    pub fn character(&self, pose: &str, height: i32) -> Result<Rc<Image>> {
        self.characters.get_or_try_insert((pose.to_string(), height), || self.decode(&format!("chars/{pose}_{height}.png")))
    }

    fn decode(&self, relative: &str) -> Result<Image> {
        let path = self.dir.join(relative);
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        Image::from_encoded(Data::new_copy(&bytes)).ok_or_else(|| anyhow!("{} is not a decodable image", path.display()))
    }
}
