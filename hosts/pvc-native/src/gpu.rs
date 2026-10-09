//! wgpu 30.0.1 GL-backend surface; drawing is `pvc_runtime::render_gl::GlRenderer` (vello_gpu 0.3.0),
//! device created with WebGL2 downlevel limits imposed (PLAN V2).
use pvc_runtime::paint::DisplayList;
use pvc_runtime::render_gl::GlRenderer;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Instant;
use winit::window::Window;

pub fn block_on<F: std::future::Future>(f: F) -> F::Output {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Renderer {
    /// vello_gpu on wgpu-GL (Tier 1 code path)
    Gl,
    /// vello_cpu raster (Tier 0 c), pixels uploaded to the same wgpu-GL surface texture. `mode=windowed`,
    /// present = texture upload + GL present (not a native X11 PutImage path).
    Cpu,
}

struct CpuState {
    ctx: vello_cpu::RenderContext,
    res: vello_cpu::Resources,
    pm: vello_cpu::Pixmap,
    tex: wgpu::Texture,
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
}

const BLIT_WGSL: &str = r#"
@group(0) @binding(0) var t: texture_2d<f32>;
@group(0) @binding(1) var s: sampler;
struct VO { @builtin(position) p: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) i: u32) -> VO {
  var xy = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
  var o: VO;
  let q = xy[i];
  o.p = vec4(q, 0.0, 1.0);
  o.uv = vec2((q.x + 1.0) * 0.5, (1.0 - q.y) * 0.5);
  return o;
}
@fragment fn fs(v: VO) -> @location(0) vec4<f32> { return textureSample(t, s, v.uv); }
"#;

fn make_cpu_state(device: &wgpu::Device, format: wgpu::TextureFormat, w: u16, h: u16) -> CpuState {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pvc cpu frame"),
        size: wgpu::Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
        ],
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bgl,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) }],
    });
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()) });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&bgl)], immediate_size: 0 });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("pvc cpu blit"),
        layout: Some(&layout),
        vertex: wgpu::VertexState { module: &module, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
        fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(format.into())] }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    CpuState { ctx: vello_cpu::RenderContext::new(w, h), res: vello_cpu::Resources::new(), pm: vello_cpu::Pixmap::new(w, h), tex, pipeline, bind }
}

pub struct Gpu {
    renderer: Renderer,
    cpu: Option<CpuState>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    gl: Option<GlRenderer>,
    pub adapter_desc: String,
    pub backend: String,
    pub format: String,
    pub present_mode: String,
}

impl Gpu {
    pub fn new(display: winit::event_loop::OwnedDisplayHandle, window: Arc<Window>, renderer: Renderer) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::GL,
            ..wgpu::InstanceDescriptor::new_with_display_handle(Box::new(display))
        });
        let surface = instance.create_surface(window.clone()).map_err(|e| format!("create_surface: {e}"))?;
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions { compatible_surface: Some(&surface), ..Default::default() }))
            .map_err(|e| format!("request_adapter: {e}"))?;
        let info = adapter.get_info();
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("pvc-native"),
            required_features: wgpu::Features::empty(),
            // Tier 1 *code path* on Tier 0 (c)-class hardware: imposed WebGL2 downlevel limits (PLAN V2)
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            ..Default::default()
        }))
        .map_err(|e| format!("request_device: {e}"))?;
        let sz = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        // vello writes display-referred values: pick a non-sRGB 8-bit format
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| matches!(f, wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm))
            .ok_or_else(|| format!("no Rgba8Unorm/Bgra8Unorm surface format in {:?}", caps.formats))?;
        let present_mode = if caps.present_modes.contains(&wgpu::PresentMode::Immediate) { wgpu::PresentMode::Immediate } else { caps.present_modes[0] };
        let mut config = surface.get_default_config(&adapter, sz.width, sz.height).ok_or("surface unsupported by adapter")?;
        config.format = format;
        config.present_mode = present_mode;
        config.usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        surface.configure(&device, &config);
        Ok(Self {
            renderer,
            cpu: None,
            surface,
            device,
            queue,
            config,
            gl: None,
            adapter_desc: format!("{} | {:?} | driver={} {}", info.name, info.device_type, info.driver, info.driver_info),
            backend: format!("{:?}", info.backend),
            format: format!("{format:?}"),
            present_mode: format!("{present_mode:?}"),
        })
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        self.gl = None;
        self.cpu = None;
    }

    pub fn width(&self) -> u32 {
        self.config.width
    }
    pub fn height(&self) -> u32 {
        self.config.height
    }

    /// Render `dl` and present. Returns `render_ms` (surface acquire + vello_gpu render + submit + present).
    pub fn frame(&mut self, dl: &DisplayList) -> Result<f64, String> {
        let t = Instant::now();
        let (w, h) = dl.pixel_size();
        if (w as u32, h as u32) != (self.config.width, self.config.height) {
            return Err(format!("display list {w}x{h} != surface {}x{}", self.config.width, self.config.height));
        }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            other => return Err(format!("get_current_texture: {}", describe(&other))),
        };
        if self.renderer == Renderer::Cpu {
            if self.cpu.is_none() {
                self.cpu = Some(make_cpu_state(&self.device, self.config.format, w, h));
            }
            let st = self.cpu.as_mut().unwrap();
            st.ctx.reset();
            pvc_runtime::render::replay_cpu(dl, &mut st.ctx, &mut st.res);
            st.ctx.flush();
            st.ctx.render(&mut st.pm, &mut st.res);
            // page background is opaque, so premultiplied == straight here
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &st.tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                st.pm.data_as_u8_slice(),
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w as u32 * 4), rows_per_image: None },
                wgpu::Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
            );
            let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("pvc cpu blit") });
            {
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &view, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store } })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(&st.pipeline);
                pass.set_bind_group(0, &st.bind, &[]);
                pass.draw(0..3, 0..1);
            }
            self.queue.submit([enc.finish()]);
        } else {
            let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
            if self.gl.is_none() {
                self.gl = Some(GlRenderer::new(&self.device, self.config.format, w, h));
            }
            self.gl.as_mut().unwrap().render(dl, &self.device, &self.queue, &view);
        }
        self.queue.present(frame);
        Ok(t.elapsed().as_secs_f64() * 1e3)
    }

    /// Forced GPU completion (METHOD §2.2 completion_ms): issued after total_ms was sampled.
    pub fn wait_idle(&self) {
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
    }
}

fn describe(c: &wgpu::CurrentSurfaceTexture) -> &'static str {
    match c {
        wgpu::CurrentSurfaceTexture::Timeout => "Timeout",
        wgpu::CurrentSurfaceTexture::Occluded => "Occluded",
        wgpu::CurrentSurfaceTexture::Outdated => "Outdated",
        wgpu::CurrentSurfaceTexture::Lost => "Lost",
        wgpu::CurrentSurfaceTexture::Validation => "Validation",
        _ => "?",
    }
}
