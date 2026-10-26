//! What a property does after its last key.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Loop {
    #[default]
    None,
    Cycle,
    PingPong,
    Offset,
}

impl Loop {
    /// `t` folded back into `first..=last`, and how many laps were made when the value should add up (`offset`).
    pub(super) fn map(self, t: f64, first: f64, last: f64) -> (f64, f64) {
        let span = last - first;
        if t <= last || span <= 0.0 {
            return (t, 0.0);
        }
        let n = ((t - first) / span).floor();
        match self {
            Loop::None => (t, 0.0),
            Loop::Cycle => (first + (t - first).rem_euclid(span), 0.0),
            Loop::Offset => (first + (t - first).rem_euclid(span), n),
            Loop::PingPong => {
                let m = (t - first).rem_euclid(2.0 * span);
                (first + if m > span { 2.0 * span - m } else { m }, 0.0)
            }
        }
    }
}
