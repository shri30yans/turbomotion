//! The frame graph: how one composition frame becomes pixels.
//!
//! A frame is a set of passes over named images (resources). A pass reads some images and writes one, so the order
//! is not hard-coded anywhere: [`build`] walks back from the picture that is wanted, emits each pass after the passes it
//! reads from (so emission order is a topological order and a pass nobody needs is never emitted), and [`run`]
//! executes them, dropping every image after its last reader.
//!
//! The composite is the one image that is *modified* rather than replaced (copying a full-frame half-float image per
//! layer would cost more than the layers): its passes write it in place, in the order the layers were emitted, and
//! every pass that only reads it takes a snapshot first.
//!
//! This follows the frame-graph design of Frostbite's FrameGraph and Unity's Render Graph: passes are nodes, images are
//! virtual resources, edges are reads.

mod build;
mod describe;
mod run;
#[cfg(test)]
mod tests;

pub use build::{comp_graph, effect_inputs_graph, layer_graph};
pub use describe::describe;
pub use run::{comp_picture, effect_inputs, layer_picture};

use turbomotion_core::effects::Blend;

use crate::{
    eval::{transit::Clip, Frame, ReflectState},
    plan::CompPlan,
    schema::MatteMode,
};

/// A virtual image: written by one pass, read by any number of later ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageId(usize);

/// Whose effect list a chain of effects is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FxOwner {
    Layer(usize),
    Adjustment(usize),
    /// The composition's `look`.
    Look,
}

/// What a pass does. The images it reads are in `Pass::reads`, in the order given here.
#[derive(Clone)]
pub enum Stage {
    /// Clear the composite to the composition's background colour (or to nothing).
    Backdrop,
    /// A layer's own picture: content, lighting, the blur or smear of a transition. Reads nothing.
    Paint { layer: usize },
    /// Effects `0..upto` of an effect list. Reads the picture they start from, then the pictures of the layers the
    /// effects read as inputs (`pictures`, in the same order).
    Effects { owner: FxOwner, upto: usize, pictures: Vec<usize> },
    /// A layer's masks as coverage. Reads nothing.
    MaskCoverage { layer: usize },
    /// A track matte's or clip's coverage from the picture of its source layer; the source's opacity is applied here.
    MatteCoverage { mode: MatteMode, opacity: f64 },
    /// Keep a picture where a coverage has alpha. Reads the picture, then the coverage.
    Multiply,
    /// The layer's soft floor reflection.
    Reflect { bounds: [f64; 4], reflect: ReflectState },
    /// A transition's wipe or iris.
    Wipe { bounds: [f64; 4], clip: Clip },
    /// Average the layer over the open shutter: reads one picture per sample, each with its opacity.
    ShutterAverage { opacities: Vec<f64> },
    /// Lay a picture on the composite with a blend mode and opacity. Modifies the composite.
    Composite { blend: Option<Blend>, opacity: f64 },
    /// What has been composited so far, as a picture.
    Snapshot,
    /// An adjustment layer's coverage: its masks (or everywhere) times its opacity.
    AdjustCoverage { layer: usize },
    /// Blend the changed picture over what was below, through the coverage. Reads below, changed, coverage; modifies the
    /// composite.
    AdjustMix,
}

/// Where a pass puts its result.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Write {
    /// A new image.
    New(ImageId),
    /// The composite, changed in place.
    Composite,
}

#[derive(Clone)]
pub struct Pass {
    pub stage: Stage,
    /// Which of the graph's frames (the frame itself, or a shutter sample) the pass draws from.
    pub frame: usize,
    pub reads: Vec<ImageId>,
    pub write: Write,
}

/// Every pass for one picture, in an order that satisfies every read.
pub struct Graph<'a> {
    pub comp: &'a CompPlan,
    /// The frame at the moment drawn, then the frames at the shutter samples.
    pub frames: Vec<&'a Frame>,
    pub passes: Vec<Pass>,
    /// How many images the passes write.
    pub images: usize,
    /// The images the graph produces: one for a picture, several for the inputs of an effect.
    pub outputs: Vec<ImageId>,
}

impl Graph<'_> {
    /// How many passes read each image, counting each output as read once more.
    fn readers(&self) -> Vec<usize> {
        let mut n = vec![0; self.images];
        for r in self.passes.iter().flat_map(|p| &p.reads) {
            n[r.0] += 1;
        }
        for o in &self.outputs {
            n[o.0] += 1;
        }
        n
    }

    /// The most images alive at once if each is dropped after its last reader (the composite not counted).
    pub fn peak_images(&self) -> usize {
        let mut left = self.readers();
        let (mut live, mut peak) = (0usize, 0usize);
        for p in &self.passes {
            if matches!(p.write, Write::New(_)) {
                live += 1;
            }
            peak = peak.max(live);
            for r in &p.reads {
                left[r.0] -= 1;
                if left[r.0] == 0 {
                    live -= 1;
                }
            }
        }
        peak
    }
}
