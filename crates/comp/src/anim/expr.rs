//! A small expression language for properties, in the spirit of After Effects expressions but pure and total:
//! numbers, `+ - * / % ^`, comparisons, `&& || !`, `a ? b : c`, constants and functions. It cannot loop, allocate
//! or touch the clock, so a frame is still a function of time.
//!
//! ```text
//! value + wiggle(3, 20)                      // shake around the keyframed value
//! linear(time, 0, 2, 0, 360)                  // 0..360 over two seconds
//! ref("title", "x") + 40                      // follow another layer
//! 1 - beat()                                  // a pulse that kicks on every beat
//! amp("kick") * 80                            // follow the loudness of an audio file
//! ```

use anyhow::{anyhow, bail, ensure, Result};

use super::noise;

#[derive(Debug, Clone)]
pub enum Node {
    Num(f64),
    Str(String),
    Var(String),
    Neg(Box<Node>),
    Not(Box<Node>),
    Bin(char, Box<Node>, Box<Node>),
    Cmp(Cmp, Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
    If(Box<Node>, Box<Node>, Box<Node>),
}

#[derive(Debug, Clone, Copy)]
pub enum Cmp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub source: String,
    root: Node,
}

/// What an expression may ask the outside world: variables (`time`, `value`, `index`...) and the host functions
/// (`ref`, `amp`, `beat`...). Everything else is pure maths inside `eval`.
pub trait Env {
    fn var(&self, name: &str) -> Option<f64>;
    /// A host function; `None` means "no such function".
    fn call(&self, name: &str, args: &[Arg]) -> Option<Result<f64>>;
    /// A per-property number mixed into `wiggle`/`random` so two properties do not move in lockstep.
    fn salt(&self) -> u32;
}

#[derive(Debug, Clone)]
pub enum Arg {
    Num(f64),
    Str(String),
}

impl Arg {
    pub fn num(&self) -> Result<f64> {
        match self {
            Arg::Num(v) => Ok(*v),
            Arg::Str(s) => bail!("expected a number, got the text {s:?}"),
        }
    }
    pub fn str(&self) -> Result<&str> {
        match self {
            Arg::Str(s) => Ok(s),
            Arg::Num(v) => bail!("expected text in quotes, got the number {v}"),
        }
    }
}

/// Names the evaluator itself defines. Anything else is looked up in the `Env`.
const BUILTIN_FNS: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "abs", "min", "max", "floor", "ceil", "round", "sqrt", "pow", "exp", "ln", "sign", "fract", "clamp", "lerp",
    "linear", "smooth", "ease", "noise", "noise2", "wiggle", "random", "radians", "degrees", "mod", "step", "pingpong",
];

impl Expr {
    pub fn parse(source: &str) -> Result<Expr> {
        let tokens = lex(source)?;
        let mut p = Parser { tokens, at: 0 };
        let root = p.ternary()?;
        ensure!(p.at == p.tokens.len(), "expression {source:?}: unexpected {:?} after the end", p.tokens[p.at]);
        Ok(Expr { source: source.to_string(), root })
    }

    pub fn eval(&self, env: &dyn Env) -> Result<f64> {
        eval(&self.root, env).map_err(|e| anyhow!("in expression {:?}: {e}", self.source))
    }

