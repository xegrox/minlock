use std::{sync::{Mutex, Arc}, marker::PhantomData};

use calloop::LoopHandle;
use xkbcommon::xkb::keysyms;

use crate::{render::indicator::IndicatorState, password::PasswordBuffer};

pub enum AppKeymapEvents {
  Terminate,
  IndicatorState(IndicatorState)
}

struct AppKeyboard<S> {
  auth_sender: calloop::channel::Sender<bool>,
  _m: PhantomData<S>
}

impl<S> AppKeyboard<S> {
  pub fn new<'a, L>(
    state: &S,
    get_state: fn(&mut L) -> &mut S
  ) -> Self 
  where
    L: 'static,
    S: AsRef<Arc<Mutex<PasswordBuffer>>> + AsRef<LoopHandle<'static, L>> + DispatchKeymapEvents + 'static
  {
    let (auth_sender, auth_channel) = calloop::channel::channel::<bool>();
    AsRef::<LoopHandle<'static, L>>::as_ref(state).insert_source(auth_channel, move |event, _, l| {
      if let calloop::channel::Event::Msg(success) = event {
        let state = get_state(l);
        if success {
          DispatchKeymapEvents::event(state, AppKeymapEvents::Terminate);
        } else {
          let event = AppKeymapEvents::IndicatorState(IndicatorState::Invalid);
          DispatchKeymapEvents::event(state, event);
        }
      }
    }).unwrap();
    AppKeyboard { auth_sender }
  }

  pub fn handle_key(&self, keysym: xkbcommon::xkb::Keysym) {
    match keysym {
      keysyms::KEY_Escape => {
        DispatchKeymapEvents::event(state, AppKeymapEvents::Terminate);
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

pub trait DispatchKeymapEvents {
  fn event(state: &mut Self, event: AppKeymapEvents);
}

