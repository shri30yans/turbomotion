//! Run a film script once on QuickJS and return its composition JSON.
//!
//! Deterministic by construction: `Math.random` is a seeded generator, `Date` and `performance` are frozen, there are
//! no timers, and the script is run exactly once. The same script and options always give the same bytes.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context as _, Result};
use rquickjs::{CatchResultExt, Context, Ctx, Function, Module, Runtime, Value};
use serde_json::Value as Json;

use crate::{
    loader::{display, sdk_directory, Load, Registry, Resolve, Shared},
    strip::strip,
};

/// What the host tells the script.
#[derive(Clone, Debug)]
pub struct Options {
    /// Override the SDK source directory (must contain index.ts).
    pub sdk_dir: Option<PathBuf>,
    pub seed: u32,
    pub time_limit: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Options { sdk_dir: None, seed: 0x7fa3_c0de, time_limit: Duration::from_secs(20) }
    }
}

const PRELUDE: &str = r#"
(function () {
  var s = __SEED__ >>> 0;
  Math.random = function () {
    s = (s + 0x6D2B79F5) | 0;
    var t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  var Real = Date;
  globalThis.Date = class extends Real {
    constructor(...a) { if (a.length) super(...a); else super(0); }
    static now() { return 0; }
  };
  globalThis.performance = { now: function () { return 0; } };
  var out = globalThis.__print;
  delete globalThis.__print;
  var show = function (a) { return typeof a === "string" ? a : (typeof a === "object" && a !== null ? JSON.stringify(a) : String(a)); };
  var log = function () { out(Array.prototype.map.call(arguments, show).join(" ")); };
  globalThis.console = { log: log, info: log, warn: log, error: log, debug: log };
})();
"#;

/// A built film plus what the run touched: every file it read, and (when asked) where each layer was created.
pub struct Built {
    pub film: Json,
    /// Source files that, when edited, change the film: the script, its imports, `svgFile` reads and the chosen SDK tree.
    pub files: Vec<PathBuf>,
    /// Layer id -> `film.ts:LINE:COL` of the author's line that made it (only with `track_sites`).
    pub sites: BTreeMap<String, String>,
}

/// Run `file` and return the composition it produces.
pub fn build(file: &Path, options: &Options) -> Result<Json> {
    Ok(build_full(file, options, false)?.film)
}

/// `build`, also reporting the files the run read and, with `track_sites`, the line that created every layer.
pub fn build_full(file: &Path, options: &Options, track_sites: bool) -> Result<Built> {
    let source = std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let name = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf()).to_string_lossy().into_owned();
    let shared: Shared = Arc::new(Mutex::new(Registry::default()));
    let entry = strip(&name, &source)?;
    let js = entry.js.clone();
    shared.lock().unwrap().modules.insert(name.clone(), entry);

    let rt = Runtime::new()?;
    rt.set_loader(Resolve, Load { registry: shared.clone(), sdk_dir: sdk_directory(options.sdk_dir.as_deref())? });
    rt.set_memory_limit(1 << 30);
    let started = Instant::now();
    let limit = options.time_limit;
    rt.set_interrupt_handler(Some(Box::new(move || started.elapsed() > limit)));
    let ctx = Context::full(&rt)?;
    let reads: crate::assets::Reads = Default::default();
    let (text, raw_sites) = ctx
        .with(|ctx| -> std::result::Result<(String, String), String> {
            run_module(&ctx, &name, js, options, &reads, track_sites).map_err(|e| report(&ctx, e, &shared, options))
        })
        .map_err(|m| anyhow!(m))?;
    let mut json: Json = serde_json::from_str(&text).context("the film is not valid JSON")?;
    if let Some(object) = json.as_object_mut() {
        let film_dir = std::fs::canonicalize(file)?.parent().unwrap().to_path_buf();
        let root = match object.get("root") {
            Some(Json::String(root)) => film_dir.join(root),
            Some(_) => return Err(anyhow!("film root must be a directory path string")),
            None => film_dir,
        };
        object.insert("root".into(), Json::String(root.to_string_lossy().into_owned()));
    }
    let registry = shared.lock().unwrap();
    let mut files: Vec<PathBuf> = registry.modules.keys().filter_map(|m| match m.strip_prefix("sdk:") {
        Some(sdk) => options.sdk_dir.as_ref().map(|d| d.join(sdk).with_extension("ts")),
        None => Some(PathBuf::from(m)),
    }).collect();
    files.extend(reads.lock().unwrap().iter().cloned());
    files.sort();
    files.dedup();
    let sites: BTreeMap<String, String> = serde_json::from_str::<BTreeMap<String, String>>(&raw_sites).unwrap_or_default().into_iter()
        .map(|(id, site)| (id, clean_site(&map_line(&site, &registry)))).collect();
    Ok(Built { film: json, files, sites })
}