    /// Names of variables and functions used, for validation.
    pub fn names(&self) -> (Vec<String>, Vec<String>) {
        fn walk(n: &Node, vars: &mut Vec<String>, fns: &mut Vec<String>) {
            match n {
                Node::Var(v) => vars.push(v.clone()),
                Node::Call(f, args) => {
                    fns.push(f.clone());
                    args.iter().for_each(|a| walk(a, vars, fns));
                }
                Node::Neg(a) | Node::Not(a) => walk(a, vars, fns),
                Node::Bin(_, a, b) | Node::Cmp(_, a, b) => {
                    walk(a, vars, fns);
                    walk(b, vars, fns);
                }
                Node::If(a, b, c) => {
                    walk(a, vars, fns);
                    walk(b, vars, fns);
                    walk(c, vars, fns);
                }
                Node::Num(_) | Node::Str(_) => {}
            }
        }
        let (mut vars, mut fns) = (vec![], vec![]);
        walk(&self.root, &mut vars, &mut fns);
        (vars, fns)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Id(String),
    Op(&'static str),
}

fn lex(src: &str) -> Result<Vec<Tok>> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = vec![];
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit)) {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            out.push(Tok::Num(text.parse().map_err(|_| anyhow!("bad number {text:?}"))?));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Tok::Id(chars[start..i].iter().collect()));
        } else if c == '"' || c == '\'' {
            let start = i + 1;
            i += 1;
            while i < chars.len() && chars[i] != c {
                i += 1;
            }
            ensure!(i < chars.len(), "unterminated text in expression {src:?}");
            out.push(Tok::Str(chars[start..i].iter().collect()));
            i += 1;
        } else {
            let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
            let op = ["<=", ">=", "==", "!=", "&&", "||"].iter().find(|o| **o == two);
            if let Some(op) = op {
                out.push(Tok::Op(op));
                i += 2;
            } else {
                let one = ["+", "-", "*", "/", "%", "^", "(", ")", ",", "<", ">", "!", "?", ":"].iter().find(|o| o.chars().next() == Some(c));
                out.push(Tok::Op(one.ok_or_else(|| anyhow!("unexpected character {c:?} in expression {src:?}"))?));
                i += 1;
            }
        }
    }
    Ok(out)
}

struct Parser {
    tokens: Vec<Tok>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.tokens.get(self.at)
    }
    fn eat(&mut self, op: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Op(o)) if *o == op) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, op: &str) -> Result<()> {
        ensure!(self.eat(op), "expected {op:?} but found {:?}", self.peek());
        Ok(())
    }

    fn ternary(&mut self) -> Result<Node> {
        let cond = self.or()?;
        if self.eat("?") {
            let a = self.ternary()?;
            self.expect(":")?;
            let b = self.ternary()?;
            return Ok(Node::If(Box::new(cond), Box::new(a), Box::new(b)));
        }
        Ok(cond)
    }
    fn or(&mut self) -> Result<Node> {
        let mut l = self.and()?;
        while self.eat("||") {
            l = Node::Cmp(Cmp::Or, Box::new(l), Box::new(self.and()?));
        }
        Ok(l)
    }
    fn and(&mut self) -> Result<Node> {
        let mut l = self.cmp()?;
        while self.eat("&&") {
            l = Node::Cmp(Cmp::And, Box::new(l), Box::new(self.cmp()?));
        }
        Ok(l)
    }
    fn cmp(&mut self) -> Result<Node> {
        let l = self.sum()?;
        for (s, c) in [("<=", Cmp::Le), (">=", Cmp::Ge), ("==", Cmp::Eq), ("!=", Cmp::Ne), ("<", Cmp::Lt), (">", Cmp::Gt)] {
            if self.eat(s) {
                return Ok(Node::Cmp(c, Box::new(l), Box::new(self.sum()?)));
            }
        }
        Ok(l)
    }
    fn sum(&mut self) -> Result<Node> {
        let mut l = self.product()?;
        loop {
            if self.eat("+") {
                l = Node::Bin('+', Box::new(l), Box::new(self.product()?));
            } else if self.eat("-") {
                l = Node::Bin('-', Box::new(l), Box::new(self.product()?));
            } else {
                return Ok(l);
            }
        }
    }
    fn product(&mut self) -> Result<Node> {
        let mut l = self.unary()?;
        loop {
            let op = if self.eat("*") { '*' } else if self.eat("/") { '/' } else if self.eat("%") { '%' } else { return Ok(l) };
            l = Node::Bin(op, Box::new(l), Box::new(self.unary()?));
        }
    }
    fn unary(&mut self) -> Result<Node> {
        if self.eat("-") {
            return Ok(Node::Neg(Box::new(self.unary()?)));
        }
        if self.eat("!") {
            return Ok(Node::Not(Box::new(self.unary()?)));
        }
        self.power()
    }
    fn power(&mut self) -> Result<Node> {
        let base = self.atom()?;
        if self.eat("^") {
            return Ok(Node::Bin('^', Box::new(base), Box::new(self.unary()?)));
        }
        Ok(base)
    }
    fn atom(&mut self) -> Result<Node> {
        match self.tokens.get(self.at).cloned() {
            Some(Tok::Num(v)) => {
                self.at += 1;
                Ok(Node::Num(v))
            }
            Some(Tok::Str(s)) => {
                self.at += 1;
                Ok(Node::Str(s))
            }
            Some(Tok::Id(name)) => {
                self.at += 1;
                if self.eat("(") {
                    let mut args = vec![];
                    if !self.eat(")") {
                        loop {
                            args.push(self.ternary()?);
                            if self.eat(")") {
                                break;
                            }
                            self.expect(",")?;
                        }
                    }
                    Ok(Node::Call(name, args))
                } else {
                    Ok(Node::Var(name))
                }
            }
            Some(Tok::Op("(")) => {
                self.at += 1;
                let e = self.ternary()?;
                self.expect(")")?;
                Ok(e)
            }
            other => bail!("expected a value but found {other:?}"),
        }
    }
}

