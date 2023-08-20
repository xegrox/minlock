mod shm;
mod globals;
mod render;
mod surface;
mod seat;
mod utils;
mod password;

use password::PasswordBuffer;
use render::indicator::IndicatorState;
use seat::{AppSeat, DispatchKeyEvents};
use wayland_client::{EventQueue, delegate_dispatch};
use wayland_client::protocol::wl_keyboard;
use wayland_client::protocol::wl_seat::WlSeat;
use xkbcommon::xkb::keysyms;
use std::os::fd::AsRawFd;
use std::sync::{Mutex, Arc};
use std::thread;
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
  password: Arc<Mutex<PasswordBuffer>>,
  auth_sender: calloop::channel::Sender<bool>
}

impl AppState {
  fn push_surface_state(&mut self, surface_state: IndicatorState) {
    self.surface.push_state(
      surface_state,
      self.loop_handle.clone(),
      |s| &mut s.1.surface
    );
  }

  fn authenticate(&mut self) {
    let password = Arc::clone(&self.password);
    let auth_sender = self.auth_sender.clone();
    thread::spawn(move || {
      if let Ok(pwd) = password.lock().as_mut() {
        let result = pwd.authenticate();
        if !result {
          pwd.clear()
        }
        auth_sender.send(result).unwrap();
      }
    });
  }
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
  
  let event_queue = connection.new_event_queue::<AppState>();
  let qh = &event_queue.handle();
  
  // Keyboard events
  wl_seat.get_keyboard(qh, ());

  // Create surface
  let surface = AppSurface::new(&wl_shm, &wl_compositor, &wl_subcompositor);
  let layer_surface = zwlr_layer_shell.get_layer_surface(
    surface.base_surface(),
    None,
    zwlr_layer_shell_v1::Layer::Overlay,
    "lockscreen".to_string(),
    qh,
    ()
  );
  layer_surface.set_anchor(
    zwlr_layer_surface_v1::Anchor::Top |
    zwlr_layer_surface_v1::Anchor::Bottom |
    zwlr_layer_surface_v1::Anchor::Right |
    zwlr_layer_surface_v1::Anchor::Left
  );
  layer_surface.set_exclusive_zone(-1);
  layer_surface.set_keyboard_interactivity(zwlr_layer_surface_v1::KeyboardInteractivity::Exclusive);
  surface.base_surface().commit();

  let (auth_sender, auth_channel) = calloop::channel::channel::<bool>();
  let mut main_loop = calloop::EventLoop::<'static, (EventQueue::<AppState>, AppState)>::try_new().expect("Failed to initialize event loop");
  let state = AppState {
    loop_handle: main_loop.handle(),
    running: true,
    seat: AppSeat::new(),
    surface,
    password: Arc::new(Mutex::new(PasswordBuffer::create())),
    auth_sender
  };

  let wayland_fd = connection.prepare_read().unwrap().connection_fd().as_raw_fd();
  let wayland_source = calloop::generic::Generic::new(wayland_fd, calloop::Interest::READ, calloop::Mode::Level);
  main_loop.handle().insert_source(wayland_source, |_event, _metadata, (queue, state)| {
    queue.prepare_read().unwrap().read().unwrap();
    queue.dispatch_pending(state).unwrap();
    Ok(calloop::PostAction::Continue)
  }).unwrap();

  main_loop.handle().insert_source(calloop::timer::Timer::immediate(), |event, _metadata, (_queue, state)| {
    state.surface.render_clock();
    calloop::timer::TimeoutAction::ToInstant(event + Duration::from_secs(1))
  }).unwrap();

  main_loop.handle().insert_source(auth_channel, |event, _, (_, state)| {
    if let calloop::channel::Event::Msg(success) = event {
      if success {
        state.running = false;
      } else {
        state.push_surface_state(IndicatorState::Invalid);
      }
    }
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
    let mut push_state = |indicator_state: IndicatorState| state.surface.push_state(
      indicator_state, 
      state.loop_handle.clone(),
      |s| &mut s.1.surface
    );
    match keysym {
      keysyms::KEY_Escape => {
        state.running = false;
      },
      keysyms::KEY_KP_Enter | keysyms::KEY_Return => {
        push_state(IndicatorState::Verifying);
        state.authenticate();
      },
      keysyms::KEY_Delete | keysyms::KEY_BackSpace => {
        if let Ok(password) = state.password.try_lock().as_mut() {
          let success = password.pop();
          if success {
            push_state(IndicatorState::Input(password.len() as u32));
          }
        }
      },
      _ => {
        if let Ok(password) = state.password.try_lock().as_mut() {
          if codepoint != 0 {
            let ch = char::from_u32(codepoint);
            if let Some(ch) = ch {
              password.push(ch);
              push_state(IndicatorState::Input(password.len() as u32));
            }
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