#![cfg(feature = "xwayland")]
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use smithay::{
    desktop::Window,
    reexports::{calloop::LoopHandle, wayland_server::DisplayHandle, x11rb},
    utils::{Logical, Rectangle},
    xwayland::{
        xwm::{Reorder, ResizeEdge, XwmId},
        X11Surface, X11Wm, XWayland, XWaylandEvent, XWaylandSource, XwmHandler,
    },
};
#[cfg(feature = "xwayland")]
use tracing::{debug, error, info};

use crate::{
    state::{Backend, CalloopData, MagmaState},
    utils::workspace::MagmaWindow,
};

#[derive(Debug)]
pub struct XWaylandState {
    pub handle: XWayland,
    pub xwm: Option<X11Wm>,
    pub xdisplay: Option<u32>,
}

impl XWaylandState {
    /// Create a new handle and event source
    pub fn new(dh: &DisplayHandle) -> (Self, XWaylandSource) {
        let (handle, source) = XWayland::new(dh);

        (
            Self {
                handle,
                xwm: None,
                xdisplay: None,
            },
            source,
        )
    }

    /// Start the xwayland server
    pub fn start(&mut self, loop_handle: LoopHandle<CalloopData<impl Backend>>) {
        let env: HashMap<String, String> = HashMap::new();

        self.xdisplay = Some(
            self.handle
                .start(loop_handle, None, env, true, |_| {})
                .expect("Failed to start xwayland server!"),
        );
    }

    pub fn on_event<BackendData: Backend>(
        &mut self,
        event: XWaylandEvent,
        loop_handle: LoopHandle<'static, CalloopData<BackendData>>,
        display_handle: &mut DisplayHandle,
    ) {
        match event {
            XWaylandEvent::Ready {
                connection,
                client,
                client_fd,
                display,
            } => {
                let d = display;
                std::env::set_var("DISPLAY", format!(":{d}"));
                info!("Initialized xwayland: fd {}, display {}", client_fd, d);
                self.xwm = match X11Wm::start_wm(
                    loop_handle,
                    display_handle.clone(),
                    connection,
                    client,
                ) {
                    Ok(wm) => Some(wm),
                    Err(e) => {
                        error!(?e, "Failed to start xwaylacnd WM");
                        None
                    }
                };
            }
            XWaylandEvent::Exited => {
                info!("xwayland exited");
                self.xwm = None;
                self.xdisplay = None;
            }
        }
    }
}

impl<BackendData: Backend> XwmHandler for CalloopData<BackendData> {
    fn xwm_state(&mut self, xwm: XwmId) -> &mut X11Wm {
        XwmHandler::xwm_state(&mut self.state, xwm)
    }

    fn new_window(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::new_window(&mut self.state, xwm, window)
    }

    fn new_override_redirect_window(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::new_override_redirect_window(&mut self.state, xwm, window)
    }

    fn map_window_request(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::map_window_request(&mut self.state, xwm, window)
    }

    fn mapped_override_redirect_window(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::mapped_override_redirect_window(&mut self.state, xwm, window)
    }

    fn unmapped_window(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::unmapped_window(&mut self.state, xwm, window)
    }

    fn destroyed_window(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::destroyed_window(&mut self.state, xwm, window)
    }

    fn configure_request(
        &mut self,
        xwm: XwmId,
        window: X11Surface,
        x: Option<i32>,
        y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        reorder: Option<Reorder>,
    ) {
        XwmHandler::configure_request(&mut self.state, xwm, window, x, y, w, h, reorder)
    }

    fn configure_notify(
        &mut self,
        xwm: XwmId,
        window: X11Surface,
        geometry: Rectangle<i32, Logical>,
        above: Option<x11rb::protocol::xproto::Window>,
    ) {
        XwmHandler::configure_notify(&mut self.state, xwm, window, geometry, above)
    }

    fn resize_request(
        &mut self,
        xwm: XwmId,
        window: X11Surface,
        button: u32,
        resize_edge: ResizeEdge,
    ) {
        XwmHandler::resize_request(&mut self.state, xwm, window, button, resize_edge)
    }

    fn move_request(&mut self, xwm: XwmId, window: X11Surface, button: u32) {
        XwmHandler::move_request(&mut self.state, xwm, window, button)
    }

