//! Audio range regression without a GPU or full video render.
use super::*;
use serde_json::json;

fn source(path: &Path) {
    // Two half-second plateaus make a source-time mistake unambiguous.
    let count = SAMPLE_RATE;
    let mut wav = Vec::new();
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + count * 4).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&3u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 4).to_le_bytes());
    wav.extend_from_slice(&4u16.to_le_bytes());
    wav.extend_from_slice(&32u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(count * 4).to_le_bytes());
    for i in 0..count { wav.extend_from_slice(&(if i < count/2 { 0.1f32 } else { 0.6f32 }).to_le_bytes()); }
    fs::write(path, wav).unwrap();
}
fn samples(path: &Path) -> Vec<f32> {
    fs::read(path).unwrap()[44..].chunks_exact(4).map(|b| f32::from_le_bytes(b.try_into().unwrap())).collect()
}
#[test]
fn interval_audio_matches_the_same_interval_of_the_full_mix() {
    let dir = std::env::temp_dir().join(format!("turbomotion-audio-range-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    source(&dir.join("source.wav"));
    let plan = Plan::from_value(json!({"width":32,"height":32,"fps":30,"dur":1,
        "layers":[{"type":"audio","src":"source.wav","dur":1,"fade_in":0.8,"fade_out":0.4,
            "volume":{"from":0.3,"to":0.9,"dur":1,"ease":"none"}}]}), &dir).unwrap();
    mix(&plan,1.0,&dir.join("full.wav")).unwrap();
    mix_range(&plan,0.6..0.8,&dir.join("range.wav")).unwrap();
    let full=samples(&dir.join("full.wav"));
    let range=samples(&dir.join("range.wav"));
    assert_eq!(range.len(), (0.2*f64::from(SAMPLE_RATE)).round() as usize*2);
    let offset=(0.6*f64::from(SAMPLE_RATE)).round() as usize*2;
    assert!(range.iter().zip(&full[offset..]).all(|(a,b)| (a-b).abs()<1e-5));
    assert!(range[100]>0.1, "must use the second source plateau");
    assert!(mix_range(&plan,0.5..0.5,&dir.join("bad.wav")).is_err());
}
