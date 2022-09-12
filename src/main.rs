mod shm;
mod globals;
mod render;
mod surface;
mod seat;
mod utils;
mod password;

use password::{PasswordBuffer, PasswordBufferEvent};
use seat::{AppSeat, DispatchKeyEvents};
use wayland_client::{EventQueue, delegate_dispatch};
use wayland_client::protocol::wl_keyboard;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_protocols_wlr::input_inhibitor::v1::client::{zwlr_input_inhibit_manager_v1, zwlr_input_inhibitor_v1};
use xkbcommon::xkb::keysyms;
use std::time::Duration;
use globals::GlobalsManager;
use surface::AppSurface;
use wayland_client::{Proxy, Dispatch, Connection, QueueHandle};
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_subcompositor::WlSubcompositor;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_surface_v1, zwlr_layer_shell_v1};
 
pub struct AppState {
  loop_handle: calloop::LoopHandle<'static, (EventQueue::<AppState>, Self)>,
  running: bool,
  seat: AppSeat,
  surface: AppSurface,
  password: PasswordBuffer,
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

  let mut main_loop = calloop::EventLoop::<'static, (EventQueue::<AppState>, AppState)>::try_new().expect("Failed to initialize event loop");
  let state = AppState {
    loop_handle: main_loop.handle(),
    running: true,
    seat: AppSeat::new(),
    surface,
    password: PasswordBuffer::create(),
  };
  

  let event_queue_fd = event_queue.prepare_read().unwrap().connection_fd();
  let event_queue_source = calloop::generic::Generic::new(event_queue_fd, calloop::Interest::READ, calloop::Mode::Level);
  main_loop.handle().insert_source(event_queue_source, |_event, _metadata, (queue, state)| {
    queue.prepare_read().unwrap().read().unwrap();
    queue.dispatch_pending(state).unwrap();
    Ok(calloop::PostAction::Continue)
  }).unwrap();

  main_loop.handle().insert_source(calloop::timer::Timer::immediate(), |event, _metadata, (_queue, state)| {
    state.surface.render_clock();
    calloop::timer::TimeoutAction::ToInstant(event + Duration::from_secs(1))
  }).unwrap();

  let signal = main_loop.get_signal();
  main_loop.run(
    Duration::from_secs(1), 
    &mut (event_queue, state),
    |(queue, state)| {
      if !state.running {signal.stop();}
      queue.flush().unwrap();
    }
  ).expect("Error during event loop");

}

delegate_dispatch!(AppState: [wl_keyboard::WlKeyboard: ()] => AppSeat);

impl AsMut<AppSeat> for AppState {
  fn as_mut(&mut self) -> &mut AppSeat { &mut self.seat }
}

impl DispatchKeyEvents for AppState {
  fn event(
    state: &mut Self,
    keysym: xkbcommon::xkb::Keysym,
    codepoint: u32
  ) {
    let mut update_indicator = |event: PasswordBufferEvent| {
      state.surface.indicator_event(
        event, 
        state.loop_handle.clone(),
        |s| {&mut s.1.surface}
      );
    };
    match keysym {
      keysyms::KEY_Escape => {
        state.running = false;
      },
      keysyms::KEY_Delete | keysyms::KEY_BackSpace => {
        let event = state.password.pop();
        update_indicator(event);
      },
      _ => {
        if codepoint != 0 {
          let ch = char::from_u32(codepoint);
          if let Some(ch) = ch {
            let event = state.password.push(ch);
            update_indicator(event);
          }
        }
      }
    }
  }
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