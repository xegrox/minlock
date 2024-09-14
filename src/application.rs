use crate::args::Args;
use crate::auth::Authenticator;
use crate::seat::AppSeat;
use crate::surface::AppSurface;
use std::time::Duration;

pub struct Application {
  pub args: Args,
  pub locked: bool,
  pub seat: AppSeat,
  pub surfaces: Vec<AppSurface>,

  loop_handle: calloop::LoopHandle<'static, Self>,
  state: AppState,
  indicator_idle_timer: Option<calloop::RegistrationToken>,
  authenticator: Authenticator,
  auth_sender: calloop::channel::Sender<bool>,
  password: String,
}

#[derive(Clone, Copy)]
pub enum AppState {
  Success,
  Invalid,
  Verifying,
  Input,
  Idle,
}

impl Application {
  pub fn new(
    args: Args,
    loop_handle: calloop::LoopHandle<'static, Self>,
    seat: AppSeat,
    surfaces: Vec<AppSurface>,
  ) -> Application {
    // Auth channel
    let (auth_sender, auth_channel) = calloop::channel::channel::<bool>();
    loop_handle
      .insert_source(auth_channel, |event, _, app| {
        if let calloop::channel::Event::Msg(success) = event {
          if success {
            app.push_state(AppState::Success);
          } else {
            app.push_state(AppState::Invalid);
            app.password.clear();
          }
        }
      })
      .unwrap();

    Application {
      args,
      loop_handle,
      locked: false,
      seat,
      surfaces,
      state: AppState::Idle,
      password: String::with_capacity(12),
      authenticator: Authenticator::new(),
      auth_sender,
      indicator_idle_timer: None,
    }
  }

  pub fn password_clear(&mut self) {
    self.password.clear();
    self.push_state(AppState::Input);
  }

  pub fn password_push(&mut self, ch: char) {
    self.password.push(ch);
    self.push_state(AppState::Input);
  }

  pub fn password_pop(&mut self) {
    if self.password.pop().is_some() {
      self.push_state(AppState::Input);
    }
  }

  pub fn authenticate(&mut self) {
    self.push_state(AppState::Verifying);
    self
      .authenticator
      .authenticate(self.password.clone(), self.auth_sender.clone());
  }

  pub fn current_state(&self) -> AppState {
    self.state
  }

  fn push_state(&mut self, state: AppState) {
    self.state = state;
    for surface in self.surfaces.iter_mut() {
      match state {
        AppState::Success => surface.render_indicator_full(self.args.indicator_idle_color, self.args.bg_color),
        AppState::Idle => surface.render_indicator_full(self.args.indicator_idle_color, self.args.bg_color),
        AppState::Invalid => surface.render_indicator_full(self.args.indicator_wrong_color, self.args.bg_color),
        AppState::Verifying => surface.render_indicator_full(self.args.indicator_verifying_color, self.args.bg_color),
        AppState::Input => {
          if self.password.len() == 0 {
            surface.render_indicator_full(self.args.indicator_clear_color, self.args.bg_color)
          } else {
            surface.render_indicator_input(
              self.password.len(),
              self.args.indicator_input_cursor_color,
              self.args.indicator_input_cursor_increment_color,
              self.args.indicator_input_trail_color,
              self.args.indicator_input_trail_increment_color,
              self.args.bg_color,
            )
          }
        }
      };
    }
    // Reset idle timer
    if let Some(timer) = self.indicator_idle_timer {
      self.loop_handle.remove(timer);
    }
    if !matches!(state, AppState::Verifying) {
      self.indicator_idle_timer = Some(
        self
          .loop_handle
          .insert_source(
            calloop::timer::Timer::from_duration(Duration::from_secs(2)),
            |_, _, app| {
              for surface in app.surfaces.iter_mut() {
                app.state = AppState::Idle;
                surface.render_indicator_full(app.args.indicator_idle_color, app.args.bg_color);
              }
              calloop::timer::TimeoutAction::Drop
            },
          )
          .unwrap(),
      );
    }
  }
}
