//! Photo and illustration shots: a graded camera move with optional overlay text.

use anyhow::Result;
use skia_safe::Canvas;

use super::{Loader, Moment, Scene, Stage};
use crate::{
    film::Shot,
    overlay::Overlay,
    pins::Pins,
    photo::{Grade, Photo},
};

pub const PARAMS: &[&str] = &["from", "to", "depth", "grade", "dim", "tag", "badge", "hero", "title", "note", "pins", "pinsFrom"];

struct PhotoShot {
    photo: Photo,
    overlay: Option<Overlay>,
    pins: Option<Pins>,
}

pub fn prepare(loader: &mut Loader, shot: &Shot) -> Result<Box<dyn Scene>> {
    // Illustrations are graded as paintings unless the shot says otherwise.
    let house = (shot.kind == "illustration").then_some(Grade::Painting);
    Ok(Box::new(PhotoShot {
        photo: Photo::for_shot(loader.context, loader.film, loader.public, shot, house)?,
        overlay: Overlay::prepare(loader.fonts, loader.film, shot)?,
        pins: Pins::prepare(loader.fonts, loader.film, shot)?,
    }))
}

impl Scene for PhotoShot {
    fn draw(&self, canvas: &Canvas, stage: &Stage, at: Moment) -> Result<()> {
        self.photo.draw(canvas, stage.photo, stage.film, at.seconds)?;
        if let Some(overlay) = &self.overlay {
            overlay.draw(canvas, at.seconds);
        }
        if let Some(pins) = &self.pins {
            pins.draw(canvas, self.photo.placement(stage.film, at.seconds).rect, at.seconds);
        }
        Ok(())
    }
}
