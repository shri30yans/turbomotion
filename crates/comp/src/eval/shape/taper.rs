//! A stroke taper at one instant: lengths and widths as numbers.

use anyhow::{ensure, Result};

use crate::{
    anim::ease::Ease,
    eval::{salt, Evaluator, LayerEnv},
    plan::{CompPlan, LayerPlan},
    schema::taper::{Extent, Taper as Spec, Tip as TipSpec},
    shape::stroke::{Dim, Taper, Tip},
};

/// `spec` at time `t` for item `k` of layer `l`.
pub fn state(ev: &Evaluator, c: &CompPlan, l: &LayerPlan, spec: &Spec, k: usize, t: f64) -> Result<Taper> {
    let owner = salt::named(salt::SHAPE_TAPER, &k.to_string());
    let tip = |end: &str, tip: &Option<TipSpec>| -> Result<Option<Tip>> {
        let dim = |what: &str, extent: &Extent| -> Result<Dim> {
            let property = salt::named(salt::named(owner, end), what);
            let value = extent.property().0.at(t - l.start, &LayerEnv::new(ev, c, l.index, t, 0).with_salt(property))?;
            ensure!(value.is_finite() && value >= 0.0, "taper {end} {what} is {value} at this time: it needs to be 0 or more");
            Ok(Dim { value, fraction: extent.is_fraction() })
        };
        tip.as_ref().map(|s| Ok(Tip { length: dim("length", &s.length)?, width: s.width.as_ref().map_or(Ok(Dim { value: 0.0, fraction: false }), |w| dim("width", w))? })).transpose()
    };
    Ok(Taper { start: tip("start", &spec.start)?, end: tip("end", &spec.end)?, ease: spec.ease.map_or(Ease::Linear, |e| e.0) })
}
