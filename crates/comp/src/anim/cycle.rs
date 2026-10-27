//! Cycling a palette over a row of copies: which colour copy `i` takes. This is arithmetic only; the schema says what
//! can be written (`schema::palette`) and the evaluator turns the choice into a colour (`eval::palette`).
//!
//! Copy `i` falls at `place = i + offset` colours along the palette, so an `offset` of 1 moves every copy on by one
//! colour and a fractional offset moves it part of the way. What the three modes do with that place:
//!
//! | mode | colour of the copy |
//! | --- | --- |
//! | `step` | colour `floor(place) mod n`: the palette repeats, and a fractional offset holds the colour until it reaches the next whole number |
//! | `ping_pong` | the same index run forwards then backwards, `0 1 .. n-1 n-2 .. 1 0 1 ..`: the end colours are not doubled, so the period is `2 (n - 1)` copies |
//! | `gradient` | a blend of the two colours either side of `place mod n`, the last colour blending back into the first |
//!
//! `reverse` reads the palette from its last colour to its first before any of this.

use std::num::NonZeroUsize;

use serde::{Deserialize, Serialize, Serializer};

/// A list that cannot be empty, so choosing "colour `i mod n`" needs no case for an empty palette.
#[derive(Debug, Clone, PartialEq)]
pub struct NonEmpty<T> {
    head: T,
    tail: Vec<T>,
}

impl<T> NonEmpty<T> {
    /// `None` for an empty list.
    pub fn new(items: Vec<T>) -> Option<Self> {
        let mut items = items.into_iter();
        let head = items.next()?;
        Some(NonEmpty { head, tail: items.collect() })
    }

    pub fn len(&self) -> NonZeroUsize {
        NonZeroUsize::MIN.saturating_add(self.tail.len())
    }

    /// Item `i mod len`.
    pub fn get(&self, i: usize) -> &T {
        match i % self.len() {
            0 => &self.head,
            k => &self.tail[k - 1],
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.head).chain(&self.tail)
    }

    /// Map every item, given its place in the list, stopping at the first error.
    pub fn try_map<U, E>(&self, mut f: impl FnMut(usize, &T) -> Result<U, E>) -> Result<NonEmpty<U>, E> {
        Ok(NonEmpty { head: f(0, &self.head)?, tail: self.tail.iter().enumerate().map(|(i, item)| f(i + 1, item)).collect::<Result<_, E>>()? })
    }

    /// The same items, last first.
    pub fn reversed(self) -> Self {
        let NonEmpty { head, mut tail } = self;
        match tail.pop() {
            None => NonEmpty { head, tail },
            Some(last) => {
                tail.insert(0, head);
                tail.reverse();
                NonEmpty { head: last, tail }
            }
        }
    }
}

impl<T: Serialize> Serialize for NonEmpty<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.iter())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CycleMode {
    /// One colour per copy, the palette repeating.
    #[default]
    Step,
    /// A blend of neighbouring colours, the palette repeating.
    Gradient,
    /// The palette forwards then backwards.
    PingPong,
}

/// What a copy takes from the palette.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Slot {
    /// Colour `i` of the palette as it is.
    Pick(usize),
    /// `by` (0 to 1) of the way from colour `from` to colour `to`.
    Blend { from: usize, to: usize, by: f64 },
}

/// The slot of copy `copy` in a palette of `len` colours shifted on by `offset` colours (`offset` must be finite).
pub fn slot(mode: CycleMode, len: NonZeroUsize, copy: usize, offset: f64) -> Slot {
    let n = len.get();
    let place = copy as f64 + offset;
    match mode {
        CycleMode::Step => Slot::Pick(whole(place, n)),
        CycleMode::PingPong => {
            let period = (2 * (n - 1)).max(1);
            let i = whole(place, period);
            Slot::Pick(if i < n { i } else { period - i })
        }
        CycleMode::Gradient => {
            let u = place.rem_euclid(n as f64);
            let k = u.floor();
            Slot::Blend { from: k as usize % n, to: (k as usize + 1) % n, by: u - k }
        }
    }
}

/// `floor(place) mod period`. `rem_euclid` of a tiny negative number can round up to `period` itself, hence the `%`.
fn whole(place: f64, period: usize) -> usize {
    place.floor().rem_euclid(period as f64) as usize % period
}

#[cfg(test)]
mod tests {
    use super::*;

