use std::ptr::NonNull;

use rwh_06::{RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle};
use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{
        wl_buffer, wl_compositor,
        wl_display::WlDisplay,
        wl_keyboard,
        wl_registry::{self},
        wl_seat, wl_shm, wl_shm_pool, wl_surface,
    },
    Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum,
};
use wayland_protocols::xdg::{
    activation::v1::client::{
        xdg_activation_token_v1::XdgActivationTokenV1, xdg_activation_v1::XdgActivationV1,
    },
    decoration::zv1::client::{
        zxdg_decoration_manager_v1::ZxdgDecorationManagerV1,
        zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1,
    },
    shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base},
};

use crate::{WindowAttribs, WindowEvent, WindowMode};

pub struct PlatformWindow {
    pub display: WlDisplay,
    event_queue: EventQueue<State>,
    state: State,
}

struct State {
    running: bool,
    pub width: u32,
    pub height: u32,
    base_surface: Option<wl_surface::WlSurface>,
    buffer: Option<wl_buffer::WlBuffer>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    xdg_surface: Option<(xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel)>,
    xdg_decoration_manager: Option<ZxdgDecorationManagerV1>,
    xdg_toplevel_decoration: Option<ZxdgToplevelDecorationV1>,
    configured: bool,

    pending_events: Vec<WindowEvent>,
}

delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_buffer::WlBuffer);

impl State {
    fn init_xdg_surface(&mut self, qh: &QueueHandle<State>, attris: &WindowAttribs) {
        let wm_base = self.wm_base.as_ref().unwrap();
        let base_surface = self.base_surface.as_ref().unwrap();

        let xdg_surface = wm_base.get_xdg_surface(base_surface, qh, ());
        let toplevel = xdg_surface.get_toplevel(qh, ());
        toplevel.set_title(attris.title.clone());
        toplevel.set_app_id("com.ventengine.VentEngine".into());

        match attris.mode {
            WindowMode::FullScreen => toplevel.set_fullscreen(None),
            WindowMode::Maximized => toplevel.set_maximized(),
            WindowMode::Minimized => toplevel.set_minimized(),
            _ => {}
        }
        if let Some(max_size) = attris.max_size {
            toplevel.set_max_size(max_size.0 as i32, max_size.1 as i32)
        }

        if let Some(min_size) = attris.min_size {
            toplevel.set_min_size(min_size.0 as i32, min_size.1 as i32)
        }

        if let Some(manager) = &self.xdg_decoration_manager {
            // if supported, let the compositor render titlebars for us
            self.xdg_toplevel_decoration = Some(manager.get_toplevel_decoration(&toplevel, qh, ()));
            self.xdg_toplevel_decoration.as_ref().unwrap().set_mode(wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::Mode::ServerSide);
        }

        self.xdg_surface = Some((xdg_surface, toplevel));
    }

    fn init_xdg_activation(&mut self, qh: &QueueHandle<State>, xdg_activation_v1: XdgActivationV1) {
        let token = xdg_activation_v1.get_activation_token(qh, ());
        token.set_app_id("com.ventengine.VentEngine".into());
        token.set_surface(self.base_surface.as_ref().unwrap())
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key {
            key,
            serial,
            time,
            state: key_state,
        } = event
        {
            if key == 1 {
                // ESC key
                state.running = false;
            }
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        data: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
        {
            if capabilities.contains(wl_seat::Capability::Keyboard) {
                seat.get_keyboard(qh, ());
            }
        }
    }
}

impl Dispatch<xdg_wm_base::XdgWmBase, ()> for State {
    fn event(
        _: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for State {
    fn event(
        state: &mut Self,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            xdg_surface.ack_configure(serial);
            state.configured = true;
            let surface = state.base_surface.as_ref().unwrap();
            if let Some(ref buffer) = state.buffer {
                surface.attach(Some(buffer), 0, 0);
                surface.commit();
            }
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for State {
    fn event(
        state: &mut Self,
        _: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_toplevel::Event::Close {} = event {
            state.pending_events.push(WindowEvent::Close);
        }
        if let xdg_toplevel::Event::ConfigureBounds { width, height } = event {
            state.width = width as u32;
            state.height = height as u32;
        }
    }
}

impl wayland_client::Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as Proxy>::Event,
        data: &GlobalListContents,
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<wl_compositor::WlCompositor, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &wl_compositor::WlCompositor,
        event: <wl_compositor::WlCompositor as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZxdgDecorationManagerV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &ZxdgDecorationManagerV1,
        event: <ZxdgDecorationManagerV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        todo!()
    }
}
impl Dispatch<ZxdgToplevelDecorationV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &ZxdgToplevelDecorationV1,
        event: <ZxdgToplevelDecorationV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        todo!()
    }
}
impl Dispatch<XdgActivationV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &XdgActivationV1,
        event: <XdgActivationV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        todo!()
    }
}
impl Dispatch<XdgActivationTokenV1, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &XdgActivationTokenV1,
        event: <XdgActivationTokenV1 as Proxy>::Event,
        data: &(),
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        todo!()
    }
}

