//! Tier 1 code path on native: `vello_gpu` 0.3.0 on wgpu 30.0.1, **GL backend only** (the `gles`
//! feature is the only wgpu backend compiled in), with `Limits::downlevel_webgl2_defaults()`
//! requested so compute is not relied on even where the driver offers it (FINDINGS F8).
//!
//! Hosts own the surface: they pass `device/queue/target view`. `new_headless` is for tests.

use crate::paint::DisplayList;
use crate::render::replay_gpu;
use vello_gpu::{RenderSize, RenderTargetConfig, Renderer, Resources, Scene, TargetInit, TextureBindings};

pub struct GlRenderer {
    renderer: Renderer,
    resources: Resources,
    scene: Scene,
    format: wgpu::TextureFormat,
    size: (u16, u16),
    depth: wgpu::TextureView,
}

impl GlRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, width: u16, height: u16) -> GlRenderer {
        let (renderer, resources) = Renderer::new(device, &RenderTargetConfig { format, width, height });
        let depth = Renderer::create_depth_texture_view(device, &RenderSize { width, height });
        GlRenderer { renderer, resources, scene: Scene::new(width, height), format, size: (width, height), depth }
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Replay the display list into the vello_gpu scene and draw it into `target`.
    /// The target size must equal `dl.pixel_size()` (re-create the renderer on resize).
    pub fn render(&mut self, dl: &DisplayList, device: &wgpu::Device, queue: &wgpu::Queue, target: &wgpu::TextureView) {
        let (w, h) = dl.pixel_size();
        assert_eq!((w, h), self.size, "GlRenderer size mismatch; recreate on resize");
        self.scene.reset();
        replay_gpu(dl, &mut self.scene, &mut self.resources);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("pvc") });
        self.renderer
            .render(
                &self.scene,
                &mut self.resources,
                device,
                queue,
                &mut encoder,
                &RenderSize { width: w, height: h },
                target,
                Some(&self.depth),
                &TextureBindings::new(),
                TargetInit::Clear(vello_gpu::ClearSettings::default()),
            )
            .expect("vello_gpu render");
        queue.submit([encoder.finish()]);
    }
}

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    use std::task::{Context, Poll, Waker};
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

pub struct Headless {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter_info: wgpu::AdapterInfo,
    pub limits: wgpu::Limits,
}

/// GL-only instance, any adapter the GL backend finds (llvmpipe on this container), device created
/// with the WebGL2 downlevel limits.
pub fn new_headless() -> Result<Headless, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::GL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        force_fallback_adapter: false,
        compatible_surface: None,
        ..Default::default()
    }))
    .map_err(|e| format!("no GL adapter: {e:?}"))?;
    let limits = wgpu::Limits::downlevel_webgl2_defaults();
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("pvc-gl-downlevel"),
        required_features: wgpu::Features::empty(),
        required_limits: limits.clone(),
        ..Default::default()
    }))
    .map_err(|e| format!("request_device: {e:?}"))?;
    Ok(Headless { device, queue, adapter_info: adapter.get_info(), limits })
}

/// Offscreen render-to-texture + readback → RGBA8 (un-premultiplied by the compositor: the target
/// is cleared and fully covered by the opaque page background, so alpha is 255).
pub fn render_gl_rgba(h: &Headless, dl: &DisplayList) -> (u16, u16, Vec<u8>) {
    let (w, hh) = dl.pixel_size();
    let tex = h.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pvc-target"),
        size: wgpu::Extent3d { width: w as u32, height: hh as u32, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    let mut r = GlRenderer::new(&h.device, wgpu::TextureFormat::Rgba8Unorm, w, hh);
    r.render(dl, &h.device, &h.queue, &view);
    readback(h, &tex, w, hh)
}

pub fn readback(h: &Headless, tex: &wgpu::Texture, w: u16, hh: u16) -> (u16, u16, Vec<u8>) {
    let bpr = (w as u32 * 4).next_multiple_of(256);
    let buf = h.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("pvc-readback"),
        size: bpr as u64 * hh as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = h.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("pvc-copy") });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: None } },
        wgpu::Extent3d { width: w as u32, height: hh as u32, depth_or_array_layers: 1 },
    );
    h.queue.submit([enc.finish()]);
    buf.slice(..).map_async(wgpu::MapMode::Read, |r| r.expect("map"));
    h.device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
    let data = buf.slice(..).get_mapped_range().expect("range");
    let mut out = Vec::with_capacity(w as usize * hh as usize * 4);
    for row in data.chunks_exact(bpr as usize) {
        out.extend_from_slice(&row[..w as usize * 4]);
    }
    drop(data);
    buf.unmap();
    (w, hh, out)
}
