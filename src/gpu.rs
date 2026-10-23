//! Skia on Metal: one device, one direct context, BGRA render targets.

use anyhow::{Context, Result};
use metal::foreign_types::ForeignType;
use skia_safe::{
    gpu::{self, direct_contexts, mtl, Budgeted, DirectContext, SurfaceOrigin},
    AlphaType, ColorType, ISize, ImageInfo, Surface,
};

pub struct Gpu {
    pub context: DirectContext,
    // Declared after `context` so the Metal objects outlive nothing that still uses them.
    _device: metal::Device,
    _queue: metal::CommandQueue,
}

impl Gpu {
    pub fn new() -> Result<Self> {
        let device = metal::Device::system_default().context("no Metal device")?;
        let queue = device.new_command_queue();
        // SAFETY: both handles are live Metal objects; Skia takes its own reference to each.
        let backend = unsafe { mtl::BackendContext::new(device.as_ptr() as mtl::Handle, queue.as_ptr() as mtl::Handle) };
        let context = direct_contexts::make_metal(&backend, None).context("Skia could not create a Metal context")?;
        Ok(Self { context, _device: device, _queue: queue })
    }

    /// An opaque BGRA render target, the byte order ffmpeg ingests as `bgra`.
    pub fn frame_surface(&mut self, size: ISize) -> Result<Surface> {
        let info = ImageInfo::new(size, ColorType::BGRA8888, AlphaType::Premul, None);
        gpu::surfaces::render_target(&mut self.context, Budgeted::Yes, &info, None, SurfaceOrigin::TopLeft, None, false, false)
            .context("could not allocate the frame surface")
    }
}