fn eval(n: &Node, env: &dyn Env) -> Result<f64> {
    Ok(match n {
        Node::Num(v) => *v,
        Node::Str(s) => bail!("text {s:?} is only allowed as a function argument"),
        Node::Var(v) => match v.as_str() {
            "pi" => std::f64::consts::PI,
            "tau" => std::f64::consts::TAU,
            "e" => std::f64::consts::E,
            _ => env.var(v).ok_or_else(|| anyhow!("unknown variable {v:?}"))?,
        },
        Node::Neg(a) => -eval(a, env)?,
        Node::Not(a) => f64::from(eval(a, env)? == 0.0),
        Node::Bin(op, a, b) => {
            let (x, y) = (eval(a, env)?, eval(b, env)?);
            match op {
                '+' => x + y,
                '-' => x - y,
                '*' => x * y,
                '/' => if y == 0.0 { 0.0 } else { x / y },
                '%' => if y == 0.0 { 0.0 } else { x.rem_euclid(y) },
                _ => x.powf(y),
            }
        }
        Node::Cmp(c, a, b) => {
            let x = eval(a, env)?;
            match c {
                Cmp::And => f64::from(x != 0.0 && eval(b, env)? != 0.0),
                Cmp::Or => f64::from(x != 0.0 || eval(b, env)? != 0.0),
                _ => {
                    let y = eval(b, env)?;
                    f64::from(match c {
                        Cmp::Lt => x < y,
                        Cmp::Le => x <= y,
                        Cmp::Gt => x > y,
                        Cmp::Ge => x >= y,
                        Cmp::Eq => x == y,
                        _ => x != y,
                    })
                }
            }
        }
        Node::If(c, a, b) => if eval(c, env)? != 0.0 { eval(a, env)? } else { eval(b, env)? },
        Node::Call(name, args) => {
            let args = args
                .iter()
                .map(|a| match a {
                    Node::Str(s) => Ok(Arg::Str(s.clone())),
                    other => eval(other, env).map(Arg::Num),
                })
                .collect::<Result<Vec<_>>>()?;
            if BUILTIN_FNS.contains(&name.as_str()) {
                builtin(name, &args, env)?
            } else {
                env.call(name, &args).ok_or_else(|| anyhow!("unknown function {name}()"))??
            }
        }
    })
}

