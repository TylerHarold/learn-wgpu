use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    window::Window,
};

#[cfg(target_arch="wasm32")]
use wasm_bindgen::prelude::*;


#[cfg_attr(target_arch="wasm32", wasm_bindgen(start))]
pub async fn run() {
    /*
    *   It is very important to enable logging via env_logger::init();. When gpu hits any
    *   error it panics with a generic message, while logging the real error via the log
    *   crate. This means if you don't include env_logger::init(), wgpu will fail
    *   silently, leaving you very confused!
    *
    *   We are then using cfg-if to toggle what logger we are using based on if we're in
    *   WASM or not.
    */
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
        } else {
            env_logger::init();
        }
    }

    /*
    *   The code below crates a window and keeps it open until the user closes it, or
    *   presses escape.
    */

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // After we build the window, create a mutable state
    let mut state = State::new(&window).await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                },
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        },
        _ => {}
    });

    /*
    *   After our event loop & window, if we're on WASM, we need to add a canvas to
    *   the HTML document that we'll host our application
    */
    #[cfg(target_arch = "wasm32")] {
        // Winit prevents sizing with css, so we have to set the size manually
        // when on the web
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm_example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }
}

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
}

impl State {
    // Creating some of the wgpu types requires async code
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());

        // The surface is the part of the window we draw to.
        let surface = unsafe {
            instance.create_surface(window)
        };

        // The adapter is the handle to our graphics card.
        // We can use this to get information about the graphics card
        // including its name and what backend the adapter uses. We will
        // use this to create our Device & Queue later.
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                // power_preference has two variants, LowPower, and HighPerformance.
                power_preference: wgpu::PowerPreference::default(),
                // compatible_surface field tells wgpu to find an adapter that can present
                // to the supplied surface.
                compatible_surface: Some(&surface),
                // force_fallback_adapter forces wgpu to pick an adapter that will work on
                // all hardware. This usually means that the rendering backend will use a
                // "software" system, instead of hardware such as a GPU.
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        // The options passed to request_adapter aren't guaranteed to work for all devices,
        // but will work for most of them. If wgpu can''t find an adapter with the required
        // permissions, request_adapter will return None. If you want to get all the adapters
        // for a particular backend you can use enumerate_adapters. This will give you an
        // iterator that you cna loop over to check if one of the adapters work for your needs.
        //
        // Another thing to note is that Adapters are locked to a specific backend. If you are
        // on Windows and have 2 graphics cards you will have at least 4 adapters available to use.
        
        /*
            let adapter = instance
                .enumerate_adapters(wgpu::Backends::all())
                .filter(|adapter| {
                    // Check if this adapter supports our surface
                    surface.get_preferred_format(&adapter).is_some()
                })
                .next()
                .unwrap()
        */

        // Use the adapter to create the device and queue.
        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if we're building for
                // the web we'll have to disable some.
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        // Surface config
        let config = wgpu::SurfaceConfiguration {
            // Usage field will describe how SurfaceTexture(s) will be used. RENDER_ATTACHMENT
            // specifices that the textures will be used to write to the screen.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // Format describes how SurfaceTexture(s) will be stored on the gpu. We use
            // get_preferred_format(&adapter) to figure out the best format to use based on the
            // display you're using.
            format: surface.get_supported_formats(&adapter)[0],
            // Width & height are the width & height in pixels of a SurfaceTexture. This should
            // usually be the width and height of the window. Don't set this to 0, this WILL crash lol.
            width: size.width,
            height: size.height,
            // present_mode uses wgpu::PresentMode enum which determines how to sync the surface with
            // the display. The option we picked, PresentMode::Fifo, will cap the display rate at the
            // display's framerate (essentially VSync). This mode is guaranteed to be supoorted on all platforms.

            // If we want to let our users pick what PresentMode they use, you can use Surface::get_supported_modes()
            // to get a list of all the PresentModes the surface supports
            // let modes = surface.get_supported_modes(&adapter);
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);


        Self {
            surface,
            device,
            queue,
            config,
            size,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        todo!()
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        todo!()
    }

    fn update(&mut self) {
        todo!()
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        todo!()
    }
}
