//! `turbomotion comp …`: render a composition, look at it, and ask it questions.

use std::{fs, path::PathBuf, time::Instant};

use anyhow::{ensure, Context, Result};
use clap::{Args, Subcommand, ValueEnum};
use turbomotion_core::encode::Codec;

use crate::{catalogue, check, eval::Evaluator, media::refframe::{Crop, RefVideo}, plan::{Plan, ROOT}, render};

#[derive(Args)]
pub struct Source {
    /// The composition file (`comp.json`); relative paths inside it are resolved from its folder
    pub file: PathBuf,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum CodecKind {
    /// Hardware H.264 (VideoToolbox), quality set by `--bitrate`
    Hardware,
    /// libx264 on the CPU, quality set by `--crf`
    Software,
    /// ProRes 4444 with transparency (write to a `.mov`): overlays and mattes for an editor
    Alpha,
}

#[derive(Subcommand)]
pub enum FxCommand {
    /// Every effect and generator in the built-in pack and the project's `effects/` folder, with parameters, as JSON
    List {
        /// The project folder (the one holding `effects/`)
        #[arg(long, default_value = ".")]
        dir: PathBuf,
    },
    /// Render one effect or generator with each float parameter swept over five values, as a contact sheet
    Test {
        name: String,
        #[arg(long, default_value = ".")]
        dir: PathBuf,
        /// A picture to apply an effect to (default: a built-in test card)
        #[arg(long)]
        on: Option<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum Command {
    /// Print everything a composition can contain, with a working example of each, as JSON
    Catalogue,
    /// Load and validate a composition; print what it contains. Fails with a path to the first mistake
    Check {
        #[command(flatten)]
        source: Source,
        /// Assertions to verify (default: `comp.assert.json` beside the composition, if there is one)
        #[arg(long)]
        assert: Option<PathBuf>,
        /// Print the findings as JSON
        #[arg(long)]
        json: bool,
        /// Estimate effect surface work per composition (no GPU timing prediction)
        #[arg(long)]
        cost: bool,
        /// Treat warnings as failures
        #[arg(long)]
        strict: bool,
        /// Also render a frame twice and require identical pixels
        #[arg(long)]
        determinism: bool,
        /// Also render the foreground layers of every `layout` composition and report overlaps and crowding
        #[arg(long)]
        layout: bool,
    },
    /// List the installed font families a text layer can name
    Fonts,
    /// Reusable parametrised sub-compositions: the built-in ones and any in `blocks/`, with their variables, as JSON
    Blocks {
        #[arg(long, default_value = ".")]
        dir: PathBuf,
    },
    /// Shader plugins: list what is available, or render one with each knob swept across its range
    Fx {
        #[command(subcommand)]
        command: FxCommand,
    },
    /// What is true at a moment: every layer's position, opacity, bounds and state, as JSON
    Inspect {
        #[command(flatten)]
        source: Source,
        /// Seconds into the composition
        #[arg(long)]
        t: f64,
        /// Only this layer: its id, or a path through precompositions such as intro/title (a bare id works when only one layer has it); bounds use its containing comp
        #[arg(long)]
        layer: Option<String>,
    },
    /// How a moment is drawn: the render graph's passes and the images each reads and writes
    Graph {
        #[command(flatten)]
        source: Source,
        /// Seconds into the composition
        #[arg(long, default_value_t = 0.0)]
        t: f64,
    },
    /// Write PNGs of chosen moments
    Still {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        out: PathBuf,
        /// Times in seconds (comma separated)
        #[arg(long, value_delimiter = ',', required = true)]
        times: Vec<f64>,
    },
    /// One labelled contact sheet of several moments, for reviewing motion at a glance
    Sheet {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        out: PathBuf,
        /// Times in seconds (comma separated); default: eight evenly spread moments
        #[arg(long, value_delimiter = ',')]
        times: Vec<f64>,
        #[arg(long, default_value_t = 4)]
        cols: u32,
        /// Width of each thumbnail in pixels (with --ref, at most 2 pairs per row)
        #[arg(long, default_value_t = 480)]
        thumb: u32,
        /// A reference video: its frame at each time is drawn beside ours, labelled REF / OURS
        #[arg(long = "ref")]
        reference: Option<PathBuf>,
        /// Crop the reference to `x,y,w,h` (its own pixels), to remove letterbox bars
        #[arg(long, requires = "reference")]
        crop: Option<String>,
        /// Seconds added to every time when reading the reference (if it starts earlier or later than the film)
        #[arg(long, default_value_t = 0.0, requires = "reference")]
        ref_offset: f64,
    },
    /// Wrap a composition for another aspect ratio (16:9 film to 9:16 short) without editing it; writes a new comp file
    Reframe {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        out: PathBuf,
        /// Target size, `WxH`
        #[arg(long, default_value = "1080x1920")]
        size: String,
        /// `cover` crops to fill the frame; `contain` fits it over a blurred copy
        #[arg(long, default_value = "cover")]
        mode: String,
        /// Where a crop is centred horizontally: 0 left, 0.5 middle, 1 right
        #[arg(long, default_value_t = 0.5)]
        focus: f64,
    },
    /// Convert an SVG into a shape layer (JSON) that can be pasted into a composition's `layers`
    Svg {
        file: PathBuf,
        /// Layer id
        #[arg(long, default_value = "art")]
        id: String,
        /// Width in pixels the artwork should have in the composition
        #[arg(long, default_value_t = 400.0)]
        width: f64,
        /// Write the layer here instead of printing it
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Re-check a composition and re-draw its contact sheet whenever the file changes (Ctrl-C to stop)
    Watch {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        out: PathBuf,
        /// Times in seconds (comma separated); default: eight evenly spread moments
        #[arg(long, value_delimiter = ',')]
        times: Vec<f64>,
        /// Poll interval in milliseconds
        #[arg(long, default_value_t = 400)]
        interval: u64,
        /// Do one pass and exit (for scripts and tests)
        #[arg(long)]
        once: bool,
    },
    /// Render the composition (or a range of frames) to an mp4
    Render {
        #[command(flatten)]
        source: Source,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, default_value_t = 0)]
        from: u32,
        /// Last frame, exclusive (default: the end)
        #[arg(long)]
        to: Option<u32>,
        #[arg(long, default_value_t = 3)]
        workers: u32,
        #[arg(long, value_enum, default_value_t = CodecKind::Hardware)]
        codec: CodecKind,
        /// Hardware codec bitrate, e.g. `12M`. Left out, it follows the picture: 0.07 bits per pixel per frame
        /// (between 3M and 40M), which is about 7 Mbit/s for 1080p30 and 14 for 1080p60.
        #[arg(long)]
        bitrate: Option<String>,
        #[arg(long, default_value_t = 17)]
        crf: u32,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Catalogue => {
            println!("{}", serde_json::to_string_pretty(&catalogue::all())?);
            Ok(())
        }
        Command::Fonts => {
            for f in turbomotion_core::typeface::families() {
                println!("{f}");
            }
            Ok(())
        }
        Command::Blocks { dir } => {
            let list: Vec<_> = crate::schema::blocks::available(&dir, &[])?
                .into_iter()
                .filter_map(|b| {
                    let v: serde_json::Value = serde_json::from_str(&b.source).ok()?;
                    Some(serde_json::json!({
                        "block": b.name,
                        "job": v.get("job").and_then(|j| j.as_str()).unwrap_or(&b.job),
                        "best_for": v.get("best_for"),
                        "breaks": v.get("breaks"),
                        "effect": v.get("effect"),
                        "size": [v["width"], v["height"]],
                        "dur": v["dur"],
                        "vars": v["vars"],
                        "use": {"type": "precomp", "block": b.name, "vars": {}, "pos": [960, 540]}
                    }))
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&list)?);
            Ok(())
        }
        Command::Fx { command } => fx(command),
        Command::Graph { source, t } => {
            let plan = Plan::load(&source.file)?;
            print!("{}", render::graph::describe(&plan, &Evaluator::new(&plan), t)?);
            Ok(())
        }
        Command::Check { source, assert, json, cost, strict, determinism, layout } => {
            let plan = Plan::load(&source.file)?;
            let root = plan.root();
            // Evaluate the timeline at the interesting moments so expression, font and reference mistakes surface now.
            let ev = Evaluator::new(&plan);
            let mut times = vec![0.0, root.dur / 2.0, (root.dur - 1.0 / f64::from(plan.fps)).max(0.0)];
            for l in &root.layers {
                times.push(l.start + 0.01);
                times.push((l.start + l.end) / 2.0);
                times.push((l.end - 0.02).max(l.start));
            }
            times.retain(|t| *t >= 0.0 && *t < root.dur);
            times.sort_by(f64::total_cmp);
            times.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
            for t in times.iter().take(96) {
                ev.frame(ROOT, *t).with_context(|| format!("evaluating the composition at {t:.2}s"))?;
            }
            let mut findings = check::lint::run(&plan)?;
            let assert_path = assert.or_else(|| Some(source.file.with_file_name("comp.assert.json")).filter(|p| p.exists()));
            let mut goldens: Vec<(f64, String, f64)> = vec![];
            let asserted = if let Some(p) = &assert_path {
                let list = check::assertions::load(p)?;
                goldens = list
                    .iter()
                    .filter_map(|a| match a {
                        check::assertions::Assertion::Golden { t, file, rmse } => Some((*t, file.clone(), *rmse)),
                        _ => None,
                    })
                    .collect();
                findings.extend(check::assertions::run(&plan, &list)?);
                list.len()
            } else {
                0
            };
            if layout {
                findings.extend(check::layout::run(&plan)?);
            }
            if !goldens.is_empty() {
                let mut r = render::Renderer::new(&plan)?;
                let dir = source.file.parent().unwrap_or(std::path::Path::new("."));
                for (t, file, max_rmse) in &goldens {
                    r.draw_at(*t)?;
                    let mut now = vec![];
                    r.pixels(&mut now)?;
                    let path = dir.join(file);
                    if !path.exists() {
                        if let Some(p) = path.parent() {
                            fs::create_dir_all(p).ok();
                        }
                        fs::write(&path, r.png()?).with_context(|| format!("writing golden {}", path.display()))?;
                        findings.push(check::Finding { severity: check::Severity::Warn, rule: "wrote-golden", layer: None, t: Some(*t), message: format!("wrote {} (first run; re-run check to compare)", path.display()), fix: "commit the PNG; later runs fail if this frame changes".into() });
                        continue;
                    }
                    let got = r.png()?;
                    let want = fs::read(&path).with_context(|| format!("reading golden {}", path.display()))?;
                    if got == want {
                        continue;
                    }
                    // same pixels can still encode differently; fall back to a coarse RMSE on the raw frame vs a decode of the file
                    let rmse = png_rmse(&now, &want, root.width, root.height).unwrap_or(255.0);
                    if rmse > *max_rmse {
                        findings.push(check::Finding { severity: check::Severity::Error, rule: "golden", layer: None, t: Some(*t), message: format!("{} differs from the golden (rmse {rmse:.1}, max {max_rmse:.0})", path.display()), fix: "if the change is intended, delete the PNG and re-run check to write a new golden".into() });
                    }
                }
            }
            if determinism {
                let mut r = render::Renderer::new(&plan)?;
                let (mut a, mut b) = (vec![], vec![]);
                let t = root.dur / 2.0;
                r.draw_at(t)?;
                r.pixels(&mut a)?;
                r.draw_at(0.0)?;
                r.draw_at(t)?;
                r.pixels(&mut b)?;
                if a != b {
                    findings.push(check::Finding { severity: check::Severity::Error, rule: "determinism", layer: None, t: Some(t), message: "the same frame drew differently the second time".into(), fix: "something depends on state outside time: an unseeded random, a cache keyed wrongly, a video decoder restart".into() });
                }
            }
            let errors = findings.iter().filter(|f| f.severity == check::Severity::Error).count();
            let warnings = findings.len() - errors;
            let cost_report = cost.then(|| check::cost::estimate(&plan)).transpose()?;
            if json {
                if let Some(cost) = &cost_report {
                    println!("{}", serde_json::to_string_pretty(&serde_json::json!({"findings":findings,"cost":cost}))?);
                } else { println!("{}", serde_json::to_string_pretty(&findings)?); }
            } else {
                if let Some(cost) = &cost_report {
                    println!("Effect cost: {}", cost.units);
                    for row in &cost.compositions {
                        println!("  {}: {:.3} MPx effects, {:.3} MPx glow, {} stages", if row.comp.is_empty() { "root" } else { &row.comp }, row.effect_surface_mpx, row.glow_surface_mpx, row.effect_stages);
                    }
                    println!("{}", cost.method);
                }
                for f in &findings {
                    println!("{f}");
                }
                println!(
                    "{}: {}x{} at {} fps, {:.2}s ({} frames), {} layers, {} nested comps; {} assertions; {errors} errors, {warnings} warnings",
                    if errors == 0 { "ok" } else { "FAILED" },
                    root.width, root.height, plan.fps, root.dur, plan.frames(), root.layers.len(), plan.comps.len() - 1, asserted
                );
            }
            ensure!(errors == 0 && !(strict && warnings > 0), "{errors} errors, {warnings} warnings");
            Ok(())
        }
        Command::Reframe { source, out, size, mode, focus } => {
            let (w, h) = size.split_once('x').and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?))).with_context(|| format!("--size {size:?} should look like 1080x1920"))?;
            let text = fs::read_to_string(&source.file).with_context(|| format!("reading {}", source.file.display()))?;
            let wrapped = crate::tools::reframe::reframe(&serde_json::from_str(&text).context("the composition is not valid JSON")?, &crate::tools::reframe::Options { width: w, height: h, mode: mode.parse()?, focus })?;
            // The wrapped file must load before it is offered: resolve it from the source's folder so relative assets still work.
            let dir = source.file.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(std::path::Path::new("."));
            Plan::from_value(wrapped.clone(), dir).context("the reframed composition does not load")?;
            fs::write(&out, serde_json::to_string_pretty(&wrapped)?).with_context(|| format!("writing {}", out.display()))?;
            println!("{}x{} {mode} reframe of {} -> {} (keep it in the same folder so relative assets resolve)", w, h, source.file.display(), out.display());
            Ok(())
        }
        Command::Svg { file, id, width, out } => {
            let text = fs::read_to_string(&file).with_context(|| format!("reading {}", file.display()))?;
            let imported = crate::tools::svg::import(&text, &id, width)?;
            for w in &imported.warnings {
                eprintln!("warning: {w}");
            }
            let json = serde_json::to_string_pretty(&imported.layer)?;
            match out {
                Some(p) => {
                    fs::write(&p, json).with_context(|| format!("writing {}", p.display()))?;
                    println!("{} items -> {}", imported.layer["items"].as_array().map_or(0, Vec::len), p.display());
                }
                None => println!("{json}"),
            }
            Ok(())
        }
        Command::Watch { source, out, times, interval, once } => watch(&source.file, &out, &times, interval, once),
        Command::Inspect { source, t, layer } => {
            let plan = Plan::load(&source.file)?;
            let evaluated = Evaluator::new(&plan).frame(ROOT, t)?;
            let frame = match layer {
                Some(id) => {
                    let (containing, selected) = crate::inspect::select(&evaluated, &crate::lookup::resolve(&plan, &id)?)?;
                    let mut value = serde_json::to_value(containing)?;
                    value["layers"] = serde_json::json!([selected]);
                    value
                }
                None => serde_json::to_value(&evaluated)?,
            };
            println!("{}", serde_json::to_string_pretty(&frame)?);
            Ok(())
        }
        Command::Still { source, out, times } => {
            let plan = Plan::load(&source.file)?;
            fs::create_dir_all(&out).with_context(|| format!("creating {}", out.display()))?;
            let mut renderer = render::Renderer::new(&plan)?;
            for t in times {
                renderer.draw_at(t)?;
                let path = out.join(format!("{t:07.2}.png"));
                fs::write(&path, renderer.png()?).with_context(|| format!("writing {}", path.display()))?;
                println!("still {t:.2}s -> {}", path.display());
            }
            Ok(())
        }
        Command::Sheet { source, out, times, cols, thumb, reference, crop, ref_offset } => {
            let reference = reference
                .map(|path| -> Result<RefVideo> {
                    RefVideo::exists(&path)?;
                    Ok(RefVideo { path, crop: crop.as_deref().map(Crop::parse).transpose()?, offset: ref_offset })
                })
                .transpose()?;
            let plan = Plan::load(&source.file)?;
            let times = if times.is_empty() { (0..8).map(|i| plan.root().dur * (f64::from(i) + 0.5) / 8.0).collect() } else { times };
            let mut renderer = render::Renderer::new(&plan)?;
            let cols = if reference.is_some() { cols.min(2) } else { cols };
            fs::write(&out, renderer.sheet_against(&times, cols.max(1), thumb.max(64), reference.as_ref())?).with_context(|| format!("writing {}", out.display()))?;
            println!("sheet of {} moments -> {}", times.len(), out.display());
            Ok(())
        }
        Command::Render { source, out, from, to, workers, codec, bitrate, crf } => {
            let plan = Plan::load(&source.file)?;
            let frames = from..to.unwrap_or(plan.frames());
            ensure!(frames.end <= plan.frames() && frames.start < frames.end, "frame range {frames:?} is outside the composition ({} frames)", plan.frames());
            let codec = match codec {
                CodecKind::Hardware => Codec::Hardware { bitrate: bitrate.unwrap_or_else(|| auto_bitrate(plan.comps[crate::plan::ROOT].width, plan.comps[crate::plan::ROOT].height, f64::from(plan.fps))) },
                CodecKind::Software => Codec::Software { crf, preset: "faster".into() },
                CodecKind::Alpha => {
                    ensure!(out.extension().is_some_and(|e| e == "mov"), "--codec alpha writes ProRes 4444, so name the output .mov");
                    Codec::Alpha
                }
            };
            let started = Instant::now();
            let n = frames.len();
            render::render_video(&source.file, &codec, frames, workers, &out)?;
            let wall = started.elapsed().as_secs_f64();
            println!("{n} frames in {wall:.2}s = {:.1} fps ({workers} workers) -> {}", n as f64 / wall, out.display());
            Ok(())
        }
    }
}

