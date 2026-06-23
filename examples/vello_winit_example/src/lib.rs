use std::num::NonZeroUsize;
use std::sync::Arc;
use vello::kurbo::Size;
use vello::peniko::Color;
use vello::util::{RenderContext, RenderSurface};
use vello::wgpu;
use vello::{
    AaConfig, RenderParams, Renderer, RendererOptions, Scene,
};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

pub trait VelloDemo {
    fn window_title(&self) -> &'static str;
    fn initial_logical_size(&self) -> (f64, f64);
    fn size_changed(&mut self, size: Size);
    fn rebuild_scene(&mut self, scene: &mut Scene, scale_factor: f64);
}

pub struct VelloWinitApp<'s, D: VelloDemo> {
    pub context: RenderContext,
    pub renderer: Option<Renderer>,
    pub state: RenderState<'s>,
    pub scene: Scene,
    pub demo: D,
}

pub enum RenderState<'s> {
    Suspended(Option<Arc<Window>>),
    Active {
        surface: Box<RenderSurface<'s>>,
        window: Arc<Window>,
    },
}

impl<'s, D: VelloDemo> VelloWinitApp<'s, D> {
    pub fn new(demo: D) -> Self {
        Self {
            context: RenderContext::new(),
            renderer: None,
            state: RenderState::Suspended(None),
            scene: Scene::new(),
            demo,
        }
    }

    fn render(&mut self) {
        let (surface, window) = match &mut self.state {
            RenderState::Active { surface, window } => {
                (surface, window)
            }
            _ => return,
        };

        self.scene.reset();

        let size = window.inner_size();
        let scale_factor = window.scale_factor();

        if size.width == 0 || size.height == 0 {
            return;
        }

        if surface.config.width != size.width
            || surface.config.height != size.height
        {
            self.context.resize_surface(
                surface,
                size.width,
                size.height,
            );
        }

        self.demo.rebuild_scene(&mut self.scene, scale_factor);

        let dev = &self.context.devices[surface.dev_id];
        let texture = match surface.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => {
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(
                surface_texture,
            ) => {
                self.context.resize_surface(
                    surface,
                    size.width,
                    size.height,
                );
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Outdated
            | wgpu::CurrentSurfaceTexture::Lost => {
                self.context.resize_surface(
                    surface,
                    size.width,
                    size.height,
                );
                return;
            }
            _ => return,
        };

        self.renderer
            .as_mut()
            .unwrap()
            .render_to_texture(
                &dev.device,
                &dev.queue,
                &self.scene,
                &surface.target_view,
                &RenderParams {
                    base_color: Color::from_rgb8(28, 28, 46),
                    width: surface.config.width,
                    height: surface.config.height,
                    antialiasing_method: AaConfig::Area,
                },
            )
            .unwrap();

        let mut enc = dev.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: None },
        );

        surface.blitter.copy(
            &dev.device,
            &mut enc,
            &surface.target_view,
            &texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default()),
        );

        dev.queue.submit([enc.finish()]);
        texture.present();
    }

    fn handle_resize(
        &mut self,
        scale_factor: f64,
        size: PhysicalSize<u32>,
    ) {
        let logical_width = size.width as f64 / scale_factor;
        let logical_height = size.height as f64 / scale_factor;
        self.demo
            .size_changed(Size::new(logical_width, logical_height));
    }
}

impl<D: VelloDemo> ApplicationHandler for VelloWinitApp<'_, D> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let RenderState::Suspended(cached_window) = &mut self.state
        else {
            return;
        };

        let window = cached_window.take().unwrap_or_else(|| {
            let (w, h) = self.demo.initial_logical_size();
            let attr = Window::default_attributes()
                .with_inner_size(LogicalSize::new(w, h))
                .with_title(self.demo.window_title());
            Arc::new(event_loop.create_window(attr).unwrap())
        });

        self.handle_resize(
            window.scale_factor(),
            window.inner_size(),
        );

        let size = window.inner_size();
        let surface_future = self.context.create_surface(
            window.clone(),
            size.width,
            size.height,
            wgpu::PresentMode::AutoVsync,
        );
        let surface =
            pollster::block_on(surface_future).expect("surface");

        let device_handle = &self.context.devices[surface.dev_id];
        surface
            .surface
            .configure(&device_handle.device, &surface.config);

        if self.renderer.is_none() {
            self.renderer = Some(
                Renderer::new(
                    &device_handle.device,
                    RendererOptions {
                        use_cpu: false,
                        antialiasing_support:
                            vello::AaSupport::area_only(),
                        num_init_threads: NonZeroUsize::new(1),
                        pipeline_cache: None,
                    },
                )
                .unwrap(),
            );
        }

        self.state = RenderState::Active {
            surface: Box::new(surface),
            window,
        };
    }

    fn suspended(&mut self, _el: &ActiveEventLoop) {
        if let RenderState::Active { window, .. } = &self.state {
            self.state = RenderState::Suspended(Some(window.clone()));
        }
    }

    fn window_event(
        &mut self,
        el: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => {
                let scale_factor = match &self.state {
                    RenderState::Active { window, .. } => {
                        window.scale_factor()
                    }
                    _ => return,
                };
                self.handle_resize(scale_factor, size);
                self.render();
            }
            WindowEvent::RedrawRequested => {
                self.render();
                if let RenderState::Active { window, .. } =
                    &self.state
                {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