    fn map_window_notify(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::map_window_notify(&mut self.state, xwm, window)
    }

    fn maximize_request(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::maximize_request(&mut self.state, xwm, window)
    }

    fn unmaximize_request(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::unmaximize_request(&mut self.state, xwm, window)
    }

    fn fullscreen_request(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::fullscreen_request(&mut self.state, xwm, window)
    }

    fn unfullscreen_request(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::unfullscreen_request(&mut self.state, xwm, window)
    }

    fn minimize_request(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::minimize_request(&mut self.state, xwm, window)
    }

    fn unminimize_request(&mut self, xwm: XwmId, window: X11Surface) {
        XwmHandler::unminimize_request(&mut self.state, xwm, window)
    }

    fn allow_selection_access(
        &mut self,
        xwm: XwmId,
        selection: smithay::wayland::selection::SelectionTarget,
    ) -> bool {
        XwmHandler::allow_selection_access(&mut self.state, xwm, selection)
    }

    fn send_selection(
        &mut self,
        xwm: XwmId,
        selection: smithay::wayland::selection::SelectionTarget,
        mime_type: String,
        fd: std::os::unix::prelude::OwnedFd,
    ) {
        XwmHandler::send_selection(&mut self.state, xwm, selection, mime_type, fd)
    }

    fn new_selection(
        &mut self,
        xwm: XwmId,
        selection: smithay::wayland::selection::SelectionTarget,
        mime_types: Vec<String>,
    ) {
        XwmHandler::new_selection(&mut self.state, xwm, selection, mime_types)
    }

    fn cleared_selection(
        &mut self,
        xwm: XwmId,
        selection: smithay::wayland::selection::SelectionTarget,
    ) {
        XwmHandler::cleared_selection(&mut self.state, xwm, selection)
    }
}

impl<BackendData: Backend> XwmHandler for MagmaState<BackendData> {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.xwayland_state.xwm.as_mut().unwrap()
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {
        debug!("New x11 window");
    }

    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn map_window_request(&mut self, _xwm: XwmId, window: X11Surface) {
        window.set_mapped(true).unwrap();
        let rec = window.geometry();
        let window = Window::new_x11_window(window);
        let magma_window = MagmaWindow {
            window: window,
            rec,
        };
        self.workspaces
            .current_mut()
            .add_window(Rc::new(RefCell::new(magma_window)));
        // self.set_input_focus(FocusTarget::Window(window));
        debug!("Mapped new x11 window");
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, window: X11Surface) {
        let rec = window.geometry();
        let window = Window::new_x11_window(window);
        self.workspaces
            .current_mut()
            .add_window(Rc::new(RefCell::new(MagmaWindow { window, rec })));
        debug!("Override mapped new x11 window");
    }

    fn unmapped_window(&mut self, _xwm: XwmId, window: X11Surface) {
        let workspaces = self.workspaces.iter().collect::<Vec<_>>();
        let elem = Window::new_x11_window(window.clone());
        for workspace in workspaces {
            if workspace.contains_window(&elem) {
                workspace.remove_window(&elem);
            }
        }
        window.set_mapped(false).unwrap();
        debug!("Unmapped x11 window");
    }

    fn destroyed_window(&mut self, _xwm: XwmId, _window: X11Surface) {
        debug!("Destroyed x11 window");
    }

    fn configure_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        _x: Option<i32>,
        _y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        _reorder: Option<Reorder>,
    ) {
        let mut geo = window.geometry();
        if let Some(w) = w {
            geo.size.w = w as i32;
        }
        if let Some(h) = h {
            geo.size.h = h as i32;
        }
        let _ = window.configure(geo);
    }

    fn configure_notify(
        &mut self,
        _xwm: XwmId,
        _window: X11Surface,
        _geometry: Rectangle<i32, Logical>,
        _above: Option<x11rb::protocol::xproto::Window>,
    ) {
        info!("TODO: x11 configure_notify");
    }

    fn resize_request(
        &mut self,
        _xwm: XwmId,
        _window: X11Surface,
        _button: u32,
        _resize_edge: ResizeEdge,
    ) {
        info!("TODO: x11 resize_request");
    }

    fn move_request(&mut self, _xwm: XwmId, _window: X11Surface, _button: u32) {
        info!("TODO: x11 move_request");
    }
}