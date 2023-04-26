use crate::render::gl::Gl;
use glutin::config::{Config, ConfigTemplateBuilder, GlConfig};
use glutin::context::{
    ContextApi, ContextAttributesBuilder, NotCurrentContext, NotCurrentGlContextSurfaceAccessor,
    PossiblyCurrentContext, Version,
};
use glutin::display::{Display, GetGlDisplay, GlDisplay};
use glutin::surface::{Surface, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use log::{error, info, warn};
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::ffi::CString;
use std::ops::Deref;
use std::process::exit;
use std::time::Instant;
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
    let config = config_iter.reduce(|best, next| {
        if next.num_samples() > best.num_samples() {
            next
        } else {
            best
        }
    });

    match config {
        Some(x) => x,
        None => {
            error!("No 3D display configurations available on this system");
            exit(1);
        }
    }
}

unsafe fn create_context(
    config: &Config,
    display: &Display,
    raw_window_handle: Option<RawWindowHandle>,
) -> NotCurrentContext {
    // The context creation part. It can be created before surface and that's how
    // it's expected in multithreaded + multiwindow operation mode, since you
    // can send NotCurrentContext, but not Surface.
    info!("Using OpenGL Core graphics context");
    let context_attributes = ContextAttributesBuilder::new().build(raw_window_handle);

    match display.create_context(config, &context_attributes) {
        Ok(context) => return context,
        Err(err) => warn!("Failed to create OpenGL core context: {}", err),
    }

    // Since glutin by default tries to create OpenGL core context, which may not be
    // present we should try gles.
    info!("Using GLes graphics context");
    let fallback_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::Gles(None))
        .build(raw_window_handle);

    match display.create_context(config, &fallback_context_attributes) {
        Ok(context) => return context,
        Err(err) => warn!("Failed to create OpenGL core context: {}", err),
    }

    // There are also some old devices that support neither modern OpenGL nor GLES.
    // To support these we can try and create a 2.1 context.
    info!("Using OpenGL 2.1 (legacy) graphics context");
    let legacy_context_attributes = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(Some(Version::new(2, 1))))
        .build(raw_window_handle);

    match display.create_context(config, &legacy_context_attributes) {
        Ok(context) => context,
        Err(err) => {
            error!("Failed to create OpenGL 2.1 (legacy) context: {}", err);
            exit(1)
        }
    }
}

pub fn setup_opengl(width: u32, height: u32) -> (EventLoop<()>, DisplayManager) {
    info!("Creating graphics context");
    let context_setup_start_time = Instant::now();

    let event_loop = EventLoop::new();
    let template = ConfigTemplateBuilder::new();

    let window_context = WindowBuilder::new()
        .with_visible(false)
        .with_transparent(true)
        .with_inner_size(PhysicalSize::new(width, height));

    let display_builder = DisplayBuilder::new();
    let display_builder = display_builder.with_window_builder(Some(window_context));

    let (window, gl_config) = match display_builder.build(&event_loop, template, pick_gl_config) {
        Ok(v) => v,
        Err(e) => {
            error!("Failed to find graphics configuration: {}", e);
            exit(1);
        }
    };

    info!("Picked a GL config with MSAAx{}", gl_config.num_samples());

    let window = match window {
        Some(x) => x,
        None => {
            error!("Failed to create hidden window");
            exit(1);
        }
    };
    let raw_window_handle = Some(window.raw_window_handle());

    // The display could be obtained from the any object created by it, so we
    // can query it from the config.
    let gl_display = gl_config.display();
    let not_current_gl_context =
        unsafe { create_context(&gl_config, &gl_display, raw_window_handle) };

    let attrs = window.build_surface_attributes(Default::default());
    let gl_surface = match unsafe { gl_display.create_window_surface(&gl_config, &attrs) } {
        Ok(surface) => surface,
        Err(err) => {
            error!("Failed to create window GL surface: {}", err);
            exit(1);
        }
    };

    let gl_context = not_current_gl_context.make_current(&gl_surface).unwrap();

    let gl = Gl::load_with(|symbol| {
        let symbol = CString::new(symbol).expect("symbols do not contain null terminators");
        gl_display.get_proc_address(symbol.as_c_str()).cast()
    });

    let display_manager = DisplayManager {
        window,
        gl_config,
        gl_surface,
        gl_context,
        gl,
    };

    info!(
        "Setup graphics context in {:?}",
        context_setup_start_time.elapsed()
    );
    (event_loop, display_manager)
}
