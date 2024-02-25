mod auth;
mod render;
mod seat;
mod shm;
mod state;
mod surface;

use render::indicator::IndicatorState;
use seat::{AppSeat, DispatchKeyEvents};
use std::time::Duration;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{
  wl_compositor, wl_output, wl_registry, wl_seat, wl_shm, wl_subcompositor, wl_surface,
};
use wayland_client::{delegate_noop, Connection, Dispatch, Proxy, QueueHandle, WaylandSource};
use wayland_protocols::ext::session_lock::v1::client::{
  ext_session_lock_manager_v1, ext_session_lock_surface_v1, ext_session_lock_v1,
};
use xkbcommon::xkb::keysyms;

use crate::auth::Authenticator;
use crate::state::AppState;
use crate::surface::AppSurface;

fn main() {
  let connection = Connection::connect_to_env().unwrap();
  let (globals, wl_queue) = registry_queue_init::<AppState>(&connection).unwrap();
  let qh = wl_queue.handle();

  // Bind globals
  let wl_compositor: wl_compositor::WlCompositor = globals.bind(&qh, 4..=4, ()).unwrap();
  let wl_subcompositor: wl_subcompositor::WlSubcompositor = globals.bind(&qh, 1..=1, ()).unwrap();
  let wl_shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).unwrap();
  let wl_seat: wl_seat::WlSeat = globals.bind(&qh, 7..=7, ()).unwrap();
  let ext_session_lock_mgr: ext_session_lock_manager_v1::ExtSessionLockManagerV1 =
    globals.bind(&qh, 1..=1, ()).unwrap();

  delegate_noop!(AppState: wl_compositor::WlCompositor);
  delegate_noop!(AppState: wl_subcompositor::WlSubcompositor);
  delegate_noop!(AppState: ignore wl_shm::WlShm); // Ignore advertise format events
  delegate_noop!(AppState: ext_session_lock_manager_v1::ExtSessionLockManagerV1);

  let wayland_queue = connection.new_event_queue::<AppState>();
  let qh = &wayland_queue.handle();

  // Request lock
  let ext_session_lock = ext_session_lock_mgr.lock(&qh, ());
  connection.roundtrip().unwrap(); // In case finished event sent by compositor

  // Bind keyboard events
  let seat = AppSeat::from(&qh, wl_seat);
  delegate_dispatch_seat!(AppState);

  // Create surface for each output
  let surfaces = globals
    .contents()
    .clone_list()
    .iter()
    .filter_map(|global| {
      if global.interface == wl_output::WlOutput::interface().name {
        let wl_output: wl_output::WlOutput = globals.bind(&qh, 4..=4, ()).unwrap();
        let surface = AppSurface::create(&qh, &wl_shm, &wl_compositor, &wl_subcompositor);
        ext_session_lock.get_lock_surface(
          surface.as_ref(),
          &wl_output,
          &qh,
          surface.as_ref().clone(),
        );
        Some(surface)
      } else {
        None
      }
    })
    .collect();
  delegate_dispatch_surface!(AppState);
  delegate_noop!(AppState: ignore wl_output::WlOutput);

  let (auth_sender, auth_channel) = calloop::channel::channel::<bool>();
  let mut main_loop =
    calloop::EventLoop::<'static, AppState>::try_new().expect("Failed to initialize event loop");
  let mut state = AppState {
    loop_handle: main_loop.handle(),
    running: true,
    locked: false,
    seat,
    surfaces,
    password: String::with_capacity(12),
    authenticator: Authenticator::new(),
    auth_sender,
    indicator_idle_timer: None,
    ext_session_lock,
    wl_shm,
    wl_compositor,
    wl_subcompositor,
  };

  // Wayland event queue
  let wayland_source = WaylandSource::new(wayland_queue).unwrap();
  wayland_source.insert(main_loop.handle()).unwrap();

  // Periodic clock redraw
  main_loop
    .handle()
    .insert_source(
      calloop::timer::Timer::immediate(),
      |event, _metadata, state| {
        for surface in state.surfaces.iter_mut() {
          surface.render_clock()
        }
        calloop::timer::TimeoutAction::ToInstant(event + Duration::from_secs(1))
      },
    )
    .unwrap();

  // Auth channel
  main_loop
    .handle()
    .insert_source(auth_channel, |event, _, state| {
      if let calloop::channel::Event::Msg(success) = event {
        if success {
          state.running = false;
        } else {
          for surface in state.surfaces.iter_mut() {
            surface.render_indicator(IndicatorState::Invalid);
            state.password.clear();
          }
        }
      }
    })
    .unwrap();

  let signal = main_loop.get_signal();
  main_loop
    .run(Duration::from_secs(1), &mut state, |state| {
      if !state.running {
        if state.locked {
          state.ext_session_lock.unlock_and_destroy()
        } else {
          state.ext_session_lock.destroy()
        }
        connection.flush().unwrap();
        signal.stop();
      }
      connection.flush().unwrap();
    })
    .expect("Error during event loop");
}