fn builtin(name: &str, a: &[Arg], env: &dyn Env) -> Result<f64> {
    let n = |i: usize| -> Result<f64> { a.get(i).ok_or_else(|| anyhow!("{name}() needs more arguments"))?.num() };
    let opt = |i: usize, d: f64| -> Result<f64> { a.get(i).map_or(Ok(d), Arg::num) };
    Ok(match name {
        "sin" => n(0)?.sin(),
        "cos" => n(0)?.cos(),
        "tan" => n(0)?.tan(),
        "asin" => n(0)?.asin(),
        "acos" => n(0)?.acos(),
        "atan" => n(0)?.atan(),
        "atan2" => n(0)?.atan2(n(1)?),
        "abs" => n(0)?.abs(),
        "min" => n(0)?.min(n(1)?),
        "max" => n(0)?.max(n(1)?),
        "floor" => n(0)?.floor(),
        "ceil" => n(0)?.ceil(),
        "round" => n(0)?.round(),
        "sqrt" => n(0)?.max(0.0).sqrt(),
        "pow" => n(0)?.powf(n(1)?),
        "exp" => n(0)?.exp(),
        "ln" => n(0)?.max(1e-12).ln(),
        "sign" => n(0)?.signum() * f64::from(n(0)? != 0.0),
        "fract" => n(0)? - n(0)?.floor(),
        "mod" => if n(1)? == 0.0 { 0.0 } else { n(0)?.rem_euclid(n(1)?) },
        "step" => f64::from(n(1)? >= n(0)?),
        "pingpong" => {
            let (x, len) = (n(0)?, n(1)?.max(1e-9));
            let m = x.rem_euclid(2.0 * len);
            if m > len { 2.0 * len - m } else { m }
        }
        "radians" => n(0)?.to_radians(),
        "degrees" => n(0)?.to_degrees(),
        "clamp" => n(0)?.clamp(n(1)?.min(n(2)?), n(1)?.max(n(2)?)),
        "lerp" => n(0)? + (n(1)? - n(0)?) * n(2)?,
        // linear(x, x0, x1, y0, y1): x0..x1 mapped to y0..y1, clamped.
        "linear" => {
            let (x, x0, x1, y0, y1) = (n(0)?, n(1)?, n(2)?, n(3)?, n(4)?);
            let u = if x1 == x0 { 1.0 } else { ((x - x0) / (x1 - x0)).clamp(0.0, 1.0) };
            y0 + (y1 - y0) * u
        }
        // smooth(x0, x1, x): smoothstep.
        "smooth" => {
            let u = ((n(2)? - n(0)?) / (n(1)? - n(0)?).max(1e-12)).clamp(0.0, 1.0);
            u * u * (3.0 - 2.0 * u)
        }
        // ease("expo.out", x, x0, x1, y0, y1)
        "ease" => {
            let e = super::ease::Ease::parse(a.first().ok_or_else(|| anyhow!("ease() needs a name"))?.str()?)?;
            let (x, x0, x1, y0, y1) = (n(1)?, opt(2, 0.0)?, opt(3, 1.0)?, opt(4, 0.0)?, opt(5, 1.0)?);
            let u = if x1 == x0 { 1.0 } else { ((x - x0) / (x1 - x0)).clamp(0.0, 1.0) };
            y0 + (y1 - y0) * e.at(u)
        }
        "noise" => noise::noise1(env.salt(), n(0)?),
        "noise2" => noise::noise2(env.salt(), n(0)?, n(1)?),
        // wiggle(freq, amp, octaves = 1): smooth random offset in -amp..amp, a function of `time`.
        "wiggle" => {
            let time = env.var("time").ok_or_else(|| anyhow!("wiggle() needs a time"))?;
            noise::fbm1(env.salt(), time * n(0)?, opt(2, 1.0)?.max(1.0) as u32) * n(1)?
        }
        // random(seed): the same number every call, in 0..1.
        "random" => noise::rand01(env.salt(), n(0)?.floor() as i32, 7),
        _ => unreachable!("{name} is listed in BUILTIN_FNS"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct T(f64);
    impl Env for T {
        fn var(&self, name: &str) -> Option<f64> {
            match name {
                "time" => Some(self.0),
                "value" => Some(100.0),
                _ => None,
            }
        }
        fn call(&self, _: &str, _: &[Arg]) -> Option<Result<f64>> {
            None
        }
        fn salt(&self) -> u32 {
            5
        }
    }

    fn run(src: &str, t: f64) -> f64 {
        Expr::parse(src).unwrap().eval(&T(t)).unwrap()
    }

    #[test]
    fn arithmetic_precedence_and_functions() {
        assert_eq!(run("1 + 2 * 3 ^ 2", 0.0), 19.0);
        assert_eq!(run("-2 ^ 2", 0.0), -4.0);
        assert_eq!(run("time > 1 ? 10 : 20", 2.0), 10.0);
        assert_eq!(run("linear(time, 0, 2, 0, 360)", 1.0), 180.0);
        assert_eq!(run("clamp(value * 2, 0, 150)", 0.0), 150.0);
        assert!((run("ease('power2.in', time, 0, 1, 0, 8)", 0.5) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn wiggle_is_deterministic_bounded_and_moves() {
        let a = run("wiggle(2, 30)", 0.37);
        assert_eq!(a, run("wiggle(2, 30)", 0.37));
        assert!(a.abs() <= 30.0);
        assert_ne!(run("wiggle(2, 30)", 0.1), run("wiggle(2, 30)", 0.3));
    }

    #[test]
    fn errors_name_the_problem() {
        assert!(Expr::parse("1 +").is_err());
        assert!(Expr::parse("1 $ 2").is_err());
        let e = Expr::parse("nope + 1").unwrap().eval(&T(0.0)).unwrap_err().to_string();
        assert!(e.contains("unknown variable"), "{e}");
    }
}
