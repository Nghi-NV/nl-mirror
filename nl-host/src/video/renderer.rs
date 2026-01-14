use anyhow::{anyhow, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::dpi::LogicalSize;
use winit::window::Window;

use crate::core::FrameData;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct AspectRatioUniform {
    scale: [f32; 2],
    _padding: [f32; 2], // Align to 16 bytes
}

pub struct MirrorRenderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    // YUV textures for GPU-based conversion
    y_texture: wgpu::Texture,
    u_texture: wgpu::Texture,
    v_texture: wgpu::Texture,
    texture_bind_group: wgpu::BindGroup,
    aspect_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    frame_width: u32,
    frame_height: u32,
}

impl MirrorRenderer {
    pub fn new(window: Arc<Window>, width: u32, height: u32) -> Result<Self> {
        println!(
            "Creating wgpu renderer for frame: {}x{} (GPU YUV->RGB)",
            width, height
        );

        // Scale window to fit on screen (max 800 logical height)
        let max_height = 800.0_f64;
        let scale = if (height as f64) > max_height {
            max_height / (height as f64)
        } else {
            1.0
        };

        let logical_w = (width as f64 * scale) as f64;
        let logical_h = (height as f64 * scale) as f64;

        // Resize existing window
        let _ = window.request_inner_size(LogicalSize::new(logical_w, logical_h));

        let physical_size = window.inner_size();

        println!(
            "Window: {}x{} logical -> {}x{} physical",
            logical_w as u32, logical_h as u32, physical_size.width, physical_size.height
        );

        // Initialize wgpu
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| anyhow!("Failed to create surface: {}", e))?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| anyhow!("Failed to get adapter"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ))
        .map_err(|e| anyhow!("Failed to get device: {}", e))?;

        let surface_caps = surface.get_capabilities(&adapter);
        // Use NON-sRGB format to avoid double gamma correction
        // Video YUV data is already gamma-corrected, so we don't want sRGB to apply gamma again
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| !f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        // Use Mailbox for lower latency
        let present_mode = if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::Mailbox)
        {
            println!("Using PresentMode::Mailbox for low latency");
            wgpu::PresentMode::Mailbox
        } else if surface_caps
            .present_modes
            .contains(&wgpu::PresentMode::Immediate)
        {
            println!("Using PresentMode::Immediate");
            wgpu::PresentMode::Immediate
        } else {
            println!("Using PresentMode::Fifo (vsync)");
            wgpu::PresentMode::Fifo
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: physical_size.width.max(1),
            height: physical_size.height.max(1),
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 0,
        };
        surface.configure(&device, &config);

        // Create Y texture (full resolution, single channel)
        let y_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Y Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        // Create U and V textures (half resolution, single channel each)
        let uv_width = width / 2;
        let uv_height = height / 2;

        let u_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("U Texture"),
            size: wgpu::Extent3d {
                width: uv_width,
                height: uv_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let v_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("V Texture"),
            size: wgpu::Extent3d {
                width: uv_width,
                height: uv_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let y_view = y_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let u_view = u_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let v_view = v_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create sampler with LINEAR filtering for smooth UV upscaling
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Calculate initial aspect ratio scale
        let aspect_uniform =
            Self::calculate_aspect_scale(width, height, physical_size.width, physical_size.height);

        let aspect_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Aspect Ratio Buffer"),
            contents: bytemuck::cast_slice(&[aspect_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Bind group layout for YUV textures
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("YUV Bind Group Layout"),
            entries: &[
                // Y texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // U texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // V texture
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                // Aspect ratio uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("YUV Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&y_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&u_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&v_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: aspect_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("YUV Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        println!("wgpu renderer initialized (GPU YUV->RGB conversion enabled)");

        Ok(Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            y_texture,
            u_texture,
            v_texture,
            texture_bind_group,
            aspect_buffer,
            bind_group_layout,
            sampler,
            frame_width: width,
            frame_height: height,
        })
    }

    fn calculate_aspect_scale(
        frame_w: u32,
        frame_h: u32,
        surface_w: u32,
        surface_h: u32,
    ) -> AspectRatioUniform {
        let frame_aspect = frame_w as f32 / frame_h as f32;
        let surface_aspect = surface_w as f32 / surface_h as f32;

        let (scale_x, scale_y) = if frame_aspect > surface_aspect {
            (1.0, surface_aspect / frame_aspect)
        } else {
            (frame_aspect / surface_aspect, 1.0)
        };

        AspectRatioUniform {
            scale: [scale_x, scale_y],
            _padding: [0.0, 0.0],
        }
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) -> Result<()> {
        if width > 0 && height > 0 {
            println!("resize_surface: {}x{}", width, height);
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);

            let aspect_uniform =
                Self::calculate_aspect_scale(self.frame_width, self.frame_height, width, height);
            self.queue.write_buffer(
                &self.aspect_buffer,
                0,
                bytemuck::cast_slice(&[aspect_uniform]),
            );
        }
        Ok(())
    }

    /// Render a frame from YUV data (GPU does the conversion)
    pub fn render_yuv_frame(&mut self, frame: &FrameData) -> Result<()> {
        let total_start = std::time::Instant::now();

        // Upload Y plane
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.y_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &frame.y_plane,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(frame.y_stride as u32),
                rows_per_image: Some(frame.height),
            },
            wgpu::Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
        );

        // Upload U plane
        let uv_width = frame.width / 2;
        let uv_height = frame.height / 2;

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.u_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &frame.u_plane,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(frame.uv_stride as u32),
                rows_per_image: Some(uv_height),
            },
            wgpu::Extent3d {
                width: uv_width,
                height: uv_height,
                depth_or_array_layers: 1,
            },
        );

        // Upload V plane
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.v_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &frame.v_plane,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(frame.uv_stride as u32),
                rows_per_image: Some(uv_height),
            },
            wgpu::Extent3d {
                width: uv_width,
                height: uv_height,
                depth_or_array_layers: 1,
            },
        );

        let upload_time = total_start.elapsed();

        let get_texture_start = std::time::Instant::now();
        let output = match self.surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Timeout) => {
                eprintln!("[REN] ERROR: get_current_texture timeout");
                return Err(anyhow!("Surface timeout"));
            }
            Err(wgpu::SurfaceError::Outdated) => {
                eprintln!("[REN] WARNING: Surface outdated, reconfiguring...");
                self.surface.configure(&self.device, &self.config);
                return Err(anyhow!("Surface outdated, skipped frame"));
            }
            Err(wgpu::SurfaceError::Lost) => {
                eprintln!("[REN] ERROR: Surface lost");
                return Err(anyhow!("Surface lost"));
            }
            Err(e) => {
                eprintln!("[REN] ERROR: get_current_texture failed: {:?}", e);
                return Err(anyhow!("Failed to get surface texture: {:?}", e));
            }
        };
        let get_texture_time = get_texture_start.elapsed();

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        let submit_start = std::time::Instant::now();
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        let submit_time = submit_start.elapsed();
        let total_time = total_start.elapsed();

        if total_time.as_millis() > 100 {
            println!(
                "WARNING: render_yuv_frame total time: {}ms (upload: {}ms, get_texture: {}ms, submit: {}ms)",
                total_time.as_millis(),
                upload_time.as_millis(),
                get_texture_time.as_millis(),
                submit_time.as_millis()
            );
        }

        Ok(())
    }
}
