//! Everything wrong with a composition, reported together.
//!
//! A mistake in one layer must not hide the next: a film with five bad layers is fixed in one pass, not five. Parsing
//! and planning collect a [`Problem`] per mistake and fail once with all of them; callers that want to point at the
//! layer (the SDK's `check` maps each one to the script line that made it) read them with `downcast_ref::<Problems>()`.

use std::fmt;

/// One mistake, with the layer it belongs to when there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The composition holding the layer (empty for the root).
    pub comp: String,
    /// Position of the layer in its composition's list.
    pub layer: Option<usize>,
    /// The layer's `id`, if it has one.
    pub id: Option<String>,
    /// What is wrong, starting with the layer's label (`layer 2 (text "title"): ...`).
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problems(pub Vec<Problem>);

impl fmt::Display for Problems {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let [only] = self.0.as_slice() {
            return f.write_str(&only.message);
        }
        write!(f, "{} problems:", self.0.len())?;
        for (i, p) in self.0.iter().enumerate() {
            let scope = if p.comp.is_empty() { String::new() } else { format!("[comp {:?}] ", p.comp) };
            write!(f, "\n  {}. {scope}{}", i + 1, p.message.replace('\n', "\n     "))?;
        }
        Ok(())
    }
}

impl std::error::Error for Problems {}

impl Problems {
    /// Fail with every problem, or continue when there is none.
    pub fn finish(list: Vec<Problem>) -> anyhow::Result<()> {
        if list.is_empty() { Ok(()) } else { Err(Problems(list).into()) }
    }

    /// The same problems, tagged with the composition they were found in.
    pub fn in_comp(mut self, comp: &str) -> Problems {
        for p in &mut self.0 {
            p.comp = comp.to_string();
        }
        self
    }
}
