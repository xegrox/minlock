use std::os::fd::AsRawFd;
use wayland_client::protocol::{wl_keyboard, wl_seat};
use wayland_client::{Dispatch, WEnum, QueueHandle};
use xkbcommon::xkb::{Keymap, KEYMAP_FORMAT_TEXT_V1, Context, ffi::XKB_CONTEXT_NO_FLAGS, KEYMAP_COMPILE_NO_FLAGS, Keysym};

pub struct AppSeat {
  xkb_state: Option<xkbcommon::xkb::State>
}

impl AppSeat {
  pub fn from<D>(qh: &QueueHandle<D>, wl_seat: wl_seat::WlSeat) -> Self
  where D: 'static + Dispatch<wl_keyboard::WlKeyboard, ()> {
    wl_seat.get_keyboard(qh, ());
    Self { xkb_state: None }
  }
}

impl<State> Dispatch<wl_keyboard::WlKeyboard, (), State> for AppSeat
where
  State: Dispatch<wl_keyboard::WlKeyboard, ()>,
  State: DispatchKeyEvents,
  State: AsMut<Self> {
    fn event(
        state: &mut State,
        _proxy: &wl_keyboard::WlKeyboard,
        event: <wl_keyboard::WlKeyboard as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<State>,
    ) {
      if let wl_keyboard::Event::Keymap { format, fd, size } = event {
        if let WEnum::Value(format) = format {
          if format == wl_keyboard::KeymapFormat::XkbV1 {
            let context = Context::new(XKB_CONTEXT_NO_FLAGS);
            let keymap = unsafe {
              Keymap::new_from_fd(&context, fd.as_raw_fd(), size as usize, KEYMAP_FORMAT_TEXT_V1, KEYMAP_COMPILE_NO_FLAGS)
            }.unwrap().unwrap();
            state.as_mut().xkb_state = Some(xkbcommon::xkb::State::new(&keymap));
          } else {
            panic!("Unsupported keymap format");
          }
        } else {
          panic!("Unknown keymap format");
        }
      } else if let wl_keyboard::Event::Key { key, state: key_state, .. } = event {
        if let WEnum::Value(key_state) = key_state {
          if let wl_keyboard::KeyState::Pressed = key_state {
            if let Some(xkb_state) = state.as_mut().xkb_state.as_ref() {
              let keysym = xkb_state.key_get_one_sym(key + 8);
              let codepoint = xkb_state.key_get_utf32(key + 8);
              DispatchKeyEvents::event(state, keysym, codepoint);
            }
          }
        }
      } else if let wl_keyboard::Event::Modifiers { mods_depressed, mods_latched, mods_locked, group, .. } =  event {
        if let Some(xkb_state) = state.as_mut().xkb_state.as_mut() {
          xkb_state.update_mask(mods_depressed, mods_latched, mods_locked, 0, 0, group);
        }
      }
    }
}

pub trait DispatchKeyEvents {
  fn event(
    state: &mut Self,
    keysym: Keysym,
    codepoint: u32
  );
}

#[macro_export]
macro_rules! delegate_dispatch_seat {
  ($l: ty) => {
    wayland_client::delegate_noop!($l: ignore wayland_client::protocol::wl_seat::WlSeat);
    wayland_client::delegate_dispatch!($l: [wayland_client::protocol::wl_keyboard::WlKeyboard: ()] => AppSeat);
  };
}