impl PlatformWindow {
    pub fn create_window(attribs: &WindowAttribs) -> Self {
        let conn = wayland_client::Connection::connect_to_env().expect("Failed to get connection");
        println!("Connected to Wayland Server");

        let mut state = State {
            running: true,
            width: attribs.width,
            height: attribs.height,
            base_surface: None,
            buffer: None,
            wm_base: None,
            xdg_surface: None,
            configured: false,
            xdg_toplevel_decoration: None,
            xdg_decoration_manager: None,
            pending_events: vec![],
        };

        let display = conn.display();

        let (globals, event_queue) = registry_queue_init::<State>(&conn).unwrap();
        let qhandle = event_queue.handle();

        dbg!(&globals.contents());

        let wm_base: xdg_wm_base::XdgWmBase =
            globals.bind(&event_queue.handle(), 1..=6, ()).unwrap();
        state.wm_base = Some(wm_base);

        let compositor: wl_compositor::WlCompositor =
            globals.bind(&event_queue.handle(), 1..=6, ()).unwrap();
        let surface = compositor.create_surface(&qhandle, ());
        state.base_surface = Some(surface);

        let wl_seat: wl_seat::WlSeat = globals.bind(&event_queue.handle(), 1..=6, ()).unwrap();
        // let xdg_decoration_manager: ZxdgDecorationManagerV1 =
        //     globals.bind(&event_queue.handle(), 1..=1, ()).unwrap();
        // state.xdg_decoration_manager = Some(xdg_decoration_manager);

        if state.wm_base.is_some() && state.xdg_surface.is_none() {
            state.init_xdg_surface(&qhandle, attribs);
        }
        state.base_surface.as_ref().unwrap().commit();

        let xdg_activation: XdgActivationV1 =
            globals.bind(&event_queue.handle(), 1..=1, ()).unwrap();

        state.init_xdg_activation(&qhandle, xdg_activation);

        PlatformWindow {
            display,
            state,
            event_queue,
        }
    }

    pub fn poll<F>(mut self, mut event_handler: F)
    where
        F: FnMut(WindowEvent),
    {
        while self.state.running {
            self.event_queue
                .dispatch_pending(&mut self.state)
                .expect("Failed to dispatch pending");

            self.state
                .pending_events
                .drain(..)
                .for_each(&mut event_handler);

            event_handler(WindowEvent::Draw);
        }
    }

    pub fn width(&self) -> u32 {
        self.state.width
    }

    pub fn height(&self) -> u32 {
        self.state.height
    }

    pub fn raw_display_handle(&self) -> RawDisplayHandle {
        RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
            NonNull::new(self.display.id().as_ptr().cast()).unwrap(),
        ))
    }

    pub fn raw_window_handle(&self) -> RawWindowHandle {
        let ptr = self.state.base_surface.as_ref().unwrap().id().as_ptr();
        RawWindowHandle::Wayland(WaylandWindowHandle::new(
            NonNull::new(ptr as *mut _).unwrap(),
        ))
    }

    pub fn close(&mut self) {
        self.event_queue
            .flush()
            .expect("Failed to flush Event Queue");
        self.state.running = false;
    }
}

impl Drop for PlatformWindow {
    fn drop(&mut self) {
        self.close()
    }
}
