use memfd::MemfdOptions;
use memmap::{MmapMut, MmapOptions};
use std::os::fd::AsFd;
use std::{fs::File, sync::Arc};
use wayland_client::protocol::{wl_buffer, wl_shm, wl_shm_pool};
use wayland_client::{backend::ObjectData, Proxy, WEnum};

pub struct RawPool {
  len: usize,
  mem_file: File,
  mmap: MmapMut,
  pool: wl_shm_pool::WlShmPool,
}

impl RawPool {
  pub fn create(len: usize, wl_shm: &wl_shm::WlShm) -> Self {
    let mem_file = MemfdOptions::default().create("minlock_buffer").unwrap().into_file();
    mem_file.set_len(len as u64).unwrap();
    let request = wl_shm::Request::CreatePool {
      fd: mem_file.as_fd(),
      size: len as i32,
    };
    let wl_shm_pool = wl_shm.send_constructor(request, Arc::new(DummyObjectData)).unwrap();
    let mmap = unsafe { MmapOptions::new().map_mut(&mem_file).unwrap() };
    Self {
      len,
      mem_file,
      mmap,
      pool: wl_shm_pool,
    }
  }

  pub fn resize(&mut self, size: usize) {
    if size > self.len {
      self.len = size;
      self.mem_file.set_len(size as u64).unwrap();
      self.mmap = unsafe { MmapOptions::new().map_mut(&self.mem_file).unwrap() };
      self.pool.resize(size as i32);
    }
  }

  pub fn create_buffer(
    &mut self,
    offset: i32,
    width: i32,
    height: i32,
    stride: i32,
    format: wl_shm::Format,
    data: Arc<dyn ObjectData + 'static>,
  ) -> wl_buffer::WlBuffer {
    let request = wl_shm_pool::Request::CreateBuffer {
      offset,
      width,
      height,
      stride,
      format: WEnum::Value(format),
    };
    self.pool.send_constructor(request, data).unwrap()
  }

  pub fn mmap(&mut self) -> &mut MmapMut {
    &mut self.mmap
  }
  pub fn len(&self) -> usize {
    self.len
  }
}

impl Drop for RawPool {
  fn drop(&mut self) {
    self.pool.destroy();
  }
}

struct DummyObjectData;
impl ObjectData for DummyObjectData {
  fn event(
    self: Arc<Self>,
    _backend: &wayland_client::backend::Backend,
    _msg: wayland_client::backend::protocol::Message<wayland_client::backend::ObjectId, std::os::fd::OwnedFd>,
  ) -> Option<Arc<dyn ObjectData>> {
    // Do nothing
    None
  }

  fn destroyed(&self, _object_id: wayland_client::backend::ObjectId) {
    // Do nothing
  }
}
