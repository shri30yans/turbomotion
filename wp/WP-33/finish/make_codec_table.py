#!/usr/bin/env python3
"""What survives an encode: the dark ramp films rendered to H.264 (software crf 0, 18 and hardware VideoToolbox), decoded and
measured on the luma plane. Writes codec-table.md. Run from turbomotion/ after make_films.py."""
import pathlib, subprocess, numpy as np

ROOT = pathlib.Path(__file__).resolve().parents[3]
BIN = ROOT / "target/release/turbomotion"
TMP = pathlib.Path("/tmp/fs-codec"); TMP.mkdir(exist_ok=True)
FILMS = ROOT / "wp/WP-33/finish/films"
W, H = 1280, 720


def luma(video):
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", video, "-frames:v", "1", "-vf", "format=gray", "-f", "rawvideo", "-"], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.uint8).reshape(H, W).astype(float)


def slabs(y):
    return y.reshape(H, 80, 16).mean(axis=(0, 2))


lines = ["| encode | rounding | distinct luma values | distinct 16-column means | max step between 16-column means (luma levels) | luma of the ramp's ends |", "|---|---|---|---|---|---|"]
for label, args in [("libx264 crf 0 (lossless)", ["--codec", "software", "--crf", "0"]), ("libx264 crf 18", ["--codec", "software", "--crf", "18"]), ("h264_videotoolbox 20 Mbit/s", ["--codec", "hardware", "--bitrate", "20M"])]:
    for kind in ["none", "ordered", "blue_noise"]:
        out = TMP / f"{label.split()[0]}-{args[-1]}-{kind}.mp4"
        subprocess.run([BIN, "comp", "render", FILMS / f"dark-ramp-{kind}.json", "--out", out, "--to", "2", "--workers", "1", *args], check=True, capture_output=True)
        y = luma(out)
        s = slabs(y)
        lines.append(f"| {label} | {kind} | {len(np.unique(y))} | {len(np.unique(s.round(6)))} | {np.abs(np.diff(s)).max():.3f} | {y[:, :16].mean():.1f} to {y[:, -16:].mean():.1f} |")
(pathlib.Path(__file__).resolve().parent / "codec-table.md").write_text("\n".join(lines) + "\n")
print("\n".join(lines))
