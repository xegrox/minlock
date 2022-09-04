mod shm;
mod globals;
mod render;
mod surface;
mod utils;

use wayland_client::EventQueue;
use wayland_client::protocol::wl_keyboard;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_protocols_wlr::input_inhibitor::v1::client::{zwlr_input_inhibit_manager_v1, zwlr_input_inhibitor_v1};
use std::time::Duration;
use globals::GlobalsManager;
use surface::AppSurface;
use wayland_client::{Proxy, Dispatch, Connection, QueueHandle};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_subcompositor::WlSubcompositor;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_surface_v1, zwlr_layer_shell_v1};
 
pub struct AppState {
  running: bool,
  pub surface: AppSurface
}

fn main() {
  let connection = Connection::connect_to_env().unwrap();
  let gm = GlobalsManager::new(&connection);
  
  // Bind globals
  let wl_compositor = gm.instantiate::<WlCompositor>(4).unwrap();
  let wl_subcompositor = gm.instantiate::<WlSubcompositor>(1).unwrap();
  let wl_shm = gm.instantiate::<WlShm>(1).unwrap();
  let wl_seat = gm.instantiate::<WlSeat>(7).unwrap();
  let zwlr_layer_shell = gm.instantiate::<zwlr_layer_shell_v1::ZwlrLayerShellV1>(1).unwrap();
  let zwlr_input_inhibit_manager_v1 = gm.instantiate::<zwlr_input_inhibit_manager_v1::ZwlrInputInhibitManagerV1>(1).unwrap();
  
  let event_queue = connection.new_event_queue::<AppState>();
  let qh = &event_queue.handle();
  
  // Keyboard events
  zwlr_input_inhibit_manager_v1.get_inhibitor(qh, ()).unwrap();
  wl_seat.get_keyboard(qh, ()).unwrap();

  // Create surface
  let surface = AppSurface::new(&wl_shm, &wl_compositor, &wl_subcompositor);
  let layer_surface = zwlr_layer_shell.get_layer_surface(
    surface.base_surface(),
    None,
    zwlr_layer_shell_v1::Layer::Overlay,
    "lockscreen".to_string(),
    qh,
    ()
  ).unwrap();
  layer_surface.set_anchor(
    zwlr_layer_surface_v1::Anchor::Top |
    zwlr_layer_surface_v1::Anchor::Bottom |
    zwlr_layer_surface_v1::Anchor::Right |
    zwlr_layer_surface_v1::Anchor::Left
  );
  layer_surface.set_exclusive_zone(-1);
  layer_surface.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive);
  surface.base_surface().commit();

  let state = AppState {
    running: true,
    surface
  };  
  
  let mut main_loop = calloop::EventLoop::<(EventQueue::<AppState>, AppState, calloop::LoopSignal)>::try_new().expect("Failed to initialize event loop");
  let main_loop_signal = main_loop.get_signal();

  let event_queue_fd = event_queue.prepare_read().unwrap().connection_fd();
  let event_queue_source = calloop::generic::Generic::new(event_queue_fd, calloop::Interest::READ, calloop::Mode::Level);
  main_loop.handle().insert_source(event_queue_source, |_event, _metadata, (queue, state, _signal)| {
    queue.prepare_read().unwrap().read().unwrap();
    queue.dispatch_pending(state).unwrap();
    Ok(calloop::PostAction::Continue)
  }).unwrap();

  main_loop.handle().insert_source(calloop::timer::Timer::immediate(), |event, _metadata, (_queue, state, _signal)| {
    state.surface.render_clock();
    calloop::timer::TimeoutAction::ToInstant(event + Duration::from_secs(1))
  }).unwrap();

  main_loop.run(
    Duration::from_secs(1), 
    &mut (event_queue, state, main_loop_signal),
    | (queue, state, signal)| {
      if !state.running {signal.stop();}
      queue.flush().unwrap();
    }
  ).expect("Error during event loop");

}

impl Dispatch<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1, ()> for AppState {
  fn event(
      state: &mut Self,
      proxy: &zwlr_layer_surface_v1::ZwlrLayerSurfaceV1,
      event: <zwlr_layer_surface_v1::ZwlrLayerSurfaceV1 as Proxy>::Event,
      _data: &(),
      _conn: &wayland_client::Connection,
      _qhandle: &QueueHandle<Self>,
  ) {
      if let zwlr_layer_surface_v1::Event::Configure { serial, width, height } = event {
        proxy.ack_configure(serial);
        state.surface.set_dimensions(width, height);
      } else if let zwlr_layer_surface_v1::Event::Closed = event {
        state.running = false;
      }
  }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &wl_keyboard::WlKeyboard,
        event: <wl_keyboard::WlKeyboard as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key { key, .. } = event {
          if key == 1 {
            state.running = false;
          }
        }
    }
}

impl Dispatch::<zwlr_input_inhibitor_v1::ZwlrInputInhibitorV1, ()> for AppState {
    fn event(
        _state: &mut Self,
        _proxy: &zwlr_input_inhibitor_v1::ZwlrInputInhibitorV1,
        _event: <zwlr_input_inhibitor_v1::ZwlrInputInhibitorV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        // Do nothing
    }
}