/// `fn (film.ts:12:5)` or `film.ts:12:5` -> `film.ts:12:5`.
fn clean_site(site: &str) -> String {
    let inner = site.rsplit_once('(').map_or(site, |(_, rest)| rest.trim_end_matches(')'));
    inner.to_string()
}

fn run_module<'js>(ctx: &Ctx<'js>, name: &str, js: String, options: &Options, reads: &crate::assets::Reads, track_sites: bool) -> std::result::Result<(String, String), rquickjs::CaughtError<'js>> {
    crate::animation::install(ctx).catch(ctx)?;
    crate::vocabulary::install(ctx).catch(ctx)?;
    crate::measure::install(ctx, Path::new(name).parent().unwrap()).catch(ctx)?;
    crate::assets::install(ctx, Path::new(name).parent().unwrap(), reads.clone()).catch(ctx)?;
    if track_sites { ctx.eval::<(), _>("globalThis.__tm_sites = {};").catch(ctx)?; }
    let print = Function::new(ctx.clone(), |line: String| eprintln!("{line}")).catch(ctx)?;
    ctx.globals().set("__print", print).catch(ctx)?;
    ctx.eval::<(), _>(PRELUDE.replace("__SEED__", &options.seed.to_string())).catch(ctx)?;

    let (module, promise) = Module::declare(ctx.clone(), name, js).catch(ctx)?.eval().catch(ctx)?;
    promise.finish::<()>().catch(ctx)?;
    let mut film: Value = module.get("default").catch(ctx)?;
    if let Some(f) = film.as_function() {
        film = f.call(()).catch(ctx)?;
    }
    if let Some(p) = film.as_promise() {
        film = p.finish().catch(ctx)?;
    }
    if film.is_undefined() {
        return Err(rquickjs::CaughtError::Error(rquickjs::Error::new_from_js_message("script", "film", "the script has no default export: end it with `export default film;`")));
    }
    let text = ctx.json_stringify(film).catch(ctx)?;
    match text {
        Some(t) => Ok((t.to_string().catch(ctx)?, ctx.eval::<String, _>("JSON.stringify(globalThis.__tm_sites || {})").catch(ctx)?)),
        None => Err(rquickjs::CaughtError::Error(rquickjs::Error::new_from_js_message("script", "film", "the default export cannot be turned into JSON"))),
    }
}

/// An error as the author should read it: message, then the stack with generated positions mapped to their TypeScript lines.
fn report(_ctx: &Ctx<'_>, e: rquickjs::CaughtError<'_>, shared: &Shared, options: &Options) -> String {
    let reg = shared.lock().unwrap();
    if let Some(f) = &reg.failure {
        return f.clone();
    }
    let raw = match e {
        rquickjs::CaughtError::Exception(ex) => {
            let mut s = ex.message().unwrap_or_default();
            if let Some(stack) = ex.stack() {
                s.push('\n');
                s.push_str(&stack);
            }
            s
        }
        rquickjs::CaughtError::Value(v) => format!("{v:?}"),
        rquickjs::CaughtError::Error(e) => e.to_string(),
    };
    let mapped = raw.lines().map(|l| map_line(l, &reg)).collect::<Vec<_>>().join("\n");
    if mapped.contains("interrupted") {
        return format!("the script ran longer than {}s (an endless loop?)\n{mapped}", options.time_limit.as_secs());
    }
    mapped
}

/// Replace `<module>:<line>[:<col>]` in a stack line with the original file, line and column.
fn map_line(line: &str, reg: &Registry) -> String {
    for (name, stripped) in &reg.modules {
        let Some(at) = line.find(&format!("{name}:")) else { continue };
        let rest = &line[at + name.len() + 1..];
        let digits = |s: &str| s.chars().take_while(char::is_ascii_digit).collect::<String>();
        let l = digits(rest);
        let Ok(gl) = l.parse::<u32>() else { continue };
        let after = &rest[l.len()..];
        let c = after.strip_prefix(':').map(digits).unwrap_or_default();
        let gc = c.parse::<u32>().unwrap_or(1);
        let consumed = l.len() + if c.is_empty() { 0 } else { 1 + c.len() };
        let (ol, oc) = stripped.original(gl, gc).unwrap_or((gl, gc));
        let tail = &rest[consumed..];
        return format!("{}{}:{ol}:{oc}{tail}", &line[..at], display(name));
    }
    line.to_string()
}
