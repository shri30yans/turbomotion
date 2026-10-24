//! Film-level layers drawn over every shot: chapter marks and the paper grain.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use skia_safe::{BlendMode, Canvas, Color, Data, Image, Paint, Point, RRect, Rect, SamplingOptions, TileMode};

use crate::{
    ease::Ease,
    film::Film,
    layer::with_backdrop_blur,
    motion::{self, Tween},
    palette::{OCHRE, WHITE},
    text::{Family, Fonts, Style, TextBlock, UNBOUNDED},
};

const MARK_SECONDS: f32 = 4.2;
const GRAIN_OPACITY: f32 = 0.1;
const RULE: (f32, f32) = (4.0, 46.0);
const GAP: f32 = 22.0;
const PADDING: (f32, f32, f32, f32) = (18.0, 30.0, 18.0, 22.0); // top right bottom left

struct Mark {
    start_frame: u32,
    number: TextBlock,
    title: TextBlock,
}

pub struct Chrome {
    marks: Vec<Mark>,
    grain: Image,
    size: (f32, f32),
}

impl Chrome {
    /// `assets` is the asset directory (`fonts/`, `grain/`).
    pub fn prepare(fonts: &Fonts, film: &Film, assets: &Path) -> Result<Self> {
        let number_style = Style { tabular: true, ..Style::new(Family::Fraunces, 44.0, 600.0, OCHRE) };
        let title_style = Style { letter_spacing: -0.4, ..Style::new(Family::Manrope, 34.0, 600.0, WHITE) };
        let marks = film
            .chapters
            .iter()
            .enumerate()
            .skip(1)
            .map(|(i, chapter)| {
                Ok(Mark {
                    start_frame: chapter.start_frame,
                    number: fonts.typeset(&format!("{i:02}"), &number_style, UNBOUNDED, true)?,
                    title: fonts.typeset(&chapter.title, &title_style, UNBOUNDED, false)?,
                })
            })
            .collect::<Result<_>>()?;
        let path = assets.join("grain/0.png");
        let bytes = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        let grain = Image::from_encoded(Data::new_copy(&bytes)).with_context(|| format!("decoding {}", path.display()))?;
        Ok(Self { marks, grain, size: (film.width as f32, film.height as f32) })
    }

    pub fn draw(&self, canvas: &Canvas, frame: u32, fps: u32) -> Result<()> {
        let mark_frames = (MARK_SECONDS * fps as f32).round() as u32;
        for mark in self.marks.iter().filter(|m| (m.start_frame..m.start_frame + mark_frames).contains(&frame)) {
            self.draw_mark(canvas, mark, (frame - mark.start_frame) as f32 / fps as f32);
        }
        self.draw_grain(canvas)
    }

    fn draw_mark(&self, canvas: &Canvas, mark: &Mark, seconds: f32) {
        let content_h = mark.number.height.max(mark.title.height).max(RULE.1);
        let (top, right, bottom, left) = PADDING;
        let width = left + mark.number.content_width() + GAP + RULE.0 + GAP + mark.title.content_width() + right;
        let height = top + content_h + bottom;
        let leave = Tween::new(3.4, 0.5, Ease::PowerIn(2)).progress(seconds);
        let panel = Rect::from_xywh(72.0, self.size.1 - 72.0 - height - 10.0 * leave, width, height);
        let shape = RRect::new_rect_xy(panel, 6.0, 6.0);

        with_backdrop_blur(canvas, shape, 8.0, 1.0 - leave, || {
            let mut fill = Paint::default();
            fill.set_color(Color::from_argb(184, 22, 17, 13));
            canvas.draw_rrect(shape, &fill);

            let centred = |h: f32| panel.top + top + (content_h - h) / 2.0;
            let number_x = panel.left + left;
            mark.number.draw_chars(canvas, Point::new(number_x, centred(mark.number.height)), |n| {
                motion::mask_slide(seconds, 0.2, 0.04, n, 0.6, 105.0)
            });

            let rule_x = number_x + mark.number.content_width() + GAP;
            let drawn = motion::draw(seconds, 0.15, 0.6, Ease::PowerInOut(2));
            let rule_top = centred(RULE.1);
            let mut stroke = Paint::default();
            stroke.set_color(OCHRE).set_anti_alias(true);
            canvas.draw_rect(Rect::from_xywh(rule_x + 0.5, rule_top, 3.0, RULE.1 * drawn), &stroke);

            let title_x = rule_x + RULE.0 + GAP;
            mark.title.draw_lines(canvas, Point::new(title_x, centred(mark.title.height)), |i| {
                motion::mask_slide(seconds, 0.32, 0.05, i, 0.7, 110.0)
            });
        });
    }

    /// One static tile at natural size, overlay-blended over the finished frame.
    fn draw_grain(&self, canvas: &Canvas) -> Result<()> {
        let shader = self
            .grain
            .to_shader((TileMode::Repeat, TileMode::Repeat), SamplingOptions::default(), None)
            .context("grain shader")?;
        let mut paint = Paint::default();
        paint.set_shader(shader).set_blend_mode(BlendMode::Overlay).set_alpha_f(GRAIN_OPACITY);
        canvas.draw_rect(Rect::from_wh(self.size.0, self.size.1), &paint);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mark_window_is_four_point_two_seconds() {
        assert_eq!((MARK_SECONDS * 30.0).round() as u32, 126);
    }
}
