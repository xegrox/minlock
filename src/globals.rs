use std::collections::HashMap;
use std::sync::Arc;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_client::protocol::wl_subcompositor::WlSubcompositor;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_manager_v1::ExtSessionLockManagerV1;
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::ZwlrLayerShellV1;

use crate::utils::DummyObjectData;

struct GlobalEntry {
  name: u32,
  version: u32
}

type GlobalEntries = HashMap<String, GlobalEntry>;

pub struct GlobalsManager {
  wl_registry: WlRegistry,
  globals: GlobalEntries
}

impl GlobalsManager {
  pub fn new(connection: &Connection) -> Self {
    let wl_display = connection.display();
    let mut queue = connection.new_event_queue::<Self>();
    let wl_registry = wl_display.get_registry(&queue.handle(), ());
    let mut gm = Self {globals: HashMap::new(), wl_registry};
    queue.roundtrip(&mut gm).unwrap();
    gm
  }

  pub fn instantiate<I>(&self, min_ver: u32) -> Result<I, InstantiateError>
  where
    I: Proxy + InterfaceName + 'static {
    if let Some(GlobalEntry { name, version }) = self.globals.get(I::NAME) {
      if min_ver <= *version {
        let request = wl_registry::Request::Bind {
          name: *name,
          id: (I::interface(), min_ver)
        };
        Ok(self.wl_registry.send_constructor::<I>(request, Arc::new(DummyObjectData)).unwrap())
      } else {
        Err(InstantiateError::InvalidVersion(*version))
      }
    } else {
      Err(InstantiateError::NotFound)
    }
  }

  pub fn instantiate_qh<I, U, D>(&self, min_ver: u32, qh: &QueueHandle<D>, udata: U) -> Result<I, InstantiateError>
  where
    I: Proxy + InterfaceName + 'static,
    D: Dispatch<I, U> + 'static,
    U: Send + Sync + 'static {
    if let Some(GlobalEntry { name, version }) = self.globals.get(I::NAME) {
      if min_ver <= *version {
        Ok(self.wl_registry.bind::<I, U, D>(*name, *version, qh, udata))
      } else {
        Err(InstantiateError::InvalidVersion(*version))
      }
    } else {
      Err(InstantiateError::NotFound)
    }
  }
}

impl Dispatch<wl_registry::WlRegistry, ()> for GlobalsManager {
    fn event(
        state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
          state.globals.insert(interface.to_string(), GlobalEntry { name, version });
        }
    }
}

#[derive(Debug)]
pub enum InstantiateError {
  NotFound,
  InvalidVersion(u32)
}

impl std::error::Error for InstantiateError {}
impl std::fmt::Display for InstantiateError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{:?}", self)
  }
}

pub trait InterfaceName {
  const NAME: &'static str;
}

impl InterfaceName for WlCompositor {
  const NAME: &'static str = "wl_compositor";
}

impl InterfaceName for WlSubcompositor {
  const NAME: &'static str = "wl_subcompositor";
}

impl InterfaceName for WlShm {
  const NAME: &'static str = "wl_shm";
}

impl InterfaceName for ZwlrLayerShellV1 {
  const NAME: &'static str = "zwlr_layer_shell_v1";
}

impl InterfaceName for WlSeat {
  const NAME: &'static str = "wl_seat";
}

impl InterfaceName for ExtSessionLockManagerV1 {
  const NAME: &'static str = "ext_session_lock_manager_v1";
}