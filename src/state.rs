use std::time::Duration;
use wayland_client::protocol::{wl_compositor, wl_subcompositor, wl_shm};
use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_v1;

use crate::{seat::AppSeat, surface::AppSurface, render::indicator::IndicatorState, auth::Authenticator};

pub struct AppState {
  pub loop_handle: calloop::LoopHandle<'static, Self>,
  pub running: bool,
  pub seat: AppSeat,
  pub surfaces: Vec<AppSurface>,
  pub password: String,
  pub authenticator: Authenticator,
  pub auth_sender: calloop::channel::Sender<bool>,
  pub locked: bool,
  pub indicator_idle_timer: Option<calloop::RegistrationToken>,

  pub ext_session_lock: ext_session_lock_v1::ExtSessionLockV1,
  pub wl_shm: wl_shm::WlShm,
  pub wl_compositor: wl_compositor::WlCompositor,
  pub wl_subcompositor: wl_subcompositor::WlSubcompositor,
}

impl AsMut<AppSeat> for AppState {
  fn as_mut(&mut self) -> &mut AppSeat {
    &mut self.seat
  }
}

impl AppState {

  // pub fn authenticate(&mut self) {
  //   let password = Arc::clone(&self.password);
  //   let auth_sender = self.auth_sender.clone();
  //   thread::spawn(move || {
  //     if let Ok(pwd) = password.lock().as_mut() {
  //       let result = pwd.authenticate();
  //       if !result {
  //         pwd.clear()
  //       }
  //       auth_sender.send(result).unwrap();
  //     }
  //   });
  // }

  pub fn push_indicator_state(&mut self, indicator_state: IndicatorState) {
    for surface in self.surfaces.iter_mut() {
      surface.render_indicator(indicator_state);
    }
    if let Some(timer) = self.indicator_idle_timer {
      self.loop_handle.remove(timer);
    }
    if !matches!(indicator_state, IndicatorState::Verifying) {
      self.indicator_idle_timer = Some(self.loop_handle.insert_source(calloop::timer::Timer::from_duration(Duration::from_secs(2)), |_, _, state| {
        for surface in state.surfaces.iter_mut() {
          surface.render_indicator(IndicatorState::Idle);
        }
        calloop::timer::TimeoutAction::Drop
      }).unwrap());
    }
  }
}