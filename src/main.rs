mod shm;
mod render;
mod surface;
mod seat;
mod password;

use password::PasswordBuffer;
use render::indicator::IndicatorState;
use seat::{AppSeat, DispatchKeyEvents};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::{delegate_dispatch, WaylandSource, delegate_noop};
use wayland_client::protocol::{wl_keyboard, wl_registry, wl_compositor, wl_subcompositor, wl_shm, wl_seat, wl_surface, wl_subsurface};
use wayland_client::{delegate_dispatch, WaylandSource};
use wayland_client::protocol::wl_keyboard;
use wayland_client::protocol::wl_seat::WlSeat;
use xkbcommon::xkb::keysyms;
use std::sync::{Mutex, Arc};
use std::thread;
use std::time::Duration;
use surface::AppSurface;
use wayland_client::{Proxy, Dispatch, Connection, QueueHandle};
use wayland_protocols_wlr::layer_shell::v1::client::{zwlr_layer_surface_v1, zwlr_layer_shell_v1};

pub struct AppState {
  loop_handle: calloop::LoopHandle<'static, Self>,
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
      |s| &mut s.surface
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
  let (globals, wl_queue) = registry_queue_init::<AppState>(&connection).unwrap();
  let qh = wl_queue.handle();

  // Bind globals
  let wl_compositor: wl_compositor::WlCompositor = globals.bind(&qh, 4..=4, ()).unwrap();
  let wl_subcompositor: wl_subcompositor::WlSubcompositor = globals.bind(&qh, 1..=1, ()).unwrap();
  let wl_shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).unwrap();
  let wl_seat: wl_seat::WlSeat = globals.bind(&qh, 7..=7, ()).unwrap();
  let zwlr_layer_shell: zwlr_layer_shell_v1::ZwlrLayerShellV1 = globals.bind(&qh, 1..=1, ()).unwrap();
  let wl_compositor = gm.instantiate::<WlCompositor>(4).unwrap();
  let wl_subcompositor = gm.instantiate::<WlSubcompositor>(1).unwrap();
  let wl_shm = gm.instantiate::<WlShm>(1).unwrap();
  let wl_seat = gm.instantiate::<WlSeat>(7).unwrap();
  let zwlr_layer_shell = gm.instantiate::<zwlr_layer_shell_v1::ZwlrLayerShellV1>(1).unwrap();

  delegate_noop!(AppState: wl_compositor::WlCompositor);
  delegate_noop!(AppState: wl_subcompositor::WlSubcompositor);
  delegate_noop!(AppState: ignore wl_shm::WlShm); // Ignore advertise format events
  delegate_noop!(AppState: ignore wl_seat::WlSeat); // Ignore capabilities changes
  delegate_noop!(AppState: zwlr_layer_shell_v1::ZwlrLayerShellV1);
  
  let wayland_queue = connection.new_event_queue::<AppState>();
  let qh = &wayland_queue.handle();
  
  // Keyboard events
  wl_seat.get_keyboard(qh, ());

  // Create surface
  let surface = AppSurface::new(&qh, &wl_shm, &wl_compositor, &wl_subcompositor);
  delegate_noop!(AppState: ignore wl_surface::WlSurface);
  delegate_noop!(AppState: ignore wl_subsurface::WlSubsurface);
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
  let mut main_loop = calloop::EventLoop::<'static, AppState>::try_new().expect("Failed to initialize event loop");
  let mut state = AppState {
    loop_handle: main_loop.handle(),
    running: true,
    seat: AppSeat::new(),
    surface,
    password: Arc::new(Mutex::new(PasswordBuffer::create())),
    auth_sender
  };

  // Wayland event queue
  let wayland_source = WaylandSource::new(wayland_queue).unwrap();
  wayland_source.insert(main_loop.handle()).unwrap();

  // Periodic clock damaging
  main_loop.handle().insert_source(calloop::timer::Timer::immediate(), |event, _metadata, state| {
    state.surface.render_clock();
    calloop::timer::TimeoutAction::ToInstant(event + Duration::from_secs(1))
  }).unwrap();

  // Auth channel
  main_loop.handle().insert_source(auth_channel, |event, _, state| {
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
    &mut state,
    |state| {
      if !state.running {signal.stop();}
      connection.flush().unwrap();
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
      |s| &mut s.surface
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

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppState {
    fn event(
        state: &mut Self,
        proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as Proxy>::Event,
        data: &GlobalListContents,
        conn: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        // TODO: handle new outputs
    }
}