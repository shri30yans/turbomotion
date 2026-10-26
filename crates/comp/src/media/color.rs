//! What the numbers in a decoded file mean. Stills carry their own profile; a video says so in its stream tags, and
//! an untagged one is taken to be BT.709, as players do. Only a colour-managed film (`schema::color`) looks at this:
//! an unmanaged one draws every file's numbers as they are.

use anyhow::{bail, Result};
use serde_json::Value;
use skia_safe::{codec::Codec, ColorSpace};
use turbomotion_core::color::{Primaries, Profile, Transfer};

/// The colour a video stream is tagged with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoColor {
    /// A gamut and a curve Turbomotion can place.
    Sdr(Profile),
    /// PQ or HLG: light levels beyond the display's white, which this renderer has no way to map.
    Hdr,
    /// Primaries or a transfer outside the supported list.
    Unplaced,
}

impl VideoColor {
    /// Read ffprobe's `color_primaries` and `color_transfer` for a stream.
    pub fn from_stream(stream: &Value) -> Self {
        let tag = |key: &str| stream[key].as_str().filter(|t| *t != "unknown").unwrap_or("");
        let transfer = match tag("color_transfer") {
            // an untagged stream, and the BT.709 family of curves, which are close enough to sRGB's for display video
            "" | "bt709" | "smpte170m" | "bt2020-10" | "bt2020-12" => Transfer::Srgb,
            "smpte2084" | "arib-std-b67" => return Self::Hdr,
            other => match Transfer::from_tag(other) {
                Some(transfer) => transfer,
                None => return Self::Unplaced,
            },
        };
        let primaries = match tag("color_primaries") {
            "" => Primaries::Srgb,
            other => match Primaries::from_tag(other) {
                Some(primaries) => primaries,
                None => return Self::Unplaced,
            },
        };
        Self::Sdr(Profile { primaries, transfer })
    }

    /// The colour space its decoded frames are in, or why Turbomotion cannot say.
    pub fn space(self, as_written: &str) -> Result<ColorSpace> {
        match self {
            Self::Sdr(profile) => Ok(profile.space()),
            Self::Hdr => bail!("{as_written:?} is HDR (PQ or HLG); Turbomotion has no tone mapping, so convert it to SDR first, for example with ffmpeg's zscale and tonemap filters"),
            Self::Unplaced => bail!("{as_written:?} is tagged with primaries or a transfer Turbomotion cannot place (primaries: {}; curves: sRGB, BT.709, linear); re-encode it as BT.709", Primaries::ALL.map(Primaries::tag).join(", ")),
        }
    }
}

/// The colour space a still's pixels are in: its embedded profile, or sRGB when it has none.
pub fn still_space(codec: &Codec) -> ColorSpace {
    codec.info().color_space().unwrap_or_else(ColorSpace::new_srgb)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use serde_json::json;
    use turbomotion_core::{
        color::Primaries,
        encode::{Codec, Encoder, VideoFormat},
    };

    use super::*;
    use crate::media::{probe, video::Videos};

    fn tagged(primaries: &str, transfer: &str) -> VideoColor {
        VideoColor::from_stream(&json!({"color_primaries": primaries, "color_transfer": transfer}))
    }

    #[test]
    fn a_stream_is_read_as_the_profile_it_is_tagged_with() {
        let p3 = Profile { primaries: Primaries::DisplayP3, transfer: Transfer::Srgb };
        assert_eq!(tagged("smpte432", "iec61966-2-1"), VideoColor::Sdr(p3));
        assert_eq!(tagged("bt709", "bt709"), VideoColor::Sdr(Profile::SRGB), "BT.709 is read with sRGB's curve");
        assert_eq!(VideoColor::from_stream(&json!({})), VideoColor::Sdr(Profile::SRGB), "an untagged stream is BT.709, as players take it");
        assert_eq!(tagged("unknown", "unknown"), VideoColor::Sdr(Profile::SRGB));
        assert_eq!(tagged("bt2020", "bt2020-10"), VideoColor::Sdr(Profile { primaries: Primaries::Rec2020, transfer: Transfer::Srgb }));
    }

    #[test]
    fn hdr_and_unplaceable_streams_are_refused_with_a_way_out() {
        assert_eq!(tagged("bt2020", "smpte2084"), VideoColor::Hdr);
        assert_eq!(tagged("bt2020", "arib-std-b67"), VideoColor::Hdr);
        assert_eq!(tagged("film", "bt709"), VideoColor::Unplaced);
        assert_eq!(tagged("bt709", "log100"), VideoColor::Unplaced);
        let e = VideoColor::Hdr.space("clip.mov").unwrap_err().to_string();
        assert!(e.contains("clip.mov") && e.contains("HDR") && e.contains("tonemap"), "{e}");
        let e = VideoColor::Unplaced.space("clip.mov").unwrap_err().to_string();
        assert!(e.contains("clip.mov") && e.contains("smpte432"), "{e}");
    }

    #[test]
    fn a_managed_film_refuses_an_hdr_video_before_decoding_and_an_unmanaged_one_does_not_look() {
        let info = probe::VideoInfo { w: 64, h: 64, fps: 10.0, dur: 1.0, has_audio: false, color: VideoColor::Hdr };
        let e = Videos::new(true).frame(Path::new("/no/such/hdr.mov"), &info, 0.0, 64).unwrap_err().to_string();
        assert!(e.contains("HDR"), "{e}");
        let e = Videos::new(false).frame(Path::new("/no/such/hdr.mov"), &info, 0.0, 64).unwrap_err().to_string();
        assert!(!e.contains("HDR"), "an unmanaged film draws the numbers as they are: {e}");
    }

    #[test]
    fn the_tags_the_encoder_writes_are_the_ones_the_probe_reads_back() {
        let p3 = Profile { primaries: Primaries::DisplayP3, transfer: Transfer::Srgb };
        let path = std::env::temp_dir().join(format!("tm-videocolor-{}.mp4", std::process::id()));
        let mut enc = Encoder::spawn(&path, VideoFormat { width: 64, height: 64, fps: 10, profile: Some(p3) }, &Codec::Software { crf: 30, preset: "ultrafast".into() }).unwrap();
        for _ in 0..3 {
            enc.write(&vec![0x80; 64 * 64 * 4]).unwrap();
        }
        enc.finish().unwrap();
        let info = probe::video(&path, "probe.mp4").unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(info.color, VideoColor::Sdr(p3));
    }
}
