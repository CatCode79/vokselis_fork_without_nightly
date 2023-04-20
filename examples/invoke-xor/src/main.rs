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
mod xor_compute;

use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use color_eyre::eyre::Result;
use wgpu::util::DeviceExt as _;
use winit::{dpi::LogicalSize, event_loop::EventLoopBuilder, window::WindowBuilder};

use invoke_selis::{dispatch_optimal, run, Camera, Context, Demo, HdrBackBuffer};

const TILE_SIZE: u32 = 256;

#[derive(Debug)]
enum Mode {
    SinglePass,
    Tile,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Offset {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TimestampData {
    start: u64,
    end: u64,
}

struct Xor {
    xor_texture: xor_compute::XorCompute,
    raycast_single: raycast::RaycastPipeline,
    raycast_tile: raycast::RaycastPipeline,
    mode: Mode,

    offset_buffer_bind_group: wgpu::BindGroup,
    buffer_len: usize,
    aligned_offset: u32,

    timestamp: wgpu::QuerySet,
    timestamp_period: f32,
    timestamp_buffer: wgpu::Buffer,
}

impl Demo for Xor {
    fn init(ctx: &mut Context) -> Self {
        let raycast_single = {
            let shader_module_desc = wgpu::include_wgsl!("../../../shaders/raycast_compute.wgsl");
            raycast::RaycastPipeline::new_with_module(&ctx.device, shader_module_desc, "single")
        };

        let raycast_tile = {
            let shader_module_desc = wgpu::include_wgsl!("../../../shaders/raycast_compute.wgsl");
            raycast::RaycastPipeline::new_with_module(&ctx.device, shader_module_desc, "tile")
        };

        let xor_texture = {
            let shader_module_desc = wgpu::include_wgsl!("../../../shaders/xor.wgsl");
            xor_compute::XorCompute::new_with_module(&ctx.device, shader_module_desc)
        };

        let padding = {
            let min_align = ctx.limits.min_storage_buffer_offset_alignment;
            (min_align - std::mem::size_of::<Offset>() as u32 % min_align) % min_align
        };
        let offsets = {
            let mut res = vec![];
            let (w, h) = HdrBackBuffer::DEFAULT_RESOLUTION;
            for y in 0..((h / TILE_SIZE) + 1) {
                for x in 0..((w / TILE_SIZE) + 1) {
                    res.extend(bytemuck::bytes_of(&Offset {
                        x: (x * TILE_SIZE) as f32,
                        y: (y * TILE_SIZE) as f32,
                    }));
                    res.extend(std::iter::repeat(0).take(padding as _));
                }
            }
            res
        };
        let aligned_offset = std::mem::size_of::<Offset>() as u32 + padding;
        let buffer_len = offsets.len() / aligned_offset as usize;

        let offset_buffer_bind_group = {
            let offset_buffer = ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Offsets Buffer"),
                    contents: bytemuck::cast_slice(&offsets),
                    usage: wgpu::BufferUsages::STORAGE,
                });
            let offset_buffer_bind_group_layout = ctx
                .device
                .create_bind_group_layout(&raycast::RaycastPipeline::OFFSET_BUFFER_DESC);
            let offset_buffer_bind_group_desc = wgpu::BindGroupDescriptor {
                label: Some("Offset Buffer Bind Group"),
                layout: &offset_buffer_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &offset_buffer,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<Offset>() as _),
                    }),
                }],
            };
            ctx.device.create_bind_group(&offset_buffer_bind_group_desc)
        };

        let timestamp = ctx.device.create_query_set(&wgpu::QuerySetDescriptor {
            label: None,
            count: 2,
            ty: wgpu::QueryType::Timestamp,
        });
        let timestamp_period = ctx.queue.get_timestamp_period();
        let timestamp_buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Query Buffer"),
            size: std::mem::size_of::<TimestampData>() as _,
            usage: wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::MAP_READ
                | wgpu::BufferUsages::QUERY_RESOLVE,
            mapped_at_creation: false,
        });

        println!("Change rendering mode on F1");

        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("XOR Update encoder"),
            });

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("XOR Update Pass"),
        });
        xor_texture.record(&mut cpass, &ctx.global_uniform_binding);
        drop(cpass);
        ctx.queue.submit(Some(encoder.finish()));

        Self {
            xor_texture,
            raycast_single,
            raycast_tile,
            mode: Mode::Tile,

            aligned_offset,
            offset_buffer_bind_group,
            buffer_len,

            timestamp,
            timestamp_period,
            timestamp_buffer,
        }
    }

    fn update(&mut self, ctx: &mut Context) {
        if ctx.global_uniform.frame % 100 == 0 {
            let _ = self
                .timestamp_buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, |_| ());
            {
                ctx.device.poll(wgpu::Maintain::Wait);
                let timestamp_view = self
                    .timestamp_buffer
                    .slice(..std::mem::size_of::<TimestampData>() as wgpu::BufferAddress)
                    .get_mapped_range();
                let timestamp_data: &TimestampData = bytemuck::from_bytes(&*timestamp_view);
                let nanoseconds =
                    (timestamp_data.end - timestamp_data.start) as f32 * self.timestamp_period;
                let time_period = std::time::Duration::from_nanos(nanoseconds as _);
                eprintln!(
                    "Time on raycast shader: {:?} ({:?})",
                    time_period, self.mode
                );
            }
            self.timestamp_buffer.unmap();
        }
    }

    fn update_input(&mut self, event: winit::event::WindowEvent<'_>) {
        match event {
            winit::event::WindowEvent::KeyboardInput {
                input:
                    winit::event::KeyboardInput {
                        state: winit::event::ElementState::Pressed,
                        virtual_keycode: Some(winit::event::VirtualKeyCode::F1),
                        ..
                    },
                ..
            } => {
                self.mode = match self.mode {
                    Mode::SinglePass => Mode::Tile,
                    Mode::Tile => Mode::SinglePass,
                };
                println!("Switched to: {:?}", self.mode);
            }
            _ => {}
        }
    }

    fn render(&mut self, ctx: &Context) {
        let mut encoder = ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Volume Encoder"),
            });

        encoder.write_timestamp(&self.timestamp, 0);

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Raycast Pass"),
        });

        match self.mode {
            Mode::SinglePass => {
                cpass.set_pipeline(&self.raycast_single.pipeline);

                cpass.set_bind_group(0, &ctx.global_uniform_binding.binding, &[]);
                cpass.set_bind_group(1, &ctx.camera_binding.bind_group, &[]);
                cpass.set_bind_group(2, &self.xor_texture.storage_bind_group, &[]);
                cpass.set_bind_group(3, &ctx.render_backbuffer.storage_bind_group, &[]);
                cpass.set_bind_group(4, &self.offset_buffer_bind_group, &[0]);
                let (width, height) = HdrBackBuffer::DEFAULT_RESOLUTION;
                cpass.dispatch_workgroups(
                    dispatch_optimal(width, 8),
                    dispatch_optimal(height, 8),
                    1,
                );
            }
            Mode::Tile => {
                cpass.set_pipeline(&self.raycast_tile.pipeline);

                cpass.set_bind_group(0, &ctx.global_uniform_binding.binding, &[]);
                cpass.set_bind_group(1, &ctx.camera_binding.bind_group, &[]);
                cpass.set_bind_group(2, &self.xor_texture.storage_bind_group, &[]);
                cpass.set_bind_group(3, &ctx.render_backbuffer.storage_bind_group, &[]);
                for offset in 0..self.buffer_len {
                    cpass.set_bind_group(
                        4,
                        &self.offset_buffer_bind_group,
                        &[offset as u32 * self.aligned_offset],
                    );
                    cpass.dispatch_workgroups(
                        dispatch_optimal(TILE_SIZE, 16),
                        dispatch_optimal(TILE_SIZE, 16),
                        1,
                    );
                }
            }
        }
        drop(cpass);

        encoder.write_timestamp(&self.timestamp, 1);
        encoder.resolve_query_set(&self.timestamp, 0..2, &self.timestamp_buffer, 0);

        ctx.queue.submit(Some(encoder.finish()));
    }
}

fn main() -> Result<()> {
    let event_loop = EventLoopBuilder::<(PathBuf, wgpu::ShaderModule)>::with_user_event().build();
    let window = WindowBuilder::new()
        .with_title("Vokselis")
        .with_inner_size(LogicalSize::new(1280, 720))
        .build(&event_loop)?;
    let window_size = window.inner_size();

    let camera = Camera::new(
        3.,
        -0.5,
        1.,
        (0., 0., 0.).into(),
        window_size.width as f32 / window_size.height as f32,
    );
    run::<Xor>(event_loop, window, Some(camera))
}
