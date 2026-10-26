//! One layer of a composition, from its JSON to a [`LayerPlan`]. Every independent mistake in the layer is collected, so
//! a layer with a bad key, a missing file and a bad effect is reported once with all three.

use std::path::Path;

use anyhow::{anyhow, Result};
use serde_json::Value;

use super::{layer_space, probe_media, LayerPlan, Media};
use crate::{
    fxlib::{Kind as FxKind, Library},
    schema::{self, EffectSpace, Kind, Problem, Problems},
};

/// The layer kinds that draw nothing: whatever is asked of their picture never happens.
fn draws_nothing(kind: &Kind) -> Option<&'static str> {
    match kind {
        Kind::Null => Some("a null layer (a group) only passes its position, scale and rotation to the layers parented to it"),
        Kind::Camera(_) => Some("a camera is not drawn"),
        Kind::Light(_) => Some("a light is not drawn"),
        _ => None,
    }
}

/// Plan layer `i` (`v`) of composition `c`.
pub(super) fn plan_layer(i: usize, v: &Value, c: &schema::Composition, root: &schema::Composition, dir: &Path, lib: &Library, kit: Option<(usize, u64)>, space: EffectSpace) -> Result<LayerPlan> {
    let def = schema::parse_layer(i, v)?;
    let ty = v.get("type").and_then(Value::as_str).unwrap_or("?");
    let label = format!("layer {i} ({ty}{})", def.common.id.as_deref().map_or(String::new(), |id| format!(" {id:?}")));
    let mut issues: Vec<String> = Vec::new();
    let mut note = |what: Result<()>| {
        if let Err(e) = what {
            issues.push(format!("{label}: {e:#}"));
        }
    };
    let start = def.common.start;
    let media = match probe_media(&def.kind, dir, kit) {
        Ok(m) => m,
        Err(e) => {
            note(Err(e));
            Media::None
        }
    };
    let natural = match (&media, &def.kind) {
        (Media::Video { info, .. }, Kind::Video(v)) if !v.looping => Some((info.dur - v.src_in).max(0.0) / v.speed.max(1e-6)),
        (Media::Audio { dur, rate, .. }, Kind::Audio(a)) => Some((dur - a.src_in).max(0.0) / (a.speed * rate).max(1e-6)),
        _ => None,
    };
    note(match def.common.dur {
        Some(d) if d < 0.0 => Err(anyhow!("dur must not be negative")),
        _ => Ok(()),
    });
    let end = match (def.common.dur, &def.kind) {
        (Some(d), _) => start + d,
        (None, _) if natural.is_some() => (start + natural.unwrap_or(0.0)).min(c.dur.max(start)),
        (None, Kind::Precomp(p)) => root.comps.get(&p.comp).map_or(c.dur, |n| (start + (n.dur - p.offset) / p.speed.max(1e-6)).min(c.dur.max(start))),
        (None, _) => c.dur,
    };
    let mut shapes = Vec::new();
    if let Kind::Shape(s) = &def.kind {
        for (k, it) in s.items.iter().enumerate() {
            match crate::plan::geometry::PathGeo::build(&it.path) {
                Ok(g) => shapes.push(g),
                Err(e) => note(Err(e.context(format!("item {k}")))),
            }
        }
    }
    if let Kind::Particles(p) = &def.kind {
        if let crate::schema::ParticleShape::Path(spec) = &p.shape {
            note(crate::plan::geometry::PathGeo::build(spec).map(|g| shapes.push(g)).map_err(|e| e.context("particle shape path")));
        }
    }
    let mut masks = Vec::new();
    for (k, m) in def.common.masks.iter().enumerate() {
        match crate::plan::geometry::PathGeo::build(&m.path) {
            Ok(g) => masks.push(g),
            Err(e) => note(Err(e.context(format!("mask {k}")))),
        }
    }
    if def.common.aim.is_some() && def.common.along.as_ref().is_some_and(|a| a.orient) {
        note(Err(anyhow!("\"aim\" and \"along.orient\" both decide where the layer faces; use one")));
    }
    let along = match def.common.along.as_ref().map(|a| crate::plan::geometry::PathGeo::build(&a.path)).transpose() {
        Ok(a) => a,
        Err(e) => {
            note(Err(e.context("along")));
            None
        }
    };
    if let Kind::Pack(pk) = &def.kind {
        note(crate::pack::check(&pk.name, &pk.params));
    }
    if let (Some(why), false) = (draws_nothing(&def.kind), def.fx.is_empty()) {
        note(Err(anyhow!("{ty} layers cannot carry \"fx\": {why}, so the effects would never apply. Put the effects on each child layer, on an adjustment layer above them, or build the group as a precomp and put them on that")));
    }
    let mut fx = Vec::new();
    for (k, f) in def.fx.iter().enumerate() {
        match lib.plan_fx(f) {
            Ok(planned) => fx.push(planned),
            Err(e) => note(Err(e.context(format!("fx {k}")))),
        }
    }
    let in_layer_space = layer_space::applies(space, &def.kind, &fx, masks.len());
    if in_layer_space {
        for problem in layer_space::problems(&def.kind, &fx, masks.len(), lib) {
            note(Err(anyhow!("{problem}")));
        }
    }
    let texture = match &def.kind {
        Kind::Texture(t) => match lib.plan(&t.shader, &t.params).and_then(|plan| lib.get(&t.shader).map(|d| (plan, d.kind))) {
            Ok((plan, FxKind::Generator)) => Some(plan),
            Ok((_, _)) => {
                note(Err(anyhow!("{:?} is an effect that needs an input layer; a texture layer takes a generator", t.shader)));
                None
            }
            Err(e) => {
                note(Err(e));
                None
            }
        },
        _ => None,
    };
    if !issues.is_empty() {
        return Err(Problems(issues.into_iter().map(|message| Problem { comp: String::new(), layer: Some(i), id: def.common.id.clone(), message }).collect()).into());
    }
    Ok(LayerPlan { index: i, label, def, start, end, parent: None, matte: None, is_matte_source: false, clip_to: None, shapes, masks, along, flip_from: None, in_3d: false, media, fx, in_layer_space, texture, text_path: None })
}

/// The problems an error from [`plan_layer`] stands for, tagged with the composition `comp`.
pub(super) fn problems_of(i: usize, v: &Value, comp: &str, error: anyhow::Error) -> Vec<Problem> {
    let mut list = match error.downcast::<Problems>() {
        Ok(p) => p.0,
        Err(other) => vec![Problem { comp: String::new(), layer: Some(i), id: v.get("id").and_then(Value::as_str).map(str::to_string), message: format!("layer {i}: {other:#}") }],
    };
    for p in &mut list {
        p.comp = comp.to_string();
    }
    list
}