    fn len(n: usize) -> NonZeroUsize {
        NonZeroUsize::new(n).expect("a test palette has a colour")
    }

    fn picks(mode: CycleMode, n: usize, copies: usize, offset: f64) -> Vec<usize> {
        (0..copies)
            .map(|i| match slot(mode, len(n), i, offset) {
                Slot::Pick(k) => k,
                other => panic!("{other:?}"),
            })
            .collect()
    }

    #[test]
    fn step_repeats_the_palette_and_a_whole_offset_moves_every_copy_on() {
        assert_eq!(picks(CycleMode::Step, 3, 7, 0.0), [0, 1, 2, 0, 1, 2, 0]);
        assert_eq!(picks(CycleMode::Step, 3, 7, 1.0), [1, 2, 0, 1, 2, 0, 1]);
        assert_eq!(picks(CycleMode::Step, 3, 5, -1.0), [2, 0, 1, 2, 0]);
        assert_eq!(picks(CycleMode::Step, 4, 4, -9.0), [3, 0, 1, 2]);
        assert_eq!(picks(CycleMode::Step, 1, 3, 5.0), [0, 0, 0]);
    }

    #[test]
    fn a_fractional_offset_holds_a_colour_until_the_offset_reaches_the_next_whole_number() {
        assert_eq!(picks(CycleMode::Step, 3, 4, 0.99), picks(CycleMode::Step, 3, 4, 0.0));
        assert_eq!(picks(CycleMode::Step, 3, 4, 1.0), picks(CycleMode::Step, 3, 4, 1.5));
        // below zero it is the floor too: -0.25 is "just short of 0", which is the colour before
        assert_eq!(picks(CycleMode::Step, 3, 4, -0.25), picks(CycleMode::Step, 3, 4, -1.0));
        assert_eq!(picks(CycleMode::Step, 3, 4, -1.0), [2, 0, 1, 2]);
    }

    #[test]
    fn ping_pong_runs_forwards_and_back_without_doubling_the_ends() {
        assert_eq!(picks(CycleMode::PingPong, 4, 10, 0.0), [0, 1, 2, 3, 2, 1, 0, 1, 2, 3]);
        assert_eq!(picks(CycleMode::PingPong, 3, 9, 0.0), [0, 1, 2, 1, 0, 1, 2, 1, 0]);
        assert_eq!(picks(CycleMode::PingPong, 2, 5, 0.0), [0, 1, 0, 1, 0]);
        assert_eq!(picks(CycleMode::PingPong, 1, 3, 0.0), [0, 0, 0]);
        assert_eq!(picks(CycleMode::PingPong, 3, 5, -1.0), [1, 0, 1, 2, 1]);
    }

    #[test]
    fn a_gradient_blends_the_colours_either_side_and_wraps_the_last_into_the_first() {
        let at = |copy, offset| slot(CycleMode::Gradient, len(3), copy, offset);
        assert_eq!(at(1, 0.0), Slot::Blend { from: 1, to: 2, by: 0.0 });
        assert_eq!(at(1, 0.25), Slot::Blend { from: 1, to: 2, by: 0.25 });
        assert_eq!(at(2, 0.5), Slot::Blend { from: 2, to: 0, by: 0.5 });
        assert_eq!(at(0, -0.5), Slot::Blend { from: 2, to: 0, by: 0.5 });
        let Slot::Blend { from: 0, to: 0, by } = slot(CycleMode::Gradient, len(1), 4, 0.3) else { panic!("one colour blends into itself") };
        assert!((by - 0.3).abs() < 1e-12);
    }

    #[test]
    fn a_non_empty_list_is_read_round_and_reversed_without_an_empty_case() {
        assert!(NonEmpty::<u8>::new(vec![]).is_none());
        let list = NonEmpty::new(vec!['a', 'b', 'c']).expect("three items");
        assert_eq!([list.get(0), list.get(4), list.get(5)], [&'a', &'b', &'c']);
        assert_eq!(list.clone().reversed().iter().collect::<String>(), "cba");
        assert_eq!(NonEmpty::new(vec![1]).expect("one item").reversed().iter().count(), 1);
        assert_eq!(list.try_map(|_, c| if *c == 'z' { Err(()) } else { Ok(*c as u32) }).map(|m| m.iter().sum::<u32>()), Ok(97 + 98 + 99));
    }
}
