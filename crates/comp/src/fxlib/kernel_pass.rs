//! A pass that is a kernel blur instead of code: the plugin names the blur and its radius, and the engine runs it.
//!
//! ```text
//! // @pass soft kernel=gaussian radius=blur/2                  a Gaussian of sigma blur/2 over the previous picture
//! // @pass wide kernel=box radius=size iterations=3 from=image  three boxes over `image`
//! ```
//!
//! `radius=` is an expression of the float parameters (see [`Expr`]); a choice is its option's number, so
//! `radius=quality*blur/2` is no blur at all while `quality` is its first option, which is how a plugin keeps an old
//! look under its first option and turns the exact blur on under another. `from=` names the picture blurred: `image` or
//! an earlier pass (default: the previous pass, or `image` for the first). `edge=repeat` continues the edge pixels
//! outward. A kernel pass has no code and runs at the layer's size. Later passes read its result by its name.

use anyhow::{anyhow, bail, ensure, Result};
use turbomotion_core::{
    kernel::{Blur, Edge, Shape},
    shader::Program,
    suggest::hint,
};

use super::{bounds::Expr, Param, Pass, Ty};

/// The picture every pass can read that is not an earlier pass.
pub const IMAGE: &str = "image";

/// What a pass does: runs the author's shader, or a kernel blur the header names.
pub enum PassBody {
    Shader(Program),
    Kernel(KernelPass),
}

