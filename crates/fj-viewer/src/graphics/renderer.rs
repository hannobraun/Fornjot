use std::{io, mem::size_of};

use fj_math::{Aabb, Point};
use thiserror::Error;
use tracing::debug;
use wgpu::util::DeviceExt as _;
use wgpu_glyph::ab_glyph::InvalidFont;

use crate::{
    camera::Camera,
    screen::{Screen, Size},
};

use super::{
    config_ui::ConfigUi, draw_config::DrawConfig, drawables::Drawables,
    geometries::Geometries, pipelines::Pipelines, transform::Transform,
    uniforms::Uniforms, vertices::Vertices, DEPTH_FORMAT,
};

#[derive(Default)]
struct EguiOptionsState {
    show_trace: bool,
    show_layout_debug_on_hover: bool,
    show_debug_text_example: bool,
    show_original_ui: bool,
    show_settings_ui: bool,
    show_inspection_ui: bool,
}

pub struct EguiState {
    pub winit_state: egui_winit::State,
    pub context: egui::Context,
    rpass: egui_wgpu::renderer::RenderPass,
    options: EguiOptionsState,
}

impl std::fmt::Debug for EguiState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EguiState {}")
    }
}

/// Graphics rendering state and target abstraction
#[derive(Debug)]
pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,

    surface_config: wgpu::SurfaceConfiguration,
    depth_view: wgpu::TextureView,

    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,

    geometries: Geometries,
    pipelines: Pipelines,

    config_ui: ConfigUi,

    /// State required for integration with `egui`.
    pub egui: EguiState,
}

