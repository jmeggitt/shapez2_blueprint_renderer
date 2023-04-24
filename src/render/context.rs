
use std::ffi::CString;
use std::num::NonZeroU32;
use glutin::config::{ColorBufferType, Config, ConfigSurfaceTypes, ConfigTemplateBuilder, GlConfig};
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContextSurfaceAccessor, PossiblyCurrentContext, Version};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, PbufferSurface, Surface, SurfaceAttributes, SurfaceAttributesBuilder, SwapInterval, WindowSurface};
use glutin_winit::{ApiPrefence, DisplayBuilder, GlWindow};
use raw_window_handle::HasRawWindowHandle;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use crate::render::gl::Gl;


pub struct DisplayManager {
    pub window: Window,
    pub gl_config: Config,
    pub gl_surface: Surface<WindowSurface>,
    pub gl_context: PossiblyCurrentContext,
    pub gl: Gl,
}


fn pick_gl_config(config_iter: Box<dyn Iterator<Item=Config> + '_>) -> Config {
    config_iter
        // .inspect(|x| println!("{:?}", x.config_surface_types()))
        .inspect(|x| println!("depth: {:?}, samples: {}", x.depth_size(), x.num_samples()))
        // .filter(|config| config.config_surface_types().contains(ConfigSurfaceTypes::PBUFFER))
        // .filter(|config| config.supports_transparency().unwrap_or(false))
        // .inspect(|x| println!("Pbuffer:     {:?}", x))
        // .filter(|config| config.supports_transparency().unwrap_or(false))
        // .inspect(|x| println!("Transparent: {:?}", x))

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

    let template = ConfigTemplateBuilder::new()
        // .with_alpha_size(8)
        // .with_depth_size(24)
        // .with_buffer_type(ColorBufferType::Rgb {
        //     r_size: 8,
        //     g_size: 8,
        //     b_size: 8,
        // })
        // .with_transparency(true)
        // .with_surface_type(ConfigSurfaceTypes::PBUFFER)
        // .with_single_buffering(true)
        // .with_pbuffer_sizes(width.try_into().expect("Width must be non-zero"), height.try_into().expect("height must be non-zero"))
        ;

    // let wb = Some(WindowBuilder::new());

    // let window = wb.map(|wb| wb.build(&event_loop).unwrap());
    //
    // let preference = ApiPrefence::default();
    // let raw_window_handle = window.as_ref().map(|window| window.raw_window_handle());
    //
    // let gl_display = create_display(&event_loop, preference, raw_window_handle)?;
    // #[cfg(windows)]
        let window_context = WindowBuilder::new()
        .with_visible(false)
            .with_transparent(true)
            .with_inner_size(PhysicalSize::new(width, height));

    let display_builder = DisplayBuilder::new();
    // .with_preference(ApiPrefence::PreferEgl)
    // #[cfg(windows)]
        let display_builder = display_builder.with_window_builder(Some(window_context));

    let (mut window, gl_config) = display_builder
        .build(&event_loop, template, pick_gl_config)
        .unwrap();

    println!("Picked a GL config with {} samples per pixel", gl_config.num_samples());

    // #[]
    let window = window.expect("Window was created");
    let raw_window_handle = Some(window.raw_window_handle());
    // let raw_window_handle = window.as_ref().map(|x| x.raw_window_handle());

    // XXX The display could be obtained from the any object created by it, so we
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

    let mut not_current_gl_context = unsafe {
        gl_display.create_context(&gl_config, &context_attributes)
            .unwrap_or_else(|_| {
                gl_display.create_context(&gl_config, &fallback_context_attributes).unwrap_or_else(
                    |_| {
                        gl_display
                            .create_context(&gl_config, &legacy_context_attributes)
                            .expect("failed to create context")
                    },
                )
            })
    };

    // let mut attrs = SurfaceAttributes::default();
    let mut attrs = window.build_surface_attributes(Default::default());
    // // gl_display.su
    let gl_surface = unsafe {
    //     // gl_config.display().create_pbuffer_surface(&gl_config, &attrs).unwrap()
    //     // gl_config.display().
    //     // println!("Pixmap surface: {}", gl_config.display().create_pixmap_surface(&gl_config, &Default::default()).is_ok());
    //     // println!("Window surface: {}", gl_config.display().create_window_surface(&gl_config, &Default::default()).is_ok());
    //     // println!("Pbuffer surface: {}", gl_config.display().create_pbuffer_surface(&gl_config, &attrs).is_ok());
        gl_display.create_window_surface(&gl_config, &attrs).unwrap()
    };


    // let gl_context = not_current_gl_context.make_current_surfaceless();
    let gl_context = not_current_gl_context.make_current(&gl_surface).unwrap();

    // Try setting vsync.
    // if let Err(res) = gl_surface
    //     .set_swap_interval(&gl_context, SwapInterval::Wait(NonZeroU32::new(1).unwrap()))
    // {
    //     eprintln!("Error setting vsync: {res:?}");
    // }

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

