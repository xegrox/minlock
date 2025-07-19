use wayland_client::protocol::{wl_compositor, wl_shm, wl_subcompositor, wl_subsurface, wl_surface};
use wayland_client::Dispatch;
use wayland_client::QueueHandle;

use crate::args::Color;
use crate::render::background::draw_background;
use crate::render::clock::draw_clock;
use crate::render::indicator::{draw_indicator, INDICATOR_BLOCK_COUNT};
use crate::shm::slot::BufferSlotPool;

pub struct AppSurface {
  pool: BufferSlotPool,
  width: u32,
  height: u32,
  base_surface: wl_surface::WlSurface,
  clock_surface: wl_surface::WlSurface,
  clock_subsurface: wl_subsurface::WlSubsurface,
  clock_height: u32,
  clock_width: u32,
  indicator_surface: wl_surface::WlSurface,
  indicator_subsurface: wl_subsurface::WlSubsurface,
}

impl AppSurface {
  pub fn create<D>(
    qh: &QueueHandle<D>,
    wl_shm: &wl_shm::WlShm,
    wl_compositor: &wl_compositor::WlCompositor,
    wl_subcompositor: &wl_subcompositor::WlSubcompositor,
  ) -> Self
  where
    D: 'static + Dispatch<wl_surface::WlSurface, ()>,
    D: 'static + Dispatch<wl_subsurface::WlSubsurface, ()>,
  {
    let base_surface = wl_compositor.create_surface(qh, ());
    let clock_surface = wl_compositor.create_surface(qh, ());
    let clock_subsurface = wl_subcompositor.get_subsurface(&clock_surface, &base_surface, qh, ());
    let indicator_surface = wl_compositor.create_surface(qh, ());
    let indicator_subsurface = wl_subcompositor.get_subsurface(&indicator_surface, &base_surface, qh, ());
    Self {
      pool: BufferSlotPool::create(4096, wl_shm),
      width: 0,
      height: 0,
      base_surface,
      clock_surface,
      clock_subsurface,
      clock_width: 0,
      clock_height: 0,
      indicator_surface,
      indicator_subsurface,
    }
  }

  pub fn set_dimensions(&mut self, width: u32, height: u32) {
    if width != 0 && height != 0 && self.width != width && self.height != height {
      self.width = width;
      self.height = height;
      self.clock_width = width;
      self.clock_height = height;
    }
  }

  pub fn render_bg(&mut self, color: Color) {
    if self.width == 0 || self.height == 0 {
      return;
    }
    let buffer = draw_background(&mut self.pool, self.width, self.height, color);
    buffer.attach_to_surface(&self.base_surface);
    self.base_surface.damage(0, 0, i32::MAX, i32::MAX);
    self.base_surface.commit();
  }

  pub fn render_clock(&mut self, text_color: Color, font: String, font_size: f64, bg_color: Color) {
    if self.clock_width == 0 || self.clock_height == 0 {
      return;
    }
    let buffer = draw_clock(
      &mut self.pool,
      self.clock_width,
      self.clock_height,
      text_color,
      font,
      font_size,
      bg_color,
    );
    buffer.attach_to_surface(&self.clock_surface);
    self.clock_surface.damage(0, 0, i32::MAX, i32::MAX);
    self.clock_surface.commit();

    // Clock surface size changed, update position
    if buffer.width() != self.clock_width || buffer.height() != self.clock_height {
      self.clock_width = buffer.width();
      self.clock_height = buffer.height();
      let x = (self.width - self.clock_width) / 2;
      let y = (self.height - self.clock_height) / 2;
      self.clock_subsurface.set_position(x as i32, y as i32);
    }

    self.base_surface.commit();
  }

  pub fn render_indicator_input(
    &mut self,
    len: usize,
    cursor_color: Color,
    cursor_inc_color: Color,
    trail_color: Color,
    trail_inc_color: Color,
    bg_color: Color,
  ) {
    if len == 0 {
      self.render_indicator_full(trail_color, bg_color);
      return;
    }
    let strength = (len - 1) / INDICATOR_BLOCK_COUNT;
    let pos = (len - 1) % INDICATOR_BLOCK_COUNT;
    let block_colors = std::array::from_fn(|i| {
      if i < pos {
        add_color(trail_color, trail_inc_color, strength + 1)
      } else if i == pos {
        add_color(cursor_color, cursor_inc_color, strength)
      } else {
        add_color(trail_color, trail_inc_color, strength)
      }
    });
    self.render_indicator(block_colors, bg_color)
  }

  pub fn render_indicator_full(&mut self, color: Color, bg_color: Color) {
    self.render_indicator([color; INDICATOR_BLOCK_COUNT], bg_color);
  }

  fn render_indicator(&mut self, block_colors: [Color; INDICATOR_BLOCK_COUNT], bg_color: Color) {
    let buffer = draw_indicator(&mut self.pool, block_colors, bg_color);
    buffer.attach_to_surface(&self.indicator_surface);
    self.indicator_surface.damage(0, 0, i32::MAX, i32::MAX);
    self.indicator_surface.commit();
    let x = (self.width - buffer.width()) / 2;
    let y = (self.height - self.clock_height) / 2 + self.clock_height + 20;
    self.indicator_subsurface.set_position(x as i32, y as i32);
    self.base_surface.commit();
  }
}

impl Drop for AppSurface {
  fn drop(&mut self) {
    self.indicator_subsurface.destroy();
    self.indicator_surface.destroy();
    self.clock_subsurface.destroy();
    self.clock_surface.destroy();
    self.base_surface.destroy();
  }
}

impl AsRef<wl_surface::WlSurface> for AppSurface {
  fn as_ref(&self) -> &wl_surface::WlSurface {
    &self.base_surface
  }
}

fn add_color(color: Color, inc_color: Color, strength: usize) -> Color {
  Color {
    r: color.r + inc_color.r * strength as f64,
    g: color.g + inc_color.g * strength as f64,
    b: color.b + inc_color.b * strength as f64,
  }
}

#[macro_export]
macro_rules! delegate_dispatch_surface {
  ($l: ty) => {
    wayland_client::delegate_noop!($l: ignore wayland_client::protocol::wl_surface::WlSurface);
    wayland_client::delegate_noop!($l: ignore wayland_client::protocol::wl_subsurface::WlSubsurface);
  };
}
