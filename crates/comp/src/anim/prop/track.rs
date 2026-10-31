//! A track: the keys of one property of any type and the one function that finds its value at a time.

use anyhow::{ensure, Result};

use super::{
    key::{Key, Point, Side, Tangents},
    looping::Loop,
    shape::Shape,
    spatial::{stops, Path},
    value::{Kind, Value},
};

/// How a track's keys are read.
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub looping: Loop,
    /// The vector is a position: the keys are joined by a motion path and speed is along it.
    pub spatial: bool,
}

/// One segment between two keys: its timing, and the path it follows if the track is spatial.
#[derive(Debug, Clone)]
struct Segment {
    shape: Shape,
    path: Option<Path>,
}

#[derive(Debug, Clone)]
pub struct Track {
    kind: Kind,
    keys: Vec<Key>,
    looping: Loop,
    segments: Vec<Segment>,
}

impl Track {
    /// A track that holds one value for ever.
    pub fn constant(v: Value) -> Track {
        Track { kind: v.kind(), keys: vec![Key::new(0.0, v)], looping: Loop::None, segments: Vec::new() }
    }

    /// A track of `kind` through `keys`, which must be in time order.
    pub fn new(kind: Kind, keys: Vec<Key>, o: Options) -> Result<Track> {
        ensure!(!keys.is_empty(), "a property needs at least one key");
        if o.spatial {
            ensure!(matches!(kind, Kind::Vec2 | Kind::Vec3), "only a vector can follow a motion path, not {}", kind.name());
        }
        for (i, k) in keys.iter().enumerate() {
            ensure!(k.v.kind() == kind, "key {i} holds {}, the property holds {}", k.v.kind().name(), kind.name());
            if i > 0 {
                ensure!(k.t >= keys[i - 1].t, "key times must not go back, but key {i} is at {}s after key {} at {}s", k.t, i - 1, keys[i - 1].t);
            }
            check_sides(i, k, kind, o.spatial)?;
            ensure!(o.spatial || k.rove.is_empty(), "key {i} has roving keys, which only a motion path can have");
        }
        ensure!(keys[keys.len() - 1].rove.is_empty(), "the last key has roving keys after it; roving keys sit between two keys");
        let all: Vec<(Point, &Tangents)> = keys.iter().flat_map(|k| std::iter::once((k.v.point(), &k.tangents)).chain(k.rove.iter().map(|r| (r.at, &r.tangents)))).collect();
        let all = stops(&all);
        let mut next = 0;
        let mut segments = Vec::with_capacity(keys.len() - 1);
        for i in 0..keys.len() - 1 {
            let (from, to) = (next, next + 1 + keys[i].rove.len());
            next = to;
            let path = o.spatial.then(|| Path::through(&all[from..=to]));
            segments.push(Segment { shape: Shape::between(&keys, i, kind, o.spatial)?, path });
        }
        Ok(Track { kind, keys, looping: o.looping, segments })
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn keys(&self) -> &[Key] {
        &self.keys
    }

    pub fn looping(&self) -> Loop {
        self.looping
    }

    pub fn is_spatial(&self) -> bool {
        self.segments.first().is_some_and(|s| s.path.is_some())
    }

    /// The segment `t` falls in and how far across it, or none before the first key and after the last.
    fn locate(&self, t: f64) -> Option<(usize, f64)> {
        let i = self.keys.windows(2).position(|w| t < w[1].t)?;
        (t >= self.keys[0].t).then(|| (i, (t - self.keys[i].t) / (self.keys[i + 1].t - self.keys[i].t).max(1e-9)))
    }

    /// The value at time `t`.
    pub fn sample(&self, t: f64) -> Value {
        let (first, last) = (&self.keys[0], &self.keys[self.keys.len() - 1]);
        if self.keys.len() == 1 {
            return first.v.clone();
        }
        let (t, laps) = self.looping.map(t, first.t, last.t);
        let base = match self.locate(t) {
            Some((i, x)) => self.within(i, x),
            None if t < first.t => first.v.clone(),
            None => last.v.clone(),
        };
        if laps == 0.0 {
            return base;
        }
        let (b, f, l) = (base.parts(), first.v.parts(), last.v.parts());
        base.with_parts(std::array::from_fn(|d| b.v[d] + laps * (l.v[d] - f.v[d])))
    }

    fn within(&self, i: usize, x: f64) -> Value {
        let (a, b) = (&self.keys[i], &self.keys[i + 1]);
        let seg = &self.segments[i];
        if !self.kind.interpolates() || matches!(seg.shape, Shape::Hold) {
            return a.v.clone();
        }
        let dur = (b.t - a.t).max(1e-9);
        let from = a.v.parts();
        let mut out = [0.0; 4];
        match &seg.path {
            Some(path) => {
                let s = seg.shape.value(0, 0.0, path.length(), dur, x);
                out[..3].copy_from_slice(&path.point(s));
            }
            None => {
                let to = b.v.parts();
                for (d, o) in out.iter_mut().enumerate().take(from.n) {
                    *o = seg.shape.value(d, from.v[d], to.v[d], dur, x);
                }
            }
        }
        a.v.with_parts(out)
    }

    /// The direction a spatial track moves in at `t`, a unit vector; none for a track without a motion path.
    pub fn direction(&self, t: f64) -> Option<Point> {
        let path_of = |i: usize| self.segments[i].path.as_ref();
        let first = self.keys.first()?;
        if self.segments.is_empty() || path_of(0).is_none() {
            return None;
        }
        let t = self.looping.map(t, first.t, self.keys[self.keys.len() - 1].t).0;
        let (i, x) = self.locate(t).unwrap_or(if t < first.t { (0, 0.0) } else { (self.segments.len() - 1, 1.0) });
        let (seg, dur) = (&self.segments[i], (self.keys[i + 1].t - self.keys[i].t).max(1e-9));
        let path = path_of(i)?;
        let s = if matches!(seg.shape, Shape::Hold) { 0.0 } else { seg.shape.value(0, 0.0, path.length(), dur, x) };
        Some(path.direction(s))
    }
}

/// A key's sides must suit the property: handles carry one speed or one per dimension, with an influence of 0 to
/// 100 %, and a value that only changes at a key has none.
fn check_sides(i: usize, k: &Key, kind: Kind, spatial: bool) -> Result<()> {
    let dims = if spatial { 1 } else { kind.dims() };
    if !kind.interpolates() {
        ensure!(k.ease.is_none() && k.ease_in.is_none() && k.ease_out.is_none(), "key {i}: {} only changes at a key, so it has no ease", kind.name());
    }
    for (name, side) in [("ease_in", &k.ease_in), ("ease_out", &k.ease_out)] {
        let Some(Side::Bezier(h)) = side else { continue };
        ensure!((0.0..=1.0).contains(&h.influence), "key {i}: {name} influence {} is outside 0..100 %", h.influence * 100.0);
        ensure!(h.speed.len() == 1 || h.speed.len() == dims, "key {i}: {name} has {} speeds, but this property takes {dims}{}", h.speed.len(), if spatial { " (speed along the path)" } else { "" });
    }
    Ok(())
}
