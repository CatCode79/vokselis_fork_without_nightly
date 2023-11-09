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

mod raycast;

use vokselis::{run, Camera, Context, Demo, VolumeTexture};
use raycast::RaycastPipeline;

use wgpu::StoreOp;
use winit::{dpi::LogicalSize, event_loop::EventLoopBuilder, window::WindowBuilder};

use std::path::PathBuf;

struct Bonsai {
    volume_texture: VolumeTexture,
    pipeline: RaycastPipeline,
}

impl Demo for Bonsai {
    fn init(ctx: &mut Context) -> Self {
        let volume_texture = VolumeTexture::new(&ctx.device, &ctx.queue);
        let pipeline = {
            let module_desc = wgpu::include_wgsl!("../../../shaders/raycast_naive.wgsl");
            RaycastPipeline::new(&ctx.device, module_desc)
        };
        Self {
            volume_texture,
            pipeline,
        }
    }

    fn render(&mut self, ctx: &Context) {
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Volume Encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Volume Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &ctx.render_backbuffer.texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                ..Default::default()
            });

            self.pipeline.record(
                &mut rpass,
                &ctx.global_uniform_binding,
                &ctx.camera_binding,
                &self.volume_texture.bind_group,
            );
        }

        ctx.queue.submit(Some(encoder.finish()));
    }
}

fn main() -> Result<(), String> {
    let event_loop = EventLoopBuilder::<(PathBuf, wgpu::ShaderModule)>::with_user_event().build().map_err(|e| e.to_string())?;
    let window = WindowBuilder::new()
        .with_title("Vokselis")
        .with_inner_size(LogicalSize::new(1280, 720))
        .build(&event_loop)
        .map_err(|e| e.to_string())?;
    let window_size = window.inner_size();

    let camera = Camera::new(
        1.,
        0.5,
        1.,
        (0.5, 0.5, 0.5).into(),
        window_size.width as f32 / window_size.height as f32,
    );
    run::<Bonsai>(event_loop, window, Some(camera))
}
