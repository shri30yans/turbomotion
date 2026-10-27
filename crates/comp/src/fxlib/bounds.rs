//! `// @bounds`: how far a plugin's output reaches past the pixels it is given.
//!
//! ```text
//! // @bounds none                              the output stays inside the input's box
//! // @bounds expand=blur*3                     every side grows by this many pixels (an expression of the float parameters)
//! // @bounds expand_x=reach expand_y=0         per axis
//! // @bounds source=map                        the output covers the named layer input as well
//! ```
//!
//! An expression is numbers, float parameter names (a choice is the number of its option), `box_w` and `box_h` (the layer's own width and height in pixels, so
//! a growth can be a share of the layer: `expand_x=box_w*(scale-1)/2`), `+ - * /`, brackets and `abs`, `min`, `max`,
//! written without spaces. A plugin without the tag has not said: in layer space its pixels cannot grow and `check` warns
//! (`fx-needs-bounds`).

use anyhow::{anyhow, bail, ensure, Context, Result};
use turbomotion_core::suggest::hint;

use super::{kernel_pass::expression_names, Param};

/// The names an expression may use besides the float parameters: the layer's box width and height.
const BOX_NAMES: [&str; 2] = ["box_w", "box_h"];

/// What a plugin declares about its reach.
#[derive(Debug, Clone, PartialEq)]
pub enum Bounds {
    /// No `@bounds` tag.
    Undeclared,
    /// `none`: the output is inside the input's box.
    Within,
    /// Grows each axis by an expression, in pixels per side.
    Expand { x: Expr, y: Expr },
    /// The output covers the layer input called this.
    Source(String),
}

impl Bounds {
    /// Whether the plugin says it can reach past its input.
    pub fn grows(&self) -> bool {
        matches!(self, Bounds::Expand { .. } | Bounds::Source(_))
    }

    /// How the tag reads, for the reference pages (`undeclared` when there is none).
    pub fn describe(&self) -> String {
        match self {
            Bounds::Undeclared => "undeclared".to_string(),
            Bounds::Within => "none".to_string(),
            Bounds::Expand { x, y } if x == y => format!("expand={}", x.source),
            Bounds::Expand { x, y } => format!("expand_x={} expand_y={}", x.source, y.source),
            Bounds::Source(input) => format!("source={input}"),
        }
    }

    /// Parse the text after `@bounds`; `params` are the plugin's parameters and `inputs` its layer inputs.
    pub fn parse(text: &str, params: &[Param], inputs: &[&str]) -> Result<Bounds> {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words == ["none"] {
            return Ok(Bounds::Within);
        }
        ensure!(!words.is_empty(), "@bounds is `none`, `expand=<expr>`, `expand_x=<expr> expand_y=<expr>` or `source=<input>`");
        let mut floats = expression_names(params);
        if let Some(clash) = floats.iter().find(|n| BOX_NAMES.contains(n)) {
            bail!("a parameter called {clash:?} would hide the layer's size in @bounds, where box_w and box_h are the layer's width and height; rename the parameter");
        }
        floats.extend(BOX_NAMES);
        let (mut all, mut x, mut y, mut source) = (None, None, None, None);
        for w in words {
            let (key, value) = w.split_once('=').ok_or_else(|| anyhow!("{w:?} is not key=value; @bounds takes expand=, expand_x=, expand_y= or source="))?;
            let slot = match key {
                "expand" => &mut all,
                "expand_x" => &mut x,
                "expand_y" => &mut y,
                "source" => {
                    ensure!(inputs.contains(&value), "source={value} is not a layer input of this effect{}", hint(value, inputs));
                    source = Some(value.to_string());
                    continue;
                }
                other => bail!("unknown @bounds key {other:?}; the keys are expand, expand_x, expand_y and source"),
            };
            ensure!(slot.is_none(), "{key}= is given twice");
            *slot = Some(Expr::parse(value, &floats).with_context(|| format!("{key}={value}"))?);
        }
        match (all, x, y, source) {
            (Some(e), None, None, None) => Ok(Bounds::Expand { x: e.clone(), y: e }),
            (None, x, y, None) if x.is_some() || y.is_some() => Ok(Bounds::Expand { x: x.unwrap_or_else(Expr::zero), y: y.unwrap_or_else(Expr::zero) }),
            (None, None, None, Some(input)) => Ok(Bounds::Source(input)),
            _ => bail!("@bounds takes expand= alone, expand_x=/expand_y= together or alone, or source= alone"),
        }
    }

