//! Model viewer initialization and event processing
//!
//! Provides the functionality to create a window and perform basic viewing
//! with programmed models.

use std::{error, time::Instant};

use fj_host::Watcher;
use fj_operations::shape_processor::ShapeProcessor;
use fj_viewer::{
    camera::Camera,
    graphics::{self, DrawConfig, Renderer},
    input::{self, KeyState},
    screen::{Position, Screen as _, Size},
};
use futures::executor::block_on;
use tracing::{debug, trace, warn};
use winit::{
    dpi::PhysicalPosition,
    event::{
        ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta,
        VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
};

use crate::window::{self, Window};

/// Initializes a model viewer for a given model and enters its process loop.
pub fn run(
    mut watcher: Watcher,
    shape_processor: ShapeProcessor,
) -> Result<(), Error> {
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop)?;

    let mut previous_time = Instant::now();

    let mut input_handler = input::Handler::default();
    let mut renderer = block_on(Renderer::new(&window))?;

    let mut draw_config = DrawConfig::default();

    let mut shape = None;
    let mut camera = None;

    event_loop.run(move |event, _, control_flow| {
        trace!("Handling event: {:?}", event);

        let mut actions = input::Actions::new();

        let now = Instant::now();

        if let Some(new_shape) = watcher.receive() {
            match shape_processor.process(&new_shape) {
                Ok(new_shape) => {
                    renderer.update_geometry(
                        (&new_shape.mesh).into(),
                        (&new_shape.debug_info).into(),
                        new_shape.aabb,
                    );

                    if camera.is_none() {
                        camera = Some(Camera::new(&new_shape.aabb));
                    }

                    shape = Some(new_shape);
                }
                Err(err) => {
                    // Can be cleaned up, once `Report` is stable:
                    // https://doc.rust-lang.org/std/error/struct.Report.html

                    println!("Shape processing error: {}", err);

                    let mut current_err = &err as &dyn error::Error;
                    while let Some(err) = current_err.source() {
                        println!();
                        println!("Caused by:");
                        println!("    {}", err);

                        current_err = err;
                    }
                }
            }
        }

        //

        let mut egui_wants_exclusive = false;

        if let Event::WindowEvent {
            event: window_event,
            ..
        } = &event
        {
            //
            // Note: In theory we could/should check if `egui` wants "exclusive" use
            //       of this event here.
            //
            //       But with the current integration with Fornjot we're kinda blurring
            //       the lines between "app" and "platform", so for the moment we pass
            //       every event to both `egui` & Fornjot.
            //
            //       The primary visible impact of this currently is that if you drag
            //       a title bar that overlaps the model then both the model & window
            //       get moved.
            //
            // DONE: Revisit this. (Added `egui_wants_exclusive` which actually seems
            //       to work as expected now.)
            //
            // TODO: Encapsulate the egui state/context access better.
            //
            egui_wants_exclusive = renderer
                .egui
                .winit_state
                .on_event(&renderer.egui.context, window_event);
        }

        //

        let event = match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
                None
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                let size = Size {
                    width: size.width,
                    height: size.height,
                };
                renderer.handle_resize(size);

                None
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(virtual_key_code),
                                ..
                            },
                        ..
                    },
                ..
            } => match virtual_key_code {
                VirtualKeyCode::Escape => Some(input::Event::Key(
                    input::Key::Escape,
                    KeyState::Pressed,
                )),
                VirtualKeyCode::Key1 => {
                    Some(input::Event::Key(input::Key::Key1, KeyState::Pressed))
                }
                VirtualKeyCode::Key2 => {
                    Some(input::Event::Key(input::Key::Key2, KeyState::Pressed))
                }
                VirtualKeyCode::Key3 => {
                    Some(input::Event::Key(input::Key::Key3, KeyState::Pressed))
                }

                _ => None,
            },
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                let position = Position {
                    x: position.x,
                    y: position.y,
                };
                Some(input::Event::CursorMoved(position))
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let state = match state {
                    ElementState::Pressed => input::KeyState::Pressed,
                    ElementState::Released => input::KeyState::Released,
                };

                match button {
                    MouseButton::Left => {
                        Some(input::Event::Key(input::Key::MouseLeft, state))
                    }
                    MouseButton::Right => {
                        Some(input::Event::Key(input::Key::MouseRight, state))
                    }
                    _ => None,
                }
            }
            Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => Some(input::Event::Scroll(match delta {
                MouseScrollDelta::LineDelta(_, y) => {
                    input::MouseScrollDelta::Line(y as f64)
                }
                MouseScrollDelta::PixelDelta(PhysicalPosition {
                    y, ..
                }) => input::MouseScrollDelta::Pixel(y),
            })),
            Event::MainEventsCleared => {
                let delta_t = now.duration_since(previous_time);
                previous_time = now;

                if let (Some(shape), Some(camera)) = (&shape, &mut camera) {
                    input_handler.update(
                        delta_t.as_secs_f64(),
                        camera,
                        window.size(),
                        &shape.mesh,
                    );
                }

                window.window().request_redraw();

                None
            }
            Event::RedrawRequested(_) => {
                if let (Some(shape), Some(camera)) = (&shape, &mut camera) {
                    camera.update_planes(&shape.aabb);

                    if let Err(err) = renderer.draw(
                        camera,
                        &mut draw_config,
                        &window.window(),
                        &mut watcher.parameters,
                    ) {
                        warn!("Draw error: {}", err);
                    }

                    if renderer.egui.options.reload_requested {
                        //
                        // NOTE: It's *VERY* important that `renderer.egui.options.reload_requested`
                        //       gets reset to `false` here immediately.
                        //
                        //       This ensures only one (at most) reload/refresh thread
                        //       is spawned per frame.
                        //
                        //       Yes, it *would* be better to have this throttling
                        //       implemented elsewhere/more robustly[0].
                        //
                        //       [0] Hey, I only segfaulted my Window Manager *once*,
                        //           you know! (See comment in `egui`-related `draw()`
                        //           code for more details.)
                        //
                        //           Actually, it now occurs to me that (since part of the
                        //           issue in that situation was that there hadn't been a
                        //           frame redraw processed) a more robust approach would
                        //           be to move this processing of state changes to be
                        //           immediately after the frame draw.
                        //
                        //           DONE: Move this code as suggested above.
                        //
                        // TODO: Handle this better.
                        //
                        renderer.egui.options.reload_requested = false;

                        debug!(
                            "pre-`refresh()` parameters: {:?}",
                            &watcher.parameters.0
                        );

                        watcher.refresh();

                        debug!(
                            "post-`refresh()` parameters: {:?}",
                            &watcher.parameters.0
                        );
                    }
                }

                None
            }
            _ => None,
        };

        if !egui_wants_exclusive {
            if let (Some(event), Some(shape), Some(camera)) =
                (event, &shape, &mut camera)
            {
                input_handler.handle_event(
                    event,
                    window.size(),
                    &shape.mesh,
                    camera,
                    &mut actions,
                );
            }
        }

        if actions.exit {
            *control_flow = ControlFlow::Exit;
        }
        if actions.toggle_model {
            draw_config.draw_model = !draw_config.draw_model;
        }
        if actions.toggle_mesh {
            draw_config.draw_mesh = !draw_config.draw_mesh;
        }
        if actions.toggle_debug {
            draw_config.draw_debug = !draw_config.draw_debug;
        }
    });
}

/// Error in main loop
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error initializing window
    #[error("Error initializing window")]
    WindowInit(#[from] window::Error),

    /// Error initializing graphics
    #[error("Error initializing graphics")]
    GraphicsInit(#[from] graphics::InitError),
}
