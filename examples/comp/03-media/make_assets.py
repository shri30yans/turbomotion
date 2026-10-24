"""Generates the media this example uses (a procedural photograph, a depth map, a phone-orientation JPEG, a clip, a beat).
Run once: python3 make_assets.py. Nothing here is a real photograph; swap in your own files of the same names."""
import math, subprocess, wave
from pathlib import Path
import numpy as np
from PIL import Image, ImageFilter

here = Path(__file__).parent
rng = np.random.default_rng(7)

W, H = 3000, 2000
y, x = np.mgrid[0:H, 0:W].astype(np.float32)
sky = np.stack([40 + 120 * (1 - y / H), 60 + 110 * (1 - y / H), 120 + 100 * (1 - y / H)], -1)
sun = np.exp(-(((x - 2100) ** 2 + (y - 700) ** 2) / (2 * 160 ** 2)))[..., None] * np.array([255, 190, 110])
img = np.clip(sky + sun, 0, 255)
depth = np.zeros((H, W), np.float32)
for k, (base, amp, col, d) in enumerate([(1050, 160, (70, 90, 120), 0.25), (1250, 200, (45, 70, 80), 0.5), (1500, 260, (22, 40, 40), 0.8), (1800, 120, (10, 20, 18), 1.0)]):
    ridge = base + amp * (np.sin(x / (380 + 90 * k) + k) * 0.5 + np.sin(x / (170 + 40 * k) + 2 * k) * 0.3) 
    mask = y > ridge
    img[mask] = np.array(col)
    depth[mask] = d
img += rng.normal(0, 3.5, img.shape)
Image.fromarray(np.clip(img, 0, 255).astype(np.uint8)).save(here / "photo.jpg", quality=92)
Image.fromarray((np.clip(depth, 0, 1) * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(6)).save(here / "depth.png")

# a portrait photo stored sideways with EXIF orientation 6, as phones write them
p = Image.fromarray(np.clip(img[:1200, :1800], 0, 255).astype(np.uint8)).rotate(90, expand=True).rotate(180)
exif = p.getexif(); exif[0x0112] = 6
p.save(here / "phone.jpg", quality=90, exif=exif)

subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "lavfi", "-i", "testsrc2=size=640x360:rate=30:duration=3", "-f", "lavfi", "-i", "sine=frequency=330:duration=3", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest", str(here / "clip.mp4")], check=True)

sr = 44100
t = np.arange(int(sr * 6)) / sr
beat = np.zeros_like(t)
for k in range(12):
    s = t - k * 0.5
    m = s >= 0
    beat[m] += np.sin(2 * math.pi * (140 * np.exp(-s[m] * 28) + 48) * s[m]) * np.exp(-s[m] * 9) * 0.9
beat += 0.05 * np.sin(2 * math.pi * 220 * t) * (0.5 + 0.5 * np.sin(2 * math.pi * 0.25 * t))
with wave.open(str(here / "beat.wav"), "wb") as w:
    w.setnchannels(1); w.setsampwidth(2); w.setframerate(sr)
    w.writeframes((np.clip(beat, -1, 1) * 32000).astype(np.int16).tobytes())
print("assets written to", here)
