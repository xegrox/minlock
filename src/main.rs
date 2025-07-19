mod application;
mod args;
mod auth;
mod render;
mod seat;
mod shm;
mod surface;

use calloop_wayland_source::WaylandSource;
use clap::Parser;
use seat::{AppSeat, DispatchKeyEvents};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_compositor, wl_output, wl_registry, wl_seat, wl_shm, wl_subcompositor, wl_surface};
use wayland_client::{delegate_noop, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::ext::session_lock::v1::client::{
  ext_session_lock_manager_v1, ext_session_lock_surface_v1, ext_session_lock_v1,
};
use xkbcommon::xkb::keysyms;

use crate::application::{AppState, Application};
use crate::args::Args;
use crate::surface::AppSurface;

struct AppProcess {
  running: bool,
  locked: bool,
}

delegate_noop!(Application: wl_compositor::WlCompositor);
delegate_noop!(Application: wl_subcompositor::WlSubcompositor);
delegate_noop!(Application: ignore wl_shm::WlShm); // Ignore advertise format events
delegate_noop!(Application: ext_session_lock_manager_v1::ExtSessionLockManagerV1);
delegate_dispatch_seat!(Application);
delegate_dispatch_surface!(Application);
delegate_noop!(Application: ignore wl_output::WlOutput);

fn main() {
  let args = Args::parse();

  let connection = Connection::connect_to_env().unwrap();
  let (globals, wl_queue) = registry_queue_init::<Application>(&connection).unwrap();
  let qh = wl_queue.handle();

  // Bind globals
  let wl_compositor: wl_compositor::WlCompositor = globals.bind(&qh, 4..=4, ()).unwrap();
  let wl_subcompositor: wl_subcompositor::WlSubcompositor = globals.bind(&qh, 1..=1, ()).unwrap();
  let wl_shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).unwrap();
  let wl_seat: wl_seat::WlSeat = globals.bind(&qh, 7..=7, ()).unwrap();
  let ext_session_lock_mgr: ext_session_lock_manager_v1::ExtSessionLockManagerV1 =
    globals.bind(&qh, 1..=1, ()).unwrap();

  let process = Arc::new(Mutex::new(AppProcess {
    running: true,
    locked: false,
  }));

  // Request lock
  let ext_session_lock = ext_session_lock_mgr.lock(&qh, Arc::clone(&process));
  connection.roundtrip().unwrap(); // In case finished event sent by compositor

  // Bind keyboard events
  let seat = AppSeat::from(&qh, wl_seat);

  // Create surface for each output
  let surfaces = globals
    .contents()
    .clone_list()
    .iter()
    .filter_map(|global| {
      if global.interface == wl_output::WlOutput::interface().name {
        let wl_output: wl_output::WlOutput = globals.bind(&qh, 4..=4, ()).unwrap();
        let surface = AppSurface::create(&qh, &wl_shm, &wl_compositor, &wl_subcompositor);
        ext_session_lock.get_lock_surface(surface.as_ref(), &wl_output, &qh, surface.as_ref().clone());
        Some(surface)
      } else {
        None
      }
    })
    .collect();

  let mut main_loop = calloop::EventLoop::<'static, Application>::try_new().expect("Failed to initialize event loop");

  let mut app = Application::new(args, main_loop.handle(), seat, surfaces);

  // Wayland event queue
  let wayland_source = WaylandSource::new(connection.clone(), wl_queue);
  wayland_source.insert(main_loop.handle()).unwrap();

  // Periodic clock redraw
  main_loop
    .handle()
    .insert_source(calloop::timer::Timer::immediate(), |event, _metadata, app| {
      for surface in app.surfaces.iter_mut() {
        surface.render_clock(
          app.args.clock_color,
          app.args.clock_font.clone(),
          app.args.clock_font_size,
          app.args.bg_color,
        );
      }
      calloop::timer::TimeoutAction::ToInstant(event + Duration::from_secs(1))
    })
    .unwrap();

  let signal = main_loop.get_signal();
  main_loop
    .run(Duration::from_secs(1), &mut app, |app| {
      let process = Arc::clone(&process);
      let mut process = process.lock().unwrap();

      // Exit once authenticated
      if matches!(app.current_state(), AppState::Success) {
        process.running = false;
      }
      // Destroy lock
      if !process.running {
        if process.locked {
          ext_session_lock.unlock_and_destroy();
        } else {
          ext_session_lock.destroy();
        }
        connection.flush().unwrap();
        signal.stop();
      }
      connection.flush().unwrap();
    })
    .expect("Error during event loop");
}

impl DispatchKeyEvents for Application {
  fn event(app: &mut Self, keysym: xkbcommon::xkb::Keysym, codepoint: u32) {
    if matches!(app.current_state(), AppState::Verifying) {
      // Block key events when verifying
      return;
    }
    match keysym {
      keysyms::KEY_Escape => {
        app.password_clear();
      }
      keysyms::KEY_KP_Enter | keysyms::KEY_Return => {
        app.authenticate();
      }
      keysyms::KEY_Delete | keysyms::KEY_BackSpace => {
        app.password_pop();
      }
      _ => {
        if codepoint != 0 {
          let ch = char::from_u32(codepoint);
          if let Some(ch) = ch {
            app.password_push(ch);
          }
        }
      }
    }
  }
}

impl Dispatch<ext_session_lock_surface_v1::ExtSessionLockSurfaceV1, wl_surface::WlSurface> for Application {
  fn event(
    app: &mut Self,
    proxy: &ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,
    event: <ext_session_lock_surface_v1::ExtSessionLockSurfaceV1 as Proxy>::Event,
    data: &wl_surface::WlSurface,
    _conn: &Connection,
    _qhandle: &QueueHandle<Self>,
  ) {
    if let ext_session_lock_surface_v1::Event::Configure { serial, width, height } = event {
      proxy.ack_configure(serial);
      let surface = app
        .surfaces
        .iter_mut()
        .find(|surface| surface.as_ref().id() == data.id());
      if let Some(surface) = surface {
        surface.set_dimensions(width, height);
        surface.render_bg(app.args.bg_color);
        surface.render_clock(
          app.args.clock_color,
          app.args.clock_font.clone(),
          app.args.clock_font_size,
          app.args.bg_color,
        );
        surface.render_indicator_full(app.args.indicator_idle_color, app.args.bg_color);
        surface.as_ref().commit();
      }
    }
  }
}

impl Dispatch<ext_session_lock_v1::ExtSessionLockV1, Arc<Mutex<AppProcess>>> for Application {
  fn event(
    _app: &mut Self,
    _proxy: &ext_session_lock_v1::ExtSessionLockV1,
    event: <ext_session_lock_v1::ExtSessionLockV1 as Proxy>::Event,
    data: &Arc<Mutex<AppProcess>>,
    _conn: &Connection,
    _qhandle: &QueueHandle<Self>,
  ) {
    let mut process = data.lock().unwrap();
    if let ext_session_lock_v1::Event::Finished = event {
      process.running = false;
    } else if let ext_session_lock_v1::Event::Locked = event {
      process.locked = true;
    }
  }
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for Application {
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
