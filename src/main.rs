mod args;
mod camera;
mod graphics;
mod input;
mod kernel;
mod math;
mod mesh;
mod model;
mod window;

use std::{collections::HashMap, sync::mpsc, time::Instant};

use camera::Camera;
use futures::executor::block_on;
use notify::Watcher as _;
use tracing::trace;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

use crate::{
    args::Args,
    graphics::{DrawConfig, Renderer},
    kernel::Shape as _,
    mesh::{HashVector, MeshMaker},
    model::Model,
    window::Window,
};

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let model = Model::new(args.model);

    let mut parameters = HashMap::new();
    for parameter in args.parameters {
        let mut parameter = parameter.splitn(2, "=");

        let key = parameter
            .next()
            .expect("model parameter: key not found")
            .to_owned();
        let value = parameter
            .next()
            .expect("model parameter: value not found")
            .to_owned();

        parameters.insert(key, value);
    }

    // TASK: Since we're loading the model before setting up the watcher below,
    //       there's a race condition, and a modification could be missed
    //       between those two events.
    //
    //       This can't be addressed with the current structure, since the
    //       watcher closure takes ownership of the model.
    let shape = model.load(&parameters)?;

    let (watcher_tx, watcher_rx) = mpsc::sync_channel(0);

    let watch_path = model.src_path();
    let mut watcher = notify::recommended_watcher(
        move |event: notify::Result<notify::Event>| {
            // TASK: Figure out when this error can happen, find a better way to
            //       handle it.
            let event = event.expect("Error handling watch event");

            //Various acceptable ModifyKind kinds. Varies across platforms (e.g. MacOs vs. Windows10)
            if let notify::EventKind::Modify(notify::event::ModifyKind::Any)
            | notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any,
            ))
            | notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )) = event.kind
            {
                let shape = match model.load(&parameters) {
                    Ok(shape) => shape,
                    Err(model::Error::Compile) => {
                        // TASK: Display error message on screen.
                        println!("Error compiling model");
                        return;
                    }
                    Err(err) => {
                        panic!("Error reloading model: {:?}", err);
                    }
                };

                // This will panic, if the other end is disconnected, which is
                // probably the result of a panic on that thread, or the
                // application is being shut down.
                //
                // Either way, not much we can do about it here, except maybe to
                // provide a better error message in the future.
                watcher_tx.send(shape).unwrap();
            }
        },
    )?;
    watcher.watch(&watch_path, notify::RecursiveMode::Recursive)?;

    let aabb = shape.bounding_volume();

    let tolerance = aabb.extents().min() / 1000.;
    let mut faces = shape.faces(tolerance);

    if let Some(path) = args.export {
        let mut mesh_maker = MeshMaker::new();

        let mut triangles = Vec::new();
        faces.triangles(tolerance, &mut triangles);

        for triangle in triangles {
            for vertex in triangle.vertices() {
                mesh_maker.push(HashVector::from(vertex));
            }
        }

        let vertices =
            mesh_maker.vertices().map(|vertex| vertex.into()).collect();

        let indices: Vec<_> = mesh_maker.indices().collect();
        let triangles = indices
            .chunks(3)
            .map(|triangle| {
                [
                    triangle[0] as usize,
                    triangle[1] as usize,
                    triangle[2] as usize,
                ]
            })
            .collect();

        let mesh = threemf::TriangleMesh {
            vertices,
            triangles,
        };

        threemf::write(path, &mesh)?;

        return Ok(());
    }

    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop);

    let mut previous_time = Instant::now();

    let mut input_handler = input::Handler::new(previous_time);
    let mut renderer = block_on(Renderer::new(&window))?;

    let mut triangles = Vec::new();
    faces.triangles(tolerance, &mut triangles);
    renderer.update_geometry((&triangles).into());

    let mut draw_config = DrawConfig::default();
    let mut camera = Camera::new(&aabb);

    event_loop.run(move |event, _, control_flow| {
        trace!("Handling event: {:?}", event);

        let mut actions = input::Actions::new();

        let now = Instant::now();

        match watcher_rx.try_recv() {
            Ok(shape) => {
                faces = shape.faces(tolerance);

                let mut triangles = Vec::new();
                faces.triangles(tolerance, &mut triangles);

                renderer.update_geometry((&triangles).into());
            }
            Err(mpsc::TryRecvError::Empty) => {
                // Nothing to receive from the channel. We don't care.
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                // The other end has disconnected. This is probably the result
                // of a panic on the other thread, or a program shutdown in
                // progress. In any case, not much we can do here.
                panic!();
            }
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                renderer.handle_resize(size);
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                input_handler.handle_keyboard_input(input, &mut actions);
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                input_handler.handle_cursor_moved(
                    position,
                    &mut camera,
                    &window,
                );
            }
            Event::WindowEvent {
                event: WindowEvent::MouseInput { state, button, .. },
                ..
            } => {
                let focus_point = camera.focus_point(
                    &window,
                    input_handler.cursor(),
                    &faces,
                    tolerance,
                );

                input_handler.handle_mouse_input(button, state, focus_point);
            }
            Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => {
                input_handler.handle_mouse_wheel(delta, now);
            }
            Event::MainEventsCleared => {
                let delta_t = now.duration_since(previous_time);
                previous_time = now;

                input_handler.update(
                    delta_t.as_secs_f64(),
                    now,
                    &mut camera,
                    &window,
                    &faces,
                    tolerance,
                );

                window.inner().request_redraw();
            }
            Event::RedrawRequested(_) => {
                camera.update_planes(&aabb);

                match renderer.draw(&camera, &draw_config) {
                    Ok(()) => {}
                    Err(err) => {
                        panic!("Draw error: {}", err);
                    }
                }
            }
            _ => {}
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
    });
}