impl Renderer {
    /// Returns a new `Renderer`.
    pub async fn new(
        screen: &impl Screen<Window = egui_winit::winit::window::Window>,
    ) -> Result<Self, InitError> {
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

        //
        // NOTE: The implementation of the integration with `egui` is
        //       likely to need to change "significantly"[0] depending
        //       on what architecture approach is chosen going
        //       forward.
        //
        //       The current implementation is somewhat complicated by
        //       virtue of "sitting somewhere in the middle" in
        //       relation to being neither a standalone integration
        //       nor fully using `egui` as a framework.
        //
        //       This is a result of a combination of the current
        //       integration being "proof of concept" level; and, using
        //       `egui-winit` & `egui-wgpu` which are both relatively
        //       new additions to the core `egui` ecosystem.
        //
        //       It is recommended to read the following for additional
        //       helpful context for choosing an architecture:
        //
        //         * <https://github.com/emilk/egui/blob/eeae485629fca24a81a7251739460b671e1420f7/README.md#what-is-the-difference-between-egui-and-eframe>
        //
        //         * <https://github.com/emilk/egui/blob/eeae485629fca24a81a7251739460b671e1420f7/README.md#how-do-i-render-3d-stuff-in-an-egui-area>
        //
        //       [0] By way of specific example, the recent addition
        //           of Android support lead to considerable API
        //           change related to `wgpu` & `winit`, see:
        //
        //             * <https://github.com/emilk/egui/commit/a5076d4cc491536b07b16dced1772c7b6bf7cc29>
        //

        //
        // NOTE: If at some point you use `Painter` or similar and you
        //       get this error:
        //
        //         `VK_ERROR_NATIVE_WINDOW_IN_USE_KHR`
        //
        //       and/or:
        //
        //         `wgpu_core::device: surface configuration failed: Native window is in use`
        //
        //       it's *probably(?)* because the swapchain has already
        //       been created for the window (e.g. by an integration)
        //       and *not* because of a regression of this issue
        //       (probably):
        //
        //         <https://github.com/gfx-rs/wgpu/issues/1492>
        //
        //       Don't ask me how I know.
        //

        let egui_winit_state = egui_winit::State::new(4096, screen.window());
        let egui_context = egui::Context::default();

        // This is sound, as `window` is an object to create a surface upon.
        let surface = unsafe { instance.create_surface(screen.window()) };

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or(InitError::RequestAdapter)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    // Don't just blindly assume that we can request this
                    // feature. If it isn't available, that might cause a panic,
                    // or an error to be returned here.
                    //
                    // See this issue:
                    // https://github.com/hannobraun/fornjot/issues/33
                    features: wgpu::Features::POLYGON_MODE_LINE,
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let color_format = surface
            .get_preferred_format(&adapter)
            .expect("Error determining preferred color format");

        let Size { width, height } = screen.size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: color_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        surface.configure(&device, &surface_config);

        let depth_view = Self::create_depth_buffer(&device, &surface_config);

        let uniform_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&[Uniforms::default()]),
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST,
            });
        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(size_of::<
                            Uniforms,
                        >(
                        )
                            as u64),
                    },
                    count: None,
                }],
                label: None,
            });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &uniform_buffer,
                    offset: 0,
                    size: None,
                }),
            }],
            label: None,
        });

        let geometries = Geometries::new(
            &device,
            &Vertices::empty(),
            &Vertices::empty(),
            Aabb {
                min: Point::from([0.0, 0.0, 0.0]),
                max: Point::from([0.0, 0.0, 0.0]),
            },
        );
        let pipelines =
            Pipelines::new(&device, &bind_group_layout, color_format);

        let config_ui = ConfigUi::new(&device, color_format)?;

        //
        // Note: We need to hold on to this otherwise (from my memory)
        //       it causes the egui font texture to get dropped after
        //       drawing one frame.
        //
        //       This then results in an `egui_wgpu_backend` error of
        //       `BackendError::Internal` with message:
        //
        //           "Texture 0 used but not live"
        //
        //       See also: <https://github.com/hasenbanck/egui_wgpu_backend/blob/b2d3e7967351690c6425f37cd6d4ffb083a7e8e6/src/lib.rs#L373>
        //
        let egui_rpass = egui_wgpu::renderer::RenderPass::new(
            &device,
            surface_config.format,
            1,
        );

        Ok(Self {
            surface,
            device,
            queue,

            surface_config,
            depth_view,

            uniform_buffer,
            bind_group,

            geometries,
            pipelines,

            config_ui,

            egui: EguiState {
                context: egui_context,
                winit_state: egui_winit_state,
                rpass: egui_rpass,
                options: Default::default(),
            },
        })
    }

    /// Updates the geometry of the model being rendered.
    pub fn update_geometry(
        &mut self,
        mesh: Vertices,
        lines: Vertices,
        aabb: Aabb<3>,
    ) {
        self.geometries = Geometries::new(&self.device, &mesh, &lines, aabb);
    }

    /// Resizes the render surface.
    ///
    /// # Arguments
    /// - `size`: The target size for the render surface.
    pub fn handle_resize(&mut self, size: Size) {
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;

        self.surface.configure(&self.device, &self.surface_config);

        let depth_view =
            Self::create_depth_buffer(&self.device, &self.surface_config);
        self.depth_view = depth_view;
    }

    /// Draws the renderer, camera, and config state to the window.
    pub fn draw(
        &mut self,
        camera: &Camera,
        config: &mut DrawConfig,
        window: &egui_winit::winit::window::Window,
    ) -> Result<(), DrawError> {
        let aspect_ratio = self.surface_config.width as f64
            / self.surface_config.height as f64;
        let uniforms = Uniforms {
            transform: Transform::for_vertices(camera, aspect_ratio),
            transform_normals: Transform::for_normals(camera),
        };

        self.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );

        let surface_texture = self.surface.get_current_texture()?;
        let color_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: None },
        );

        self.clear_views(&mut encoder, &color_view);

        let drawables = Drawables::new(&self.geometries, &self.pipelines);

        if config.draw_model {
            drawables.model.draw(
                &mut encoder,
                &color_view,
                &self.depth_view,
                &self.bind_group,
            );
        }
        if config.draw_mesh {
            drawables.mesh.draw(
                &mut encoder,
                &color_view,
                &self.depth_view,
                &self.bind_group,
            );
        }
        if config.draw_debug {
            drawables.lines.draw(
                &mut encoder,
                &color_view,
                &self.depth_view,
                &self.bind_group,
            );
        }

        if self.egui.options.show_original_ui {
            self.config_ui
                .draw(
                    &self.device,
                    &mut encoder,
                    &color_view,
                    &self.surface_config,
                    &self.geometries.aabb,
                    config,
                )
                .map_err(DrawError::Text)?;
        }

        //
        // NOTE: The following comment was written for the original
        //       proof-of-concept which targeted older versions of
        //       Fornjot & `egui`, so some details may be outdated &
        //       not entirely apply to this updated implementation.
        //
        //       It's included here in case it still provides some
        //       useful context.
        //
        //
        // This integration is basically the result of locating the
        // `.present()` call in the `egui` example, here:
        //
        //     <https://github.com/hasenbanck/egui_example/blob/ca1262a701daf0b20e097ef627fc301ab63339d9/src/main.rs#L177>
        //
        // and then the equivalent call in `renderer.rs`, here:
        //
        //     <https://github.com/hannobraun/Fornjot/blob/15294c2ca2fa5ac5016bb29853943b28952f2dae/fj-app/src/graphics/renderer.rs#L245>
        //
        // Then working backwards from there to merge the functionality.
        //
        // In addition, the following examples were also referenced:
        //
        //  * "Make the example more like an actual use case #17"
        //    <https://github.com/hasenbanck/egui_example/pull/17/files>
        //    This removes some non-essential code from the example
        //    which helps clarify what's *actually* necessary.
        //
        //  * "Update to 0.17, use official winit backend #18"
        //    <https://github.com/hasenbanck/egui_example/pull/18/files>
        //    This uses a more up-to-date `egui` version which
        //    included some API changes.
        //    It's still not the *latest* `egui` version though.
        //

        let egui_input = self.egui.winit_state.take_egui_input(window);
        self.egui.context.begin_frame(egui_input);

        fn get_bbox_size_text(aabb: &Aabb<3>) -> String {
            /* Render size of model bounding box */
            let bbsize = aabb.size().components;
            let info = format!(
                "Model bounding box size:\n{:0.1} {:0.1} {:0.1}",
                bbsize[0].into_f32(),
                bbsize[1].into_f32(),
                bbsize[2].into_f32()
            );
            info
        }

        egui::SidePanel::left("fj-left-panel").show(&self.egui.context, |ui| {
            ui.add_space(16.0);

            ui.group(|ui| {
                ui.checkbox(&mut config.draw_model, "Render model")
                    .on_hover_text_at_pointer("Toggle with 1");
                ui.checkbox(&mut config.draw_mesh, "Render mesh")
                    .on_hover_text_at_pointer("Toggle with 2");
                ui.checkbox(&mut config.draw_debug, "Render debug")
                    .on_hover_text_at_pointer("Toggle with 3");
                ui.checkbox(
                    &mut self.egui.options.show_original_ui,
                    "Render original UI",
                );
                ui.add_space(16.0);
                ui.strong(get_bbox_size_text(&self.geometries.aabb));
            });

            ui.add_space(16.0);

            {
                ui.group(|ui| {
                    ui.checkbox(
                        &mut self.egui.options.show_settings_ui,
                        "Show egui settings UI",
                    );
                    if self.egui.options.show_settings_ui {
                        self.egui.context.settings_ui(ui);
                    }
                });

                ui.add_space(16.0);

                ui.group(|ui| {
                    ui.checkbox(
                        &mut self.egui.options.show_inspection_ui,
                        "Show egui inspection UI",
                    );
                    if self.egui.options.show_inspection_ui {
                        ui.indent("indent-inspection-ui", |ui| {
                            self.egui.context.inspection_ui(ui);
                        });
                    }
                });
            }

            ui.add_space(16.0);

            {
                //
                // Originally this was only meant to be a simple demonstration
                // of the `egui` `trace!()` macro...
                //
                // ...but it seems the trace feature can't be enabled
                // separately from the layout debug feature, which all
                // gets a bit messy...
                //
                // ...so, this instead shows one possible way to implement
                // "trace only" style debug text on hover.
                //
                ui.group(|ui| {
                    let label_text = format!(
                        "Show debug text demo.{}",
                        if self.egui.options.show_debug_text_example {
                            " (Hover me.)"
                        } else {
                            ""
                        }
                    );

                    ui.style_mut().wrap = Some(false);

                    if ui
                        .checkbox(
                            &mut self.egui.options.show_debug_text_example,
                            label_text,
                        )
                        .hovered()
                        && self.egui.options.show_debug_text_example
                    {
                        let hover_pos =
                            ui.input().pointer.hover_pos().unwrap_or_default();
                        ui.painter().debug_text(
                            hover_pos,
                            egui::Align2::LEFT_TOP,
                            egui::Color32::DEBUG_COLOR,
                            format!("{:#?}", &config),
                        );
                    }
                });
            }

            ui.add_space(16.0);

            {
                //
                // Demonstration of the `egui` layout debug functionality.
                //
                ui.group(|ui| {
                    //

                    if ui
                        .checkbox(
                            &mut self.egui.options.show_layout_debug_on_hover,
                            "Show layout debug on hover.",
                        )
                        .changed()
                    {
                        ui.ctx().set_debug_on_hover(
                            self.egui.options.show_layout_debug_on_hover,
                        );
                    }

                    ui.scope(|ui| {
                        if self.egui.options.show_trace {
                            egui::trace!(ui, format!("{:?}", &config));
                        }
                    });

                    ui.indent("indent-show-trace", |ui| {
                        ui.set_enabled(
                            self.egui.options.show_layout_debug_on_hover,
                        );

                        ui.checkbox(
                            &mut self.egui.options.show_trace,
                            "Also show egui trace.",
                        );

                        //
                    });
                });
            }

            ui.add_space(16.0);
        });

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let egui_output = self.egui.context.end_frame();
        let egui_paint_jobs = self.egui.context.tessellate(egui_output.shapes);

        self.paint_and_update_textures(
            //
            // Note: `scale_factor` can be overridden via `WINIT_X11_SCALE_FACTOR` environment variable,
            //       see: <https://docs.rs/winit/0.26.1/winit/window/struct.Window.html#method.scale_factor>
            //
            window.scale_factor() as f32,
            egui::Rgba::TRANSPARENT,
            &egui_paint_jobs,
            &egui_output.textures_delta,
            &color_view,
            &mut encoder,
        );

        let egui_input = egui::RawInput::default(); // TODO: Change this to use actual input.
        self.egui.context.begin_frame(egui_input);

        egui::SidePanel::left("fj-left-panel").show(&self.egui.context, |ui| {
            ui.label("Fornjot");
        });

        // End the UI frame. We could now handle the output and draw the UI with the backend.
        let egui_output = self.egui.context.end_frame();
        let egui_paint_jobs = self.egui.context.tessellate(egui_output.shapes);

        // TODO: Actually render the UI.

        let command_buffer = encoder.finish();
        self.queue.submit(Some(command_buffer));

        debug!("Presenting...");
        surface_texture.present();

        debug!("Finished drawing.");
        Ok(())
    }

    fn create_depth_buffer(
        device: &wgpu::Device,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> wgpu::TextureView {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: surface_config.width,
                height: surface_config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        });

        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn clear_views(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: true,
                },
            }],
            depth_stencil_attachment: Some(
                wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                },
            ),
        });
    }
}

