use crate::render::gl::Gl;
use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentGlContextSurfaceAccessor,
    PossiblyCurrentContext, Version,
};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{Surface, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasRawWindowHandle;
use std::ffi::CString;
use std::ops::Deref;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub struct DisplayManager {
    pub window: Window,
    pub gl_config: Config,
    pub gl_surface: Surface<WindowSurface>,
    pub gl_context: PossiblyCurrentContext,
    pub gl: Gl,
}

impl Deref for DisplayManager {
    type Target = Gl;

    fn deref(&self) -> &Self::Target {
        &self.gl
    }
}

fn pick_gl_config(config_iter: Box<dyn Iterator<Item = Config> + '_>) -> Config {
    config_iter
        .inspect(|x| println!("depth: {:?}, samples: {}", x.depth_size(), x.num_samples()))
        .reduce(|best, next| {
            if next.num_samples() > best.num_samples() {
                next
            } else {
                best
            }
        })
        .expect("at least one GL config is available")
}

pub fn setup_opengl(width: u32, height: u32) -> (EventLoop<()>, DisplayManager) {
    let event_loop = EventLoop::new();

    let template = ConfigTemplateBuilder::new();

    let window_context = WindowBuilder::new()
        .with_visible(false)
        .with_transparent(true)
        .with_inner_size(PhysicalSize::new(width, height));

    let display_builder = DisplayBuilder::new();
    let display_builder = display_builder.with_window_builder(Some(window_context));

    let (window, gl_config) = display_builder
        .build(&event_loop, template, pick_gl_config)
        .unwrap();

    println!(
        "Picked a GL config with {} samples per pixel",
        gl_config.num_samples()
    );

    let window = window.expect("Window was created");
    let raw_window_handle = Some(window.raw_window_handle());

    // The display could be obtained from the any object created by it, so we
    // can query it from the config.
    let gl_display = gl_config.display();

    // The context creation part. It can be created before surface and that's how
    // it's expected in multithreaded + multiwindow operation mode, since you
    // can send NotCurrentContext, but not Surface.
    let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);

    // Since glutin by default tries to create OpenGL core context, which may not be
    // present we should try gles.
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(raw_window_handle);

    // There are also some old devices that support neither modern OpenGL nor GLES.
    // To support these we can try and create a 2.1 context.
    let legacy_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1))))
        .build(raw_window_handle);

    let not_current_gl_context = unsafe {
        gl_display
            .create_context(&gl_config, &context_attributes)
            .unwrap_or_else(|_| {
                gl_display
                    .create_context(&gl_config, &fallback_context_attributes)
                    .unwrap_or_else(|_| {
                        gl_display
                            .create_context(&gl_config, &legacy_context_attributes)
                            .expect("failed to create context")
                    })
            })
    };

    let attrs = window.build_surface_attributes(Default::default());
    let gl_surface = unsafe {
        gl_display
            .create_window_surface(&gl_config, &attrs)
            .unwrap()
    };

    let gl_context = not_current_gl_context.make_current(&gl_surface).unwrap();

    let gl = Gl::load_with(|symbol| {
        let symbol = CString::new(symbol).unwrap();
        gl_display.get_proc_address(symbol.as_c_str()).cast()
    });

    let display_manager = DisplayManager {
        window,
        gl_config,
        gl_surface,
        gl_context,
        gl,
    };

    (event_loop, display_manager)
}