impl DispatchKeyEvents for AppState {
  fn event(state: &mut Self, keysym: xkbcommon::xkb::Keysym, codepoint: u32) {
    match keysym {
      keysyms::KEY_KP_Enter | keysyms::KEY_Return => {
        state.push_indicator_state(IndicatorState::Verifying);
        let auth_sender = state.auth_sender.clone();
        state
          .authenticator
          .authenticate(state.password.clone(), auth_sender);
      }
      keysyms::KEY_Delete | keysyms::KEY_BackSpace => {
        if state.password.pop().is_some() {
          state.push_indicator_state(IndicatorState::Input(state.password.len() as u32));
        }
      }
      _ => {
        if codepoint != 0 {
          let ch = char::from_u32(codepoint);
          if let Some(ch) = ch {
            state.password.push(ch);
            state.push_indicator_state(IndicatorState::Input(state.password.len() as u32));
          }
        }
      }
    }
  }
}

impl Dispatch<ext_session_lock_surface_v1::ExtSessionLockSurfaceV1, wl_surface::WlSurface>
  for AppState
{
  fn event(
    state: &mut Self,
    proxy: &ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,
    event: <ext_session_lock_surface_v1::ExtSessionLockSurfaceV1 as Proxy>::Event,
    data: &wl_surface::WlSurface,
    _conn: &Connection,
    _qhandle: &QueueHandle<Self>,
  ) {
    if let ext_session_lock_surface_v1::Event::Configure {
      serial,
      width,
      height,
    } = event
    {
      proxy.ack_configure(serial);
      let surface = state
        .surfaces
        .iter_mut()
        .find(|surface| surface.as_ref().id() == data.id());
      if let Some(surface) = surface {
        surface.set_dimensions(width, height);
        surface.as_ref().commit();
      }
    }
  }
}

impl Dispatch<ext_session_lock_v1::ExtSessionLockV1, ()> for AppState {
  fn event(
    state: &mut Self,
    _proxy: &ext_session_lock_v1::ExtSessionLockV1,
    event: <ext_session_lock_v1::ExtSessionLockV1 as Proxy>::Event,
    _data: &(),
    _conn: &Connection,
    _qhandle: &QueueHandle<Self>,
  ) {
    if let ext_session_lock_v1::Event::Finished = event {
      state.running = false;
    } else if let ext_session_lock_v1::Event::Locked = event {
      state.locked = true;
    }
  }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppState {
  fn event(
    _state: &mut Self,
    _proxy: &wl_registry::WlRegistry,
    _event: <wl_registry::WlRegistry as Proxy>::Event,
    _data: &GlobalListContents,
    _conn: &Connection,
    _qhandle: &QueueHandle<Self>,
  ) {
    // TODO: handle dynamically added/removed outputs
  }
}