impl Pass {
    /// The shader of a pass that is one (a generator's only pass always is).
    pub fn program(&self) -> Option<&Program> {
        match &self.body {
            PassBody::Shader(p) => Some(p),
            PassBody::Kernel(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KernelPass {
    box_iterations: Option<u32>,
    radius: Expr,
    /// The picture blurred: `image` or an earlier pass.
    pub from: String,
    edge: Edge,
    /// The declaration as written after the pass name, for the reference pages.
    pub text: String,
}

/// The names an expression may use: the float parameters, and the choices as the number of the option.
pub fn expression_names(params: &[Param]) -> Vec<&str> {
    params.iter().filter(|p| matches!(p.ty, Ty::Float | Ty::Choice)).map(|p| p.name.as_str()).collect()
}

impl KernelPass {
    /// The pass declared by `options` (the words after its name that are not `scale=`), given the plugin's parameters and
    /// the passes before this one.
    pub fn parse(options: &[String], params: &[Param], earlier: &[&str]) -> Result<Self> {
        let (mut kernel, mut radius, mut from, mut edge, mut iterations) = (None, None, None, None, None);
        for word in options {
            let (key, value) = word.split_once('=').ok_or_else(|| anyhow!("{word:?} is not key=value; a pass takes scale=, kernel=, radius=, from=, edge= or iterations="))?;
            let slot = match key {
                "kernel" => &mut kernel,
                "radius" => &mut radius,
                "from" => &mut from,
                "edge" => &mut edge,
                "iterations" => &mut iterations,
                other => bail!("unknown pass option {other:?}; the options are scale=, kernel=, radius=, from=, edge= and iterations="),
            };
            ensure!(slot.is_none(), "{key}= is given twice");
            *slot = Some(value);
        }
        let kernel = kernel.ok_or_else(|| anyhow!("radius=, from=, edge= and iterations= belong to a kernel pass: add kernel=gaussian or kernel=box"))?;
        let radius = Expr::parse(radius.ok_or_else(|| anyhow!("a kernel pass needs radius=<expression>"))?, &expression_names(params))?;
        let from = from.unwrap_or_else(|| earlier.last().copied().unwrap_or(IMAGE));
        ensure!(from == IMAGE || earlier.contains(&from), "from={from} is neither `image` nor an earlier pass{}", hint(from, &earlier.iter().copied().chain([IMAGE]).collect::<Vec<_>>()));
        let edge = match edge.unwrap_or("transparent") {
            "transparent" => Edge::Transparent,
            "repeat" => Edge::Repeat,
            other => bail!("edge={other}: the edges are {}", Edge::NAMES.join(" or ")),
        };
        let box_iterations = match (kernel, iterations) {
            ("gaussian", None) => None,
            ("gaussian", Some(_)) => bail!("iterations= repeats a box; a Gaussian is one pass"),
            ("box", n) => {
                let n = n.unwrap_or("1");
                Some(n.parse::<u32>().map_err(|_| anyhow!("iterations= takes a whole number, not {n:?}"))?)
            }
            (other, _) => bail!("kernel={other}: the kernels are gaussian and box"),
        };
        Ok(Self { box_iterations, radius, from: from.to_string(), edge, text: options.join(" ") })
    }

    /// The blur at the parameter values `value` gives (a parameter's current value, by name).
    pub fn blur(&self, value: &dyn Fn(&str) -> Option<f64>) -> Result<Blur> {
        let radius = self.radius.eval(value)?.max(0.0);
        let shape = match self.box_iterations {
            None => Shape::Gaussian { sigma: [radius; 2] },
            Some(iterations) => Shape::Box { radius: [radius; 2], iterations },
        };
        Ok(Blur { shape, edge: self.edge })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> Vec<Param> {
        let float = |name: &str| Param { name: name.to_string(), ty: Ty::Float, default: vec![1.0], optional: false, min: None, max: None, options: vec![], doc: String::new() };
        let choice = Param { name: "quality".to_string(), ty: Ty::Choice, default: vec![0.0], optional: false, min: None, max: None, options: vec!["legacy".into(), "exact".into()], doc: String::new() };
        vec![float("blur"), choice]
    }

    fn parse(words: &str, earlier: &[&str]) -> Result<KernelPass> {
        KernelPass::parse(&words.split_whitespace().map(str::to_string).collect::<Vec<_>>(), &params(), earlier)
    }

    fn values(name: &str) -> Option<f64> {
        match name {
            "blur" => Some(20.0),
            "quality" => Some(1.0),
            _ => None,
        }
    }

    #[test]
    fn a_gaussian_pass_is_its_radius_expression_read_at_the_parameters_and_a_choice_counts_as_its_number() {
        let pass = parse("kernel=gaussian radius=quality*blur/2", &[]).expect("a pass");
        assert_eq!(pass.blur(&values).expect("a blur"), Blur { shape: Shape::Gaussian { sigma: [10.0; 2] }, edge: Edge::Transparent });
        assert_eq!(pass.from, "image", "the first pass reads the picture");
        assert_eq!(parse("kernel=gaussian radius=blur", &["a", "b"]).expect("a pass").from, "b", "and any other the one before it");
        let off = |name: &str| if name == "quality" { Some(0.0) } else { values(name) };
        assert_eq!(pass.blur(&off).expect("a blur"), Blur { shape: Shape::Gaussian { sigma: [0.0; 2] }, edge: Edge::Transparent }, "the first option is no blur");
    }

    #[test]
    fn a_box_pass_repeats_and_takes_an_edge_and_a_source() {
        let pass = parse("kernel=box radius=blur iterations=3 from=a edge=repeat", &["a", "b"]).expect("a pass");
        assert_eq!(pass.from, "a");
        assert_eq!(pass.blur(&values).expect("a blur"), Blur { shape: Shape::Box { radius: [20.0; 2], iterations: 3 }, edge: Edge::Repeat });
    }

    #[test]
    fn what_a_kernel_pass_cannot_say_is_an_error_that_says_what_it_can() {
        for (words, want) in [
            ("kernel=gaussian", "needs radius="),
            ("radius=blur", "add kernel=gaussian or kernel=box"),
            ("kernel=gaussain radius=blur", "the kernels are gaussian and box"),
            ("kernel=gaussian radius=blurr", "blurr"),
            ("kernel=gaussian radius=blur iterations=2", "a Gaussian is one pass"),
            ("kernel=box radius=blur iterations=two", "whole number"),
            ("kernel=gaussian radius=blur from=nowhere", "neither `image` nor an earlier pass"),
            ("kernel=gaussian radius=blur edge=wrap", "transparent or repeat"),
            ("kernel=gaussian radius=blur speed=2", "unknown pass option"),
            ("kernel=gaussian kernel=box radius=blur", "given twice"),
        ] {
            let e = format!("{:#}", parse(words, &["a"]).expect_err(words));
            assert!(e.contains(want), "{words}: {e}");
        }
    }

    const SOFT: &str = "// @effect soft\n// @summary x\n// @param sigma float 4 0 40 -- sigma\n// @param quality choice legacy legacy,exact -- which\n// @bounds expand=sigma*(1+3*quality)\n// @pass halo kernel=gaussian radius=quality*sigma\nhalf4 pad;\n// @pass main\nhalf4 main(float2 xy) { return halo.eval(xy) * float(sigma + resolution.x + time); }\n";

    #[test]
    fn a_plugin_with_a_kernel_pass_compiles_and_its_descriptor_says_which_pass_is_the_kernel() {
        let src = SOFT.replace("half4 pad;\n", "");
        let d = super::super::Def::compile(&src, "t").expect("compiles").descriptor();
        assert_eq!(d.passes.iter().map(|p| (p.name.as_str(), p.kernel.as_str())).collect::<Vec<_>>(), [("halo", "kernel=gaussian radius=quality*sigma"), ("main", "")]);
        assert_eq!(d.bounds, "expand=sigma*(1+3*quality)", "a bound reads a choice as the number of its option");
    }

    #[test]
    fn a_kernel_pass_has_no_code_runs_at_full_size_and_is_not_for_a_generator() {
        let err = |src: &str| format!("{:#}", super::super::Def::compile(src, "t").err().expect("rejected"));
        assert!(err(SOFT).contains("kernel pass and has no code, but line 7 is: half4 pad;"), "{}", err(SOFT));
        let scaled = SOFT.replace("half4 pad;\n", "").replace("radius=quality*sigma", "radius=quality*sigma scale=0.5");
        assert!(err(&scaled).contains("runs at the layer's size"), "{}", err(&scaled));
        let generator = "// @effect g\n// @kind generator\n// @param r float 1 0 2 -- r\n// @pass a kernel=gaussian radius=r\nhalf4 main(float2 xy) { return half4(r + time + resolution.x); }\n";
        assert!(err(generator).contains("a generator has no picture to blur"), "{}", err(generator));
        let wrong_name = SOFT.replace("half4 pad;\n", "").replace("radius=quality*sigma", "radius=quality*sigmaa");
        assert!(err(&wrong_name).contains("@pass halo"), "the error names the pass: {}", err(&wrong_name));
    }
}