/// Error describing the set of render surface initialization errors
#[derive(Error, Debug)]
pub enum InitError {
    #[error("I/O error: {0}")]
    /// General IO error
    Io(#[from] io::Error),

    #[error("Error request adapter")]
    /// Graphics accelerator acquisition error
    RequestAdapter,

    #[error("Error requesting device: {0}")]
    /// Device request errors
    ///
    /// See: [wgpu::RequestDeviceError](https://docs.rs/wgpu/latest/wgpu/struct.RequestDeviceError.html)
    RequestDevice(#[from] wgpu::RequestDeviceError),

    #[error("Error loading font: {0}")]
    /// Error loading font
    ///
    /// See: [ab_glyph::InvalidFont](https://docs.rs/ab_glyph/latest/ab_glyph/struct.InvalidFont.html)
    InvalidFont(#[from] InvalidFont),
}

/// Graphics rendering error
///
/// Describes errors related to non initialization graphics errors.
#[derive(Error, Debug)]
pub enum DrawError {
    #[error("Error acquiring output surface: {0}")]
    /// Surface drawing error.
    ///
    /// See - [wgpu::SurfaceError](https://docs.rs/wgpu/latest/wgpu/enum.SurfaceError.html)
    Surface(#[from] wgpu::SurfaceError),

    #[error("Error drawing text: {0}")]
    /// Text rasterisation error.
    Text(String),
}

impl Renderer {
    //
    // Note: `egui` changed how it handles updating textures on
    //       the GPU between v0.17.0 & v0.18.0, this means we can't
    //       use the same approach as original proof-of-concept used.
    //
    //       Unfortunately we can't use the helper function provided
    //       by `egui` here, as it is tightly integrated with `Painter`
    //       which assumes it is handling surface creation itself.
    //
    //       Additionally, subsequent code changes significantly
    //       changed the API but haven't yet been released.
    //
    //       And, to top it all off, the `Painter::paint_and_update_textures()`
    //       as it currently exists doesn't support a transparent
    //       clear color, which we rely on to overlay the UI on the
    //       already rendered model.
    //
    //       So, as an interim measure, this code is a copy of the
    //       texture update code from <https://github.com/emilk/egui/blob/f807a290a422f401939bd38236ece3cf86c8ee70/egui-wgpu/src/winit.rs#L102-L136>.
    //
    //       Update: Added transparency workaround.
    //
    fn paint_and_update_textures(
        &mut self,
        pixels_per_point: f32,
        clear_color: egui::Rgba,
        clipped_primitives: &[egui::ClippedPrimitive],
        textures_delta: &egui::TexturesDelta,
        output_view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        // Upload all resources for the GPU.
        let screen_descriptor = egui_wgpu::renderer::ScreenDescriptor {
            size_in_pixels: [
                self.surface_config.width,
                self.surface_config.height,
            ],
            pixels_per_point,
        };

        for (id, image_delta) in &textures_delta.set {
            self.egui.rpass.update_texture(
                &self.device,
                &self.queue,
                *id,
                image_delta,
            );
        }
        for id in &textures_delta.free {
            self.egui.rpass.free_texture(id);
        }

        self.egui.rpass.update_buffers(
            &self.device,
            &self.queue,
            clipped_primitives,
            &screen_descriptor,
        );

        //
        // This approach is based on the original proof-of-concept
        // integration which used `egui_wgpu_backend` and included
        // the following comment for context:
        //
        //   "Set this to `None` to overlay the UI on top of what's in the framebuffer"
        //   via <https://github.com/hasenbanck/egui_example/pull/17/files#diff-42cb6807ad74b3e201c5a7ca98b911c5fa08380e942be6e4ac5807f8377f87fcR132>
        //
        //   Alternatively, for initial testing, you can use a colour without alpha
        //   (e.g. `Some(wgpu::Color {r:0.5, g:0.0, b:0.0, a:1.0})` ) in order
        //   to verify that the renderpass is doing *something*.
        //
        let clear_color_ = if clear_color == egui::Rgba::TRANSPARENT {
            None
        } else {
            Some(wgpu::Color {
                r: clear_color.r() as f64,
                g: clear_color.g() as f64,
                b: clear_color.b() as f64,
                a: clear_color.a() as f64,
            })
        };

        // Record all render passes.
        self.egui.rpass.execute(
            encoder,
            output_view,
            clipped_primitives,
            &screen_descriptor,
            clear_color_,
        );
    }
}
