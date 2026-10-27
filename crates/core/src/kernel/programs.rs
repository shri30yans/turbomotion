//! The kernel passes' SkSL programs. Each loop bound in them is written from the constant the planner checks against,
//! so the two cannot drift apart.

use anyhow::Result;

use super::taps::{MAX_IRIS_TAPS, MAX_SMEAR_TAPS, MAX_TAPS};
use crate::shader::Program;

pub struct Programs {
    pub separable: Program,
    pub iris: Program,
    pub smear: Program,
}

fn with(constants: &[(&str, u32)], body: &str) -> String {
    let head: String = constants.iter().map(|(name, value)| format!("const int {name} = {value};\n")).collect();
    format!("{head}{body}")
}

impl Programs {
    pub fn compile() -> Result<Self> {
        Ok(Self {
            separable: Program::shader("separable kernel", &with(&[("MAX_TAPS", MAX_TAPS)], include_str!("../shaders/kernel/separable.sksl")))?,
            iris: Program::shader("iris kernel", &with(&[("MAX_IRIS_TAPS", MAX_IRIS_TAPS)], include_str!("../shaders/kernel/iris.sksl")))?,
            smear: Program::shader("smear", &with(&[("MAX_SMEAR_TAPS", MAX_SMEAR_TAPS)], include_str!("../shaders/kernel/smear.sksl")))?,
        })
    }
}
