use std::ops::Deref;

use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_compositor, delegate_data_device, delegate_layer_shell, delegate_output,
    delegate_primary_selection, delegate_seat, delegate_shm,
    desktop::{layer_map_for_output, space::SpaceElement, LayerSurface},
    input::{SeatHandler, SeatState},
    output::Output,
    reexports::wayland_server::{
        protocol::{wl_output::WlOutput, wl_surface::WlSurface},
        Client, Resource,
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, CompositorClientState, CompositorHandler,
            CompositorState,
        },
        seat::WaylandFocus,
        selection::{
            data_device::{set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler}, primary_selection::{set_primary_focus, PrimarySelectionHandler, PrimarySelectionState},
            SelectionHandler,
        },
        shell::wlr_layer::{
            Layer, LayerSurface as WlrLayerSurface, WlrLayerShellHandler, WlrLayerShellState,
        },
        shm::{ShmHandler, ShmState},
    }, xwayland::XWaylandClientData,
};
use tracing::error;

use crate::{
    state::{Backend, ClientState, MagmaState},
    utils::{focus::FocusTarget, tiling::bsp_update_layout, workspace::WindowElement},
};

pub mod input;
pub mod xdg_shell;

impl<BackendData: Backend> CompositorHandler for MagmaState<BackendData> {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        if let Some(state) = client.get_data::<XWaylandClientData>() {
            return &state.compositor_state;
        }
        if let Some(state) = client.get_data::<ClientState>() {
            return &state.compositor_state;
        }
        error!("Unknown client data type");
        panic!("Unknown client data type");
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
                .workspaces
                .all_windows()
                .find(|w| match w.deref() {
                    WindowElement::Wayland(w) => w.toplevel().wl_surface() == &root,
                    WindowElement::X11(x) => match x.wl_surface() {
                        Some(s) => s == root,
                        None => false,
                    },
                })
            {
                match window.deref() {
                    WindowElement::Wayland(w) => w.on_commit(),
                    WindowElement::X11(x) => x.refresh(),
                }
            }
        };
        self.popup_manager.commit(surface);
        xdg_shell::handle_commit(&self.workspaces, surface, &self.popup_manager);
    }
}

delegate_compositor!(@<BackendData: Backend + 'static> MagmaState<BackendData>);

impl<BackendData: Backend> BufferHandler for MagmaState<BackendData> {
    fn buffer_destroyed(
        &mut self,
        _buffer: &smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer,
    ) {
    }
}

impl<BackendData: Backend> ShmHandler for MagmaState<BackendData> {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

delegate_shm!(@<BackendData: Backend + 'static> MagmaState<BackendData>);

impl<BackendData: Backend> SeatHandler for MagmaState<BackendData> {
    type KeyboardFocus = FocusTarget;
    type PointerFocus = FocusTarget;

    fn seat_state(&mut self) -> &mut SeatState<MagmaState<BackendData>> {
        &mut self.seat_state
    }

    fn cursor_image(
        &mut self,
        _seat: &smithay::input::Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }
    fn focus_changed(&mut self, seat: &smithay::input::Seat<Self>, focused: Option<&FocusTarget>) {
        let dh = &self.dh;

        let focus = focused
            .and_then(WaylandFocus::wl_surface)
            .and_then(|s| dh.get_client(s.id()).ok());
        set_data_device_focus(dh, seat, focus.clone());
        set_primary_focus(dh, seat, focus);

        if let Some(focus_target) = focused {
            match focus_target {
                FocusTarget::Window(ftw) => match ftw {
                    WindowElement::Wayland(ftw_w) => {
                        for window in self.workspaces.all_windows() {
                            if let WindowElement::Wayland(w) = window.deref() {
                                w.set_activated(ftw_w.eq(&w));
                                w.toplevel().send_configure();
                            }
                        }
                    },
                    WindowElement::X11(ftw_x) => {
                        for window in self.workspaces.all_windows() {
                            if let WindowElement::X11(x) = window.deref() {
                                x.set_activated(ftw_x.eq(&x));
                                x.configure(None);
                            }
                        }
                    },
                }
                FocusTarget::LayerSurface(_) => {
                    for window in self.workspaces.all_windows() {
                        match window.deref() {
                            WindowElement::Wayland(w) => {
                                w.set_activated(false);
                                w.toplevel().send_configure();        
                            },
                            WindowElement::X11(x) => {
                                x.set_activated(false);
                                x.configure(None);        
                            },
                        }
                    }
                }
                FocusTarget::Popup(_) => {}
            };
        }
    }
}

delegate_seat!(@<BackendData: Backend + 'static> MagmaState<BackendData>);

//
// Wl Data Device
//

impl<BackendData: Backend> SelectionHandler for MagmaState<BackendData> {
    type SelectionUserData = ();
}

impl<BackendData: Backend> DataDeviceHandler for MagmaState<BackendData> {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl<BackendData: Backend> ClientDndGrabHandler for MagmaState<BackendData> {}
impl<BackendData: Backend> ServerDndGrabHandler for MagmaState<BackendData> {}

delegate_data_device!(@<BackendData: Backend + 'static> MagmaState<BackendData>);

impl<BackendData: Backend> PrimarySelectionHandler for MagmaState<BackendData> {
    fn primary_selection_state(&self) -> &PrimarySelectionState {
        &self.primary_selection_state
    }
}

delegate_primary_selection!(@<BackendData: Backend + 'static> MagmaState<BackendData>);
//
// Wl Output & Xdg Output
//

delegate_output!(@<BackendData: Backend + 'static> MagmaState<BackendData>);

impl<BackendData: Backend> WlrLayerShellHandler for MagmaState<BackendData> {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: WlrLayerSurface,
        output: Option<WlOutput>,
        _layer: Layer,
        namespace: String,
    ) {
        let output = output
            .as_ref()
            .and_then(Output::from_resource)
            .unwrap_or_else(|| self.workspaces.current().outputs().next().unwrap().clone());
        let mut map = layer_map_for_output(&output);
        let layer_surface = LayerSurface::new(surface, namespace);
        map.map_layer(&layer_surface).unwrap();
        self.set_input_focus(FocusTarget::LayerSurface(layer_surface));
        drop(map);
        for workspace in self.workspaces.iter() {
            bsp_update_layout(workspace);
        }
    }

    fn layer_destroyed(&mut self, surface: WlrLayerSurface) {
        if let Some((mut map, layer)) = self.workspaces.outputs().find_map(|o| {
            let map = layer_map_for_output(o);
            let layer = map
                .layers()
                .find(|&layer| layer.layer_surface() == &surface)
                .cloned();
            layer.map(|layer| (map, layer))
        }) {
            map.unmap_layer(&layer);
        }
        self.set_input_focus_auto();
        for workspace in self.workspaces.iter() {
            bsp_update_layout(workspace);
        }
    }
}

delegate_layer_shell!(@<BackendData: Backend + 'static> MagmaState<BackendData>);
