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

pub mod camera;
pub mod context;
mod utils;

pub use camera::{Camera, CameraBinding};
pub use context::{Context, GlobalUniformBinding, HdrBackBuffer, Uniform, VolumeTexture};
pub use utils::{dispatch_optimal, NonZeroSized};

use pollster::FutureExt;
use utils::{frame_counter::FrameCounter, input::Input};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{
        DeviceEvent, ElementState, Event, KeyEvent, MouseScrollDelta, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    keyboard::Key,
    window::Window,
};

use std::path::PathBuf;
use winit::keyboard::NamedKey;

pub trait Demo: 'static + Sized {
    fn init(ctx: &mut Context) -> Self;
    fn resize(&mut self, _: &wgpu::Device, _: &wgpu::Queue, _: &wgpu::SurfaceConfiguration) {}
    fn update(&mut self, _: &mut Context) {}
    fn update_input(&mut self, _: WindowEvent) {}
    fn render(&mut self, _: &Context) {}
}

pub fn run<D: Demo>(
    event_loop: EventLoop<(PathBuf, wgpu::ShaderModule)>,
    window: Window,
    camera: Option<Camera>,
) -> Result<(), String> {
    env_logger::init();

    let mut context = Context::new(&window, camera).block_on()?;

    let mut frame_counter = FrameCounter::new();
    let mut input = Input::new();

    let mut mouse_dragged = false;
    let rotate_speed = 0.0025;
    let zoom_speed = 0.002;

    let mut demo = D::init(&mut context);

    let mut main_window_focused = false;
    event_loop.run(move |event, target| {
        target.set_control_flow(ControlFlow::Wait);

        match event {
            Event::AboutToWait => {
                context.update(&frame_counter, &input);
                demo.update(&mut context);
                window.request_redraw();
            },

            Event::WindowEvent {
                event: window_event, window_id, ..
            } if window.id() == window_id => {
                input.update(&window_event, &window);

                match window_event {
                    WindowEvent::Focused(focused) => main_window_focused = focused,

                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                logical_key: Key::Named(NamedKey::Escape),
                                ..
                            },
                        ..
                    } => target.exit(),

                    WindowEvent::RedrawRequested => {
                        frame_counter.record();

                        demo.render(&context);

                        match context.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => {
                                context.resize(context.width, context.height);
                                window.request_redraw();
                            }
                            Err(wgpu::SurfaceError::OutOfMemory) => target.exit(),
                            Err(e) => {
                                eprintln!("{:?}", e);
                                window.request_redraw();
                            }
                        }
                    }

                    WindowEvent::Resized(PhysicalSize { width, height }) => {
                        if width != 0 && height != 0 {
                            context.resize(width, height);
                            demo.resize(&context.device, &context.queue, &context.surface_config);
                        }
                    }

                    _ => {}
                }
                demo.update_input(window_event);
            }

            Event::DeviceEvent { ref event, .. } if main_window_focused => match event {
                DeviceEvent::Button {
                    #[cfg(target_os = "macos")]
                        button: 0,
                    #[cfg(not(target_os = "macos"))]
                        button: 1,

                    state: statee,
                } => {
                    let is_pressed = *statee == ElementState::Pressed;
                    mouse_dragged = is_pressed;
                }
                DeviceEvent::MouseWheel { delta, .. } => {
                    let scroll_amount = -match delta {
                        MouseScrollDelta::LineDelta(_, scroll) => scroll * 1.0,
                        MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => {
                            *scroll as f32
                        }
                    };
                    context.camera.add_zoom(scroll_amount * zoom_speed);
                }
                DeviceEvent::MouseMotion { delta } => {
                    if mouse_dragged {
                        context.camera.add_yaw(-delta.0 as f32 * rotate_speed);
                        context.camera.add_pitch(delta.1 as f32 * rotate_speed);
                    }
                }
                _ => (),
            },

            _ => {}
        }
    }).map_err(|e| e.to_string())
}
