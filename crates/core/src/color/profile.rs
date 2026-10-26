//! What the numbers of a picture mean: a gamut and a transfer.

use skia_safe::{named_primaries::CicpId as PrimariesId, named_transfer_fn::CicpId as TransferId, ColorSpace};

/// The three colours a gamut's channels are, as ITU-T H.273 numbers them (the numbers video files tag themselves with).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primaries {
    /// BT.709, which sRGB shares.
    Srgb,
    /// P3 with the D65 white point.
    DisplayP3,
    /// BT.2020.
    Rec2020,
    /// BT.470 system B/G: 625-line standard definition video.
    Bt470Bg,
    /// SMPTE 170M: 525-line standard definition video.
    Smpte170m,
}

impl Primaries {
    pub const ALL: [Self; 5] = [Self::Srgb, Self::DisplayP3, Self::Rec2020, Self::Bt470Bg, Self::Smpte170m];

    /// ffmpeg's name for the gamut (`color_primaries` in a stream, `-color_primaries` on output).
    pub fn tag(self) -> &'static str {
        match self {
            Self::Srgb => "bt709",
            Self::DisplayP3 => "smpte432",
            Self::Rec2020 => "bt2020",
            Self::Bt470Bg => "bt470bg",
            Self::Smpte170m => "smpte170m",
        }
    }

    pub fn from_tag(tag: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.tag() == tag)
    }

    fn id(self) -> PrimariesId {
        match self {
            Self::Srgb => PrimariesId::Rec709,
            Self::DisplayP3 => PrimariesId::SMPTE_EG_432_1,
            Self::Rec2020 => PrimariesId::Rec2020,
            Self::Bt470Bg => PrimariesId::Rec470SystemBG,
            Self::Smpte170m => PrimariesId::Rec601,
        }
    }
}

/// How a channel relates to light.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transfer {
    /// The sRGB curve (IEC 61966-2-1): what a display-encoded picture and every colour literal use.
    Srgb,
    /// The channel is proportional to light.
    Linear,
}

impl Transfer {
    pub const ALL: [Self; 2] = [Self::Srgb, Self::Linear];

    /// ffmpeg's name for the curve (`color_transfer` in a stream, `-color_trc` on output).
    pub fn tag(self) -> &'static str {
        match self {
            Self::Srgb => "iec61966-2-1",
            Self::Linear => "linear",
        }
    }

    pub fn from_tag(tag: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.tag() == tag)
    }

    fn id(self) -> TransferId {
        match self {
            Self::Srgb => TransferId::IEC61966_2_1,
            Self::Linear => TransferId::Linear,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub primaries: Primaries,
    pub transfer: Transfer,
}

impl Profile {
    /// sRGB as a display encodes it: where colour literals and untagged media live.
    pub const SRGB: Profile = Profile { primaries: Primaries::Srgb, transfer: Transfer::Srgb };

    /// Skia's colour space for the profile.
    pub fn space(self) -> ColorSpace {
        ColorSpace::new_cicp(self.primaries.id(), self.transfer.id()).expect("every profile names a colour space Skia has (a test builds them all)")
    }
}
