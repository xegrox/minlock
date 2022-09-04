use std::sync::Arc;
use wayland_client::Proxy;
use wayland_client::protocol::wl_shm;
use wayland_client::protocol::wl_surface;
use wayland_client::protocol::wl_subsurface;
use wayland_client::protocol::wl_compositor;
use wayland_client::protocol::wl_subcompositor;

use crate::render::background::draw_background;
use crate::render::clock::draw_clock;
use crate::shm::slot::BufferSlotPool;
use crate::utils::DummyObjectData;

pub struct AppSurface {
  pool: BufferSlotPool,
  width: u32,
  height: u32,
  bg_surface: wl_surface::WlSurface,
  clock_surface: wl_surface::WlSurface,
  clock_subsurface: wl_subsurface::WlSubsurface,
  clock_width: u32,
  clock_height: u32
}

impl AppSurface {
  pub fn new(
    wl_shm: &wl_shm::WlShm,
    wl_compositor: &wl_compositor::WlCompositor,
    wl_subcompositor: &wl_subcompositor::WlSubcompositor
  ) -> Self {
    let dummy_data = Arc::new(DummyObjectData);

    let bg_surface = wl_compositor.send_constructor::<wl_surface::WlSurface>(wl_compositor::Request::CreateSurface {}, dummy_data.clone()).unwrap();
    let clock_surface = wl_compositor.send_constructor::<wl_surface::WlSurface>(wl_compositor::Request::CreateSurface {}, dummy_data.clone()).unwrap();

    let clock_subsurface_req = wl_subcompositor::Request::GetSubsurface { surface: clock_surface.clone(), parent: bg_surface.clone() };
    let clock_subsurface = wl_subcompositor.send_constructor::<wl_subsurface::WlSubsurface>(clock_subsurface_req, dummy_data.clone()).unwrap();

    Self {
      pool: BufferSlotPool::create(4096, wl_shm),
      width: 0,
      height: 0,
      bg_surface,
      clock_surface,
      clock_subsurface,
      clock_height: 0,
      clock_width: 0,
    }
  }

  pub fn set_dimensions(&mut self, width: u32, height: u32) {
    if width != 0 && height != 0 && self.width != width && self.height != height {
      self.width = width;
      self.height = height;
      self.clock_width = width;
      self.clock_height = height;
      self.render_bg();
      self.render_clock();
    }
  }

  pub fn base_surface(&self) -> &wl_surface::WlSurface {&self.bg_surface}

  pub fn render_bg(&mut self) {
    if self.width == 0 || self.height == 0 {return}
    let wl_surface = &self.bg_surface;
    let buffer = draw_background(&mut self.pool, self.width, self.height);
    buffer.attach_to_surface(wl_surface);
    wl_surface.damage(0, 0, i32::MAX, i32::MAX);
    wl_surface.commit();
  }

  pub fn render_clock(&mut self) {
    if self.clock_width == 0 || self.clock_height == 0 {return}
    let wl_surface = &self.clock_surface;
    let buffer = draw_clock(&mut self.pool, self.clock_width, self.clock_height);
    buffer.attach_to_surface(wl_surface);
    wl_surface.damage(0, 0, i32::MAX, i32::MAX);
    wl_surface.commit();

    // Clock surface size changed, update position
    if buffer.width() != self.clock_width || buffer.height() != self.clock_height {
      self.clock_width = buffer.width();
      self.clock_height = buffer.height();
      let x = (self.width - self.clock_width) / 2;
      let y = (self.height - self.clock_height) / 2;
      self.clock_subsurface.set_position(x as i32, y as i32);
    }

    self.base_surface().commit();
  }
}