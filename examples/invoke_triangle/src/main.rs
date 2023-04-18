#![warn(
absolute_paths_not_starting_with_crate,
//box_pointers,
elided_lifetimes_in_paths,
explicit_outlives_requirements,
keyword_idents,
let_underscore_drop,
macro_use_extern_crate,
meta_variable_misuse,
missing_abi,
//missing_copy_implementations,
//missing_debug_implementations,
//missing_docs,
non_ascii_idents,
noop_method_call,
pointer_structural_match,
rust_2021_incompatible_closure_captures,
rust_2021_incompatible_or_patterns,
rust_2021_prefixes_incompatible_syntax,
rust_2021_prelude_collisions,
single_use_lifetimes,
trivial_casts,
trivial_numeric_casts,
unreachable_pub,
//unsafe_code,
unsafe_op_in_unsafe_fn,
unstable_features,
unused_crate_dependencies,
unused_extern_crates,
unused_import_braces,
unused_lifetimes,
unused_macro_rules,
unused_qualifications,
//unused_results,
unused_tuple_struct_fields,
variant_size_differences,
clippy::cargo,
clippy::complexity,
clippy::correctness,
clippy::nursery,
clippy::pedantic,
clippy::perf,
clippy::restriction,
clippy::style,
clippy::suspicious,
)]

use std::path::PathBuf;

use color_eyre::eyre::Result;
use winit::{dpi::LogicalSize, event_loop::EventLoopBuilder, window::WindowBuilder};

use invoke_selis::{run, CameraBinding, Context, Demo, Uniform};

pub struct BasicPipeline {
    pub pipeline: wgpu::RenderPipeline,
    _surface_format: wgpu::TextureFormat,
}

impl BasicPipeline {
    pub fn new_with_module(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        module_desc: wgpu::ShaderModuleDescriptor<'_>,
    ) -> Self {
        let global_bind_group_layout = device.create_bind_group_layout(&Uniform::DESC);
        let camera_bind_group_layout = device.create_bind_group_layout(&CameraBinding::DESC);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Screen Pass Layout"),
            bind_group_layouts: &[&global_bind_group_layout, &camera_bind_group_layout],
            push_constant_ranges: &[],
        });
        let shader = device.create_shader_module(module_desc);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render with Camera Pipeline"),
            layout: Some(&layout),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(surface_format.into())],
            }),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        Self {
            pipeline,
            _surface_format: surface_format,
        }
    }
}

struct BasicTrig {
    pipeline: BasicPipeline,
}

impl Demo for BasicTrig {
    fn init(ctx: &mut Context) -> Self {
        let pipeline = BasicPipeline::new_with_module(
            &ctx.device,
            ctx.render_backbuffer.format(),
            wgpu::include_wgsl!("../../../shaders/shader_with_camera.wgsl"),
        );
        Self { pipeline }
    }

    fn render(&mut self, ctx: &Context) {
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Trig Encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Trig Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &ctx.render_backbuffer.texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.pipeline.pipeline);
            rpass.set_bind_group(0, &ctx.global_uniform_binding.binding, &[]);
            rpass.set_bind_group(1, &ctx.camera_binding.bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        ctx.queue.submit(Some(encoder.finish()));
    }
}

fn main() -> Result<()> {
    let event_loop = EventLoopBuilder::<(PathBuf, wgpu::ShaderModule)>::with_user_event().build();
    let window = WindowBuilder::new()
        .with_title("Vokselis")
        .with_inner_size(LogicalSize::new(1280, 720))
        .build(&event_loop)?;

    run::<BasicTrig>(event_loop, window, None)
}