    /// Pixels added to each side, from the evaluated float parameters (`value(name)` is a parameter's value now) and the
    /// layer's own box `content` (`[left, top, right, bottom]`, what `box_w` and `box_h` measure).
    pub fn growth(&self, content: [f64; 4], value: &dyn Fn(&str) -> Option<f64>) -> Result<[f64; 4]> {
        let with_box = |name: &str| match name {
            "box_w" => Some(content[2] - content[0]),
            "box_h" => Some(content[3] - content[1]),
            other => value(other),
        };
        Ok(match self {
            Bounds::Undeclared | Bounds::Within => [0.0; 4],
            Bounds::Expand { x, y } => {
                let (x, y) = (x.eval(&with_box)?.max(0.0), y.eval(&with_box)?.max(0.0));
                [x, y, x, y]
            }
            Bounds::Source(_) => bail!("`@bounds source=` reaches another layer's box, which layer space does not support yet"),
        })
    }
}

/// A small arithmetic expression over a plugin's float parameters.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    source: String,
    node: Node,
}

#[derive(Debug, Clone, PartialEq)]
enum Node {
    Num(f64),
    Param(String),
    Neg(Box<Node>),
    Bin(char, Box<Node>, Box<Node>),
    Call(&'static str, Vec<Node>),
}

impl Expr {
    fn zero() -> Expr {
        Expr { source: "0".to_string(), node: Node::Num(0.0) }
    }

    /// Parse `source`, where `names` are the identifiers it may use.
    pub fn parse(source: &str, names: &[&str]) -> Result<Expr> {
        let mut p = Parser { chars: source.chars().collect(), at: 0, names };
        let node = p.sum()?;
        ensure!(p.at == p.chars.len(), "unexpected {:?} at character {}", p.chars[p.at], p.at + 1);
        Ok(Expr { source: source.to_string(), node })
    }

    pub fn eval(&self, value: &dyn Fn(&str) -> Option<f64>) -> Result<f64> {
        let v = self.node.eval(value)?;
        ensure!(v.is_finite(), "`{}` is not a finite number ({v})", self.source);
        Ok(v)
    }
}

impl Node {
    fn eval(&self, value: &dyn Fn(&str) -> Option<f64>) -> Result<f64> {
        Ok(match self {
            Node::Num(n) => *n,
            Node::Param(name) => value(name).ok_or_else(|| anyhow!("parameter {name:?} has no value"))?,
            Node::Neg(n) => -n.eval(value)?,
            Node::Bin(op, a, b) => {
                let (a, b) = (a.eval(value)?, b.eval(value)?);
                match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    _ => a / b,
                }
            }
            Node::Call(name, args) => {
                let v = args.iter().map(|a| a.eval(value)).collect::<Result<Vec<_>>>()?;
                match *name {
                    "abs" => v[0].abs(),
                    "min" => v[0].min(v[1]),
                    _ => v[0].max(v[1]),
                }
            }
        })
    }
}

struct Parser<'a> {
    chars: Vec<char>,
    at: usize,
    names: &'a [&'a str],
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        let hit = self.peek() == Some(c);
        self.at += usize::from(hit);
        hit
    }

    fn sum(&mut self) -> Result<Node> {
        let mut left = self.product()?;
        while let Some(op @ ('+' | '-')) = self.peek() {
            self.at += 1;
            left = Node::Bin(op, Box::new(left), Box::new(self.product()?));
        }
        Ok(left)
    }

    fn product(&mut self) -> Result<Node> {
        let mut left = self.unary()?;
        while let Some(op @ ('*' | '/')) = self.peek() {
            self.at += 1;
            left = Node::Bin(op, Box::new(left), Box::new(self.unary()?));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Node> {
        if self.eat('-') {
            return Ok(Node::Neg(Box::new(self.unary()?)));
        }
        self.atom()
    }

    fn atom(&mut self) -> Result<Node> {
        if self.eat('(') {
            let inner = self.sum()?;
            ensure!(self.eat(')'), "a bracket is not closed");
            return Ok(inner);
        }
        let start = self.at;
        while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.') {
            self.at += 1;
        }
        let word: String = self.chars[start..self.at].iter().collect();
        ensure!(!word.is_empty(), "expected a number, a parameter name or a bracket at character {}", start + 1);
        if word.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
            return word.parse::<f64>().map(Node::Num).map_err(|_| anyhow!("{word:?} is not a number"));
        }
        if self.eat('(') {
            let (name, arity): (&'static str, usize) = match word.as_str() {
                "abs" => ("abs", 1),
                "min" => ("min", 2),
                "max" => ("max", 2),
                other => bail!("no function {other:?}; the functions are abs, min and max"),
            };
            let mut args = vec![self.sum()?];
            while self.eat(',') {
                args.push(self.sum()?);
            }
            ensure!(self.eat(')'), "the call to {name} is not closed");
            ensure!(args.len() == arity, "{name} takes {arity} argument{}, got {}", if arity == 1 { "" } else { "s" }, args.len());
            return Ok(Node::Call(name, args));
        }
        ensure!(self.names.contains(&word.as_str()), "{word:?} is not a float parameter of this effect{}", hint(&word, self.names));
        Ok(Node::Param(word))
    }
}

#[cfg(test)]
#[path = "bounds_tests.rs"]
mod tests;
