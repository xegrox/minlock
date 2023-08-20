use std::sync::Arc;
use std::time::Duration;
use calloop::LoopHandle;
use wayland_client::backend::ObjectData;
use wayland_client::Proxy;
use wayland_client::protocol::wl_shm;
use wayland_client::protocol::wl_surface;
use wayland_client::protocol::wl_subsurface;
use wayland_client::protocol::wl_compositor;
use wayland_client::protocol::wl_subcompositor;

use crate::render::background::draw_background;
use crate::render::clock::draw_clock;
use crate::render::indicator::IndicatorState;
use crate::render::indicator::draw_indicator;
use crate::shm::slot::BufferSlotPool;
use crate::utils::DummyObjectData;

struct AppSubsurface {
  surface: wl_surface::WlSurface,
  subsurface: wl_subsurface::WlSubsurface,
}

impl AppSubsurface {
  fn create(
    wl_compositor: &wl_compositor::WlCompositor,
    wl_subcompositor: &wl_subcompositor::WlSubcompositor,
    parent: &wl_surface::WlSurface,
    data: Arc::<dyn ObjectData>
  ) -> Self {
    let surface = wl_compositor.send_constructor::<wl_surface::WlSurface>(wl_compositor::Request::CreateSurface {}, data.clone()).unwrap();
    let subsurface_req = wl_subcompositor::Request::GetSubsurface { surface: surface.clone(), parent: parent.clone() };
    let subsurface = wl_subcompositor.send_constructor::<wl_subsurface::WlSubsurface>(subsurface_req, data.clone()).unwrap();
    Self {
      surface,
      subsurface
    }
  }
}

pub struct AppSurface {
  pool: BufferSlotPool,
  width: u32,
  height: u32,
  bg_surface: wl_surface::WlSurface,
  clock_surface: AppSubsurface,
  clock_height: u32,
  clock_width: u32,
  indicator_surface: AppSubsurface,
  indicator_idle_timer: Option<calloop::RegistrationToken>
}

impl AppSurface {
  pub fn new(
    wl_shm: &wl_shm::WlShm,
    wl_compositor: &wl_compositor::WlCompositor,
    wl_subcompositor: &wl_subcompositor::WlSubcompositor
  ) -> Self {
    let dummy_data = Arc::new(DummyObjectData);
    let bg_surface = wl_compositor.send_constructor::<wl_surface::WlSurface>(wl_compositor::Request::CreateSurface {}, dummy_data.clone()).unwrap();
    let clock_surface = AppSubsurface::create(wl_compositor, wl_subcompositor, &bg_surface, dummy_data.clone());
    let indicator_surface = AppSubsurface::create(wl_compositor, wl_subcompositor, &bg_surface, dummy_data.clone());
    Self {
      pool: BufferSlotPool::create(4096, wl_shm),
      width: 0,
      height: 0,
      bg_surface,
      clock_surface,
      clock_width: 0,
      clock_height: 0,
      indicator_surface,
      indicator_idle_timer: None
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
      self.render_indicator(IndicatorState::Idle);
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
    let wl_surface = &self.clock_surface.surface;
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
      self.clock_surface.subsurface.set_position(x as i32, y as i32);
    }

    self.base_surface().commit();
  }

  pub fn render_indicator(&mut self, state: IndicatorState) {
    let wl_surface = &self.indicator_surface.surface;
    let buffer = draw_indicator(&mut self.pool, state);
    buffer.attach_to_surface(wl_surface);
    wl_surface.damage(0, 0, i32::MAX, i32::MAX);
    wl_surface.commit();
    let x = (self.width - buffer.width()) / 2;
    let y = (self.height - self.clock_height) / 2 + self.clock_height + 20;
    self.indicator_surface.subsurface.set_position(x as i32, y as i32);
    self.base_surface().commit();
  }

  pub fn push_state<S: 'static>(
    &mut self,
    state: IndicatorState,
    loop_handle: LoopHandle<'static, S>,
    get_surface: fn(&mut S) -> &mut Self
  ) {
    self.render_indicator(state);
    if let Some(timer) = self.indicator_idle_timer {
      loop_handle.remove(timer);
    }
    if !matches!(state, IndicatorState::Verifying) {
      self.indicator_idle_timer = Some(loop_handle.insert_source(calloop::timer::Timer::from_duration(Duration::from_secs(2)), move |_, _, s| {
        let surface = get_surface(s);
        surface.render_indicator(IndicatorState::Idle);
        calloop::timer::TimeoutAction::Drop
      }).unwrap());
    }
  }
}