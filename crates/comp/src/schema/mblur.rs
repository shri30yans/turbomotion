//! The composition key `"mblur"`: how many sub-frames are averaged and how long the shutter is open.

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(default, deny_unknown_fields)]
pub struct MotionBlur {
    /// Sub-frames averaged per frame.
    pub samples: u32,
    /// Shutter angle in degrees (180 = half the frame interval).
    pub shutter: f64,
}

impl Default for MotionBlur {
    fn default() -> Self {
        MotionBlur { samples: 8, shutter: 180.0 }
    }
}