/// One pass: load, lint, draw the sheet. Mistakes are printed, never fatal, so the loop survives a half-typed edit.
fn watch_pass(file: &std::path::Path, out: &std::path::Path, times: &[f64]) -> Result<()> {
    let plan = Plan::load(file)?;
    let findings = check::lint::run(&plan)?;
    for f in &findings {
        println!("{f}");
    }
    let times: Vec<f64> = if times.is_empty() { (0..8).map(|i| plan.root().dur * (f64::from(i) + 0.5) / 8.0).collect() } else { times.to_vec() };
    let mut renderer = render::Renderer::new(&plan)?;
    fs::write(out, renderer.sheet(&times, 4, 480)?).with_context(|| format!("writing {}", out.display()))?;
    println!("sheet -> {} ({} findings)", out.display(), findings.len());
    Ok(())
}

fn watch(file: &std::path::Path, out: &std::path::Path, times: &[f64], interval: u64, once: bool) -> Result<()> {
    let mut seen = None;
    loop {
        let stamp = fs::metadata(file).and_then(|m| m.modified()).with_context(|| format!("reading {}", file.display()))?;
        if seen != Some(stamp) {
            seen = Some(stamp);
            if let Err(e) = watch_pass(file, out, times) {
                println!("error: {e:#}");
                ensure!(!once, "the composition does not load");
            }
            if once {
                return Ok(());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(interval));
    }
}

fn fx(command: FxCommand) -> Result<()> {
    match command {
        FxCommand::List { dir } => {
            let lib = crate::fxlib::Library::load(&dir)?;
            let all: Vec<_> = lib.defs().map(crate::fxlib::Def::descriptor).collect();
            println!("{}", serde_json::to_string_pretty(&all)?);
            Ok(())
        }
        FxCommand::Test { name, dir, on, out } => {
            let lib = crate::fxlib::Library::load(&dir)?;
            let sweep = crate::fxlib::sweep::build(lib.get(&name)?, on.as_deref())?;
            let plan = Plan::from_value(sweep.comp, &dir)?;
            let mut renderer = render::Renderer::new(&plan)?;
            fs::write(&out, renderer.sheet(&sweep.times, crate::fxlib::sweep::STEPS as u32, 256)?).with_context(|| format!("writing {}", out.display()))?;
            for l in &sweep.legend {
                println!("{l}");
            }
            println!("sheet -> {} ({} columns sweep each parameter low to high)", out.display(), crate::fxlib::sweep::STEPS);
            Ok(())
        }
    }
}

/// RMSE of two BGRA buffers, 0..255. `golden_png` is decoded; `now_bgra` is the live frame.
fn png_rmse(now_bgra: &[u8], golden_png: &[u8], w: u32, h: u32) -> anyhow::Result<f64> {
    let img = skia_safe::Image::from_encoded(skia_safe::Data::new_copy(golden_png)).ok_or_else(|| anyhow::anyhow!("the golden file is not an image Skia can decode"))?;
    anyhow::ensure!(img.width() == w as i32 && img.height() == h as i32, "golden is {}x{}, the composition is {w}x{h}", img.width(), img.height());
    let info = skia_safe::ImageInfo::new(skia_safe::ISize::new(w as i32, h as i32), skia_safe::ColorType::BGRA8888, skia_safe::AlphaType::Premul, None);
    let mut px = vec![0u8; info.compute_min_byte_size()];
    let row = info.min_row_bytes();
    anyhow::ensure!(px.len() == now_bgra.len(), "golden pixel count does not match the live frame");
    anyhow::ensure!(img.read_pixels(&info, &mut px, row, (0, 0), skia_safe::image::CachingHint::Disallow), "could not read the golden's pixels");
    let mut acc = 0.0;
    for (a, b) in now_bgra.iter().zip(&px) {
        let d = f64::from(*a) - f64::from(*b);
        acc += d * d;
    }
    Ok((acc / now_bgra.len() as f64).sqrt())
}

/// A bitrate that scales with what is drawn: 0.07 bits per pixel per frame, kept between 3 and 40 Mbit/s.
pub fn auto_bitrate(width: u32, height: u32, fps: f64) -> String {
    let bits = f64::from(width) * f64::from(height) * fps * 0.07;
    format!("{}k", (bits / 1000.0).clamp(3000.0, 40000.0).round() as u64)
}

#[cfg(test)]
mod bitrate_tests {
    use super::auto_bitrate;

    #[test]
    fn the_default_bitrate_follows_pixels_and_frame_rate() {
        assert_eq!(auto_bitrate(1920, 1080, 30.0), "4355k");
        assert_eq!(auto_bitrate(1080, 1080, 60.0), "4899k");
        assert_eq!(auto_bitrate(320, 240, 24.0), "3000k");
        assert_eq!(auto_bitrate(7680, 4320, 60.0), "40000k");
    }
}
