use wayland_client::protocol::wl_output;
use wayland_protocols::ext::session_lock::v1::client::ext_session_lock_surface_v1;

use crate::surface::AppSurface;

pub struct AppOutput {
  pub wl_output: wl_output::WlOutput,
  pub ext_session_lock_surface: ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,
  pub surface: AppSurface
}

impl AppOutput {
  pub fn new(
    wl_output: wl_output::WlOutput,
    ext_session_lock_surface: ext_session_lock_surface_v1::ExtSessionLockSurfaceV1,
    surface: AppSurface) -> AppOutput {
      AppOutput {
        wl_output,
        ext_session_lock_surface,
        surface
      }
  }
}

impl Drop for AppOutput {
  fn drop(&mut self) {
    self.ext_session_lock_surface.destroy();
    self.wl_output.release();
  }
}

impl AsRef<wl_output::WlOutput> for AppOutput {
  fn as_ref(&self) -> &wl_output::WlOutput {
    &self.wl_output
  }
}


#[macro_export]
macro_rules! delegate_dispatch_output {
  ($l: ty) => {
    delegate_noop!(Application: ignore wl_output::WlOutput);
  };
}
