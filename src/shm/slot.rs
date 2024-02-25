use std::sync::{
  atomic::{AtomicBool, Ordering},
  Arc,
};

use super::raw::RawPool;
use wayland_client::backend::ObjectData;
use wayland_client::protocol::{wl_buffer, wl_shm, wl_surface};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct Dimensions {
  width: u32,
  height: u32,
}

impl Dimensions {
  fn stride(&self) -> u32 {
    self.width * 4
  }
  fn len(&self) -> usize {
    (self.stride() * self.height) as usize
  }
}

#[derive(Debug, Clone)]
pub struct BufferSlot {
  wl_buffer: Option<wl_buffer::WlBuffer>,
  offset: usize,
  len: usize,
  dimensions: Dimensions,
  busy: Arc<AtomicBool>,
}

impl BufferSlot {
  fn new(offset: usize, dimensions: Dimensions) -> Self {
    Self {
      wl_buffer: None,
      offset,
      len: dimensions.len(),
      dimensions,
      busy: Arc::new(AtomicBool::new(false)),
    }
  }

  pub fn height(&self) -> u32 {
    self.dimensions.height
  }
  pub fn width(&self) -> u32 {
    self.dimensions.width
  }
  pub fn stride(&self) -> u32 {
    self.dimensions.stride()
  }

  fn init_buffer(&mut self, pool: &mut RawPool) {
    let data = BufferSlotData {
      busy: self.busy.clone(),
    };
    self.wl_buffer.get_or_insert_with(|| {
      pool.create_buffer(
        self.offset.try_into().unwrap(),
        self.dimensions.width.try_into().unwrap(),
        self.dimensions.height.try_into().unwrap(),
        self.dimensions.stride().try_into().unwrap(),
        wl_shm::Format::Xrgb8888,
        Arc::new(data),
      )
    });
  }

  fn get_data<'a>(&self, pool: &'a mut RawPool) -> &'a mut [u8] {
    &mut pool.mmap()[self.offset..][..self.len]
  }

  pub fn attach_to_surface(&self, wl_surface: &wl_surface::WlSurface) {
    let buffer = self.wl_buffer.as_ref().unwrap();
    wl_surface.attach(Some(&buffer), 0, 0);
    self.busy.store(true, Ordering::Relaxed);
  }

  fn transform(&mut self, dimensions: Dimensions) {
    if !self.busy.load(Ordering::Relaxed) && dimensions.len() == self.len {
      if let Some(b) = &self.wl_buffer {
        b.destroy()
      };
      self.wl_buffer = None;
      self.dimensions = dimensions;
    }
  }
}

impl Drop for BufferSlot {
  fn drop(&mut self) {
    if let Some(b) = &self.wl_buffer {
      b.destroy()
    }
  }
}

pub struct BufferSlotPool {
  len: usize,
  inner: RawPool,
  buffers: Vec<BufferSlot>,
}

impl BufferSlotPool {
  pub fn create(len: usize, wl_shm: &wl_shm::WlShm) -> Self {
    let pool = RawPool::create(len, wl_shm);
    Self {
      len: 0,
      inner: pool,
      buffers: Vec::new(),
    }
  }

  fn push(&mut self, dimensions: Dimensions) -> usize {
    let slot = BufferSlot::new(self.len, dimensions);
    if self.len + slot.len > self.inner.len() {
      let new_len = std::cmp::max(self.len * 2, self.len + slot.len);
      self.inner.resize(new_len);
    }
    self.len += slot.len;
    self.buffers.push(slot);
    self.buffers.len() - 1
  }

  pub fn get_next_buffer(&mut self, width: u32, height: u32) -> (&mut BufferSlot, &mut [u8]) {
    let dimensions = Dimensions { width, height };
    // Search for existing buffer that can be used
    let mut buffer_index: Option<usize> = None;
    for i in 0..self.buffers.len() {
      let b = &mut self.buffers[i];
      if !b.busy.load(Ordering::Relaxed) {
        if b.dimensions == dimensions {
          // Buffer with same dimensions found, reuse it
          buffer_index = Some(i);
          break;
        } else if b.len == dimensions.len() {
          // Buffer with same length found, transform dimensions to fit
          b.transform(dimensions);
          buffer_index = Some(i);
          break;
        }
      };
    }

    let buffer_index = buffer_index.unwrap_or_else(|| self.push(dimensions));
    let buffer = &mut self.buffers[buffer_index];
    buffer.init_buffer(&mut self.inner);
    let data = buffer.get_data(&mut self.inner);
    data.fill(0);
    (buffer, data)
  }
}

struct BufferSlotData {
  busy: Arc<AtomicBool>,
}

impl ObjectData for BufferSlotData {
  fn event(
    self: std::sync::Arc<Self>,
    _backend: &wayland_client::backend::Backend,
    msg: wayland_client::backend::protocol::Message<
      wayland_client::backend::ObjectId,
      std::os::fd::OwnedFd,
    >,
  ) -> Option<std::sync::Arc<dyn ObjectData>> {
    if wl_buffer::EVT_RELEASE_OPCODE == msg.opcode.into() {
      self.busy.store(false, Ordering::Relaxed);
    }
    None
  }

  fn destroyed(&self, _object_id: wayland_client::backend::ObjectId) {}
}
