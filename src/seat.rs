use std::os::fd::AsRawFd;
use wayland_client::protocol::{wl_keyboard, wl_pointer, wl_seat};
use wayland_client::{Dispatch, QueueHandle, WEnum};
use xkbcommon::xkb::{
  ffi::XKB_CONTEXT_NO_FLAGS, Context, Keymap, Keysym, KEYMAP_COMPILE_NO_FLAGS, KEYMAP_FORMAT_TEXT_V1,
};

pub struct AppSeat {
  xkb_state: Option<xkbcommon::xkb::State>,
  wl_keyboard: Option<wl_keyboard::WlKeyboard>,
  wl_pointer: Option<wl_pointer::WlPointer>
}

impl AppSeat {
  pub fn from<D>(_qh: &QueueHandle<D>, _wl_seat: wl_seat::WlSeat) -> Self
  {
    Self { xkb_state: None, wl_keyboard: None, wl_pointer: None }
  }
}

pub trait DispatchKeyEvents {
  fn event(state: &mut Self, keysym: Keysym, codepoint: u32);
}

impl<State> Dispatch<wl_seat::WlSeat, (), State> for AppSeat
where
  State: Dispatch<wl_seat::WlSeat, ()> + 'static,
  State: Dispatch<wl_keyboard::WlKeyboard, ()>,
  State: Dispatch<wl_pointer::WlPointer, ()>,
  State: AsMut<Self>,
{
  fn event(
    state: &mut State,
    proxy: &wl_seat::WlSeat,
    event: <wl_seat::WlSeat as wayland_client::Proxy>::Event,
    _data: &(),
    _conn: &wayland_client::Connection,
    qhandle: &QueueHandle<State>,
  ) {
    if let wl_seat::Event::Capabilities { capabilities } = event {
      if let WEnum::Value(capabilities) = capabilities {
        state.as_mut().wl_keyboard.as_ref().map(|v| v.release());
        state.as_mut().wl_keyboard = None;
        state.as_mut().wl_pointer.as_ref().map(|v| v.release());
        state.as_mut().wl_pointer = None;
        if capabilities.contains(wl_seat::Capability::Keyboard) {
          println!("keyboard");
          state.as_mut().wl_keyboard = Some(proxy.get_keyboard(qhandle, ()));
        }
        if capabilities.contains(wl_seat::Capability::Pointer) {
          state.as_mut().wl_pointer = Some(proxy.get_pointer(qhandle, ()));
        }
      }
    }
  }
}

impl<State> Dispatch<wl_keyboard::WlKeyboard, (), State> for AppSeat
where
  State: Dispatch<wl_keyboard::WlKeyboard, ()>,
  State: DispatchKeyEvents,
  State: AsMut<Self>,
{
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
            Keymap::new_from_fd(
              &context,
              fd.as_raw_fd(),
              size as usize,
              KEYMAP_FORMAT_TEXT_V1,
              KEYMAP_COMPILE_NO_FLAGS,
            )
          }
          .unwrap()
          .unwrap();
          state.as_mut().xkb_state = Some(xkbcommon::xkb::State::new(&keymap));
        } else {
          panic!("Unsupported keymap format");
        }
      } else {
        panic!("Unknown keymap format");
      }
    } else if let wl_keyboard::Event::Key {
      key, state: key_state, ..
    } = event
    {
      if let WEnum::Value(key_state) = key_state {
        if let wl_keyboard::KeyState::Pressed = key_state {
          if let Some(xkb_state) = state.as_mut().xkb_state.as_ref() {
            let keysym = xkb_state.key_get_one_sym(key + 8);
            let codepoint = xkb_state.key_get_utf32(key + 8);
            DispatchKeyEvents::event(state, keysym, codepoint);
          }
        }
      }
    } else if let wl_keyboard::Event::Modifiers {
      mods_depressed,
      mods_latched,
      mods_locked,
      group,
      ..
    } = event
    {
      if let Some(xkb_state) = state.as_mut().xkb_state.as_mut() {
        xkb_state.update_mask(mods_depressed, mods_latched, mods_locked, 0, 0, group);
      }
    }
  }
}

impl<State> Dispatch<wl_pointer::WlPointer, (), State> for AppSeat
where
  State: Dispatch<wl_pointer::WlPointer, ()>,
  State: AsMut<Self>,
{
  fn event(
    _state: &mut State,
    proxy: &wl_pointer::WlPointer,
    event: <wl_pointer::WlPointer as wayland_client::Proxy>::Event,
    _data: &(),
    _conn: &wayland_client::Connection,
    _qhandle: &QueueHandle<State>,
  ) {
    if let wl_pointer::Event::Enter { serial, .. } = event {
      proxy.set_cursor(serial, None, 0, 0);
    }
  }
}

#[macro_export]
macro_rules! delegate_dispatch_seat {
  ($l: ty) => {

    impl AsMut<AppSeat> for $l {
      fn as_mut(&mut self) -> &mut AppSeat {
        &mut self.seat
      }
    }

    wayland_client::delegate_dispatch!($l: [wayland_client::protocol::wl_seat::WlSeat: ()] => AppSeat);
    wayland_client::delegate_dispatch!($l: [wayland_client::protocol::wl_keyboard::WlKeyboard: ()] => AppSeat);
    wayland_client::delegate_dispatch!($l: [wayland_client::protocol::wl_pointer::WlPointer: ()] => AppSeat);
  };
}
