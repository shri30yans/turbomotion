//! Glass panel layers (cards, lists, tickers), built once per style and reused.

use std::rc::Rc;

use anyhow::{Context, Result};
use skia_safe::{Canvas, Color, Paint, PaintStyle, RRect, Rect, TileMode};

use crate::studio::{
    layer::{linear, Layer},
    model::{Rgb, PAPER},
    noise,
    rng::RandomState,
    shader::Uniforms,
    stage::Stage,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Panel {
    pub w: i32,
    pub h: i32,
    pub radius: f64,
    pub fill: Rgb,
    pub fill_a: f64,
    pub border: Rgb,
    pub accent: Option<Rgb>,
}

impl Panel {
    fn key(&self) -> (i32, i32, i64, Rgb, i64, Rgb, Option<Rgb>) {
        (self.w, self.h, (self.radius * 10.0) as i64, self.fill, (self.fill_a * 1000.0) as i64, self.border, self.accent)
    }
}

fn color(c: Rgb, alpha: u8) -> Color {
    Color::from_argb(alpha, c[0], c[1], c[2])
}

/// The panel's rounded body, outline, optional accent strip and top sheen, with no content.
pub fn panel(st: &Stage, cv: &Canvas, p: &Panel) -> Result<Rc<Layer>> {
    st.panels.get_or_try_insert(p.key(), || {
        let (w, h) = (p.w as f32, p.h as f32);
        let body = Layer::render(cv, p.w, p.h, |c| {
            let mut paint = Paint::default();
            paint.set_anti_alias(true).set_color(color(p.fill, (255.0 * p.fill_a) as u8));
            let radius = p.radius as f32;
            c.draw_rrect(RRect::new_rect_xy(Rect::from_wh(w, h), radius, radius), &paint);
            paint.set_style(PaintStyle::Stroke).set_stroke_width(2.0).set_color(color(p.border, 255));
            c.draw_rrect(RRect::new_rect_xy(Rect::from_wh(w, h).with_inset((1.0, 1.0)), radius - 1.0, radius - 1.0), &paint);
            if let Some(accent) = p.accent {
                paint.set_style(PaintStyle::Fill).set_color(color(accent, 255));
                c.draw_rrect(RRect::new_rect_xy(Rect::from_wh(10.0, h), 5.0, 5.0), &paint);
            }
        })?;
        let sheen = st.shaders.glow_top.instance(Uniforms::new().f(f64::from(p.h)).f(0.14), &[body.image().to_shader(None, skia_safe::SamplingOptions::default(), None).context("panel shader")?])?;
        Layer::render(cv, p.w, p.h, |c| {
            let mut paint = Paint::default();
            paint.set_shader(sheen);
            c.draw_rect(Rect::from_wh(w, h), &paint);
        })
    })
}

/// A sheet of paper `w` x `h`: the stock colour with seeded mottling and fibre.
pub fn paper(st: &Stage, cv: &Canvas, w: i32, h: i32) -> Result<Rc<Layer>> {
    const SEED: u32 = 3;
    st.papers.get_or_try_insert((w, h), || {
        let mut rng = RandomState::new(SEED);
        let coarse = noise::stretch(cv, &noise::gaussian(&mut rng, (w / 3 + 1) as usize, (h / 3 + 1) as usize)?, w, h)?;
        let fibres = Layer::from_image(noise::gaussian(&mut rng, w as usize, h as usize)?).blurred_with_edge(cv, 1.0, TileMode::Clamp)?;
        let sample = |l: &Layer| l.image().to_shader((TileMode::Clamp, TileMode::Clamp), linear(), None).context("image shader");
        let shader = st.shaders.paper.instance(Uniforms::new().rgb(PAPER), &[sample(&coarse)?, sample(&fibres)?])?;
        Layer::render(cv, w, h, |c| {
            let mut paint = Paint::default();
            paint.set_shader(shader);
            c.draw_rect(Rect::from_wh(w as f32, h as f32), &paint);
        })
    })
}
