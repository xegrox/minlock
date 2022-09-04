use crate::shm::slot::{BufferSlotPool, BufferSlot};

pub fn draw_background(pool: &mut BufferSlotPool, width: u32, height: u32) -> &mut BufferSlot {
  let (buffer, data) = pool.get_next_buffer(width, height);
  let surface = unsafe {
    cairo::ImageSurface::create_for_data_unsafe(
      data.first_mut().unwrap(),
      cairo::Format::ARgb32,
      buffer.width().try_into().unwrap(),
      buffer.height().try_into().unwrap(),
      buffer.stride().try_into().unwrap()
    ).unwrap()
  };
  let context = cairo::Context::new(&surface).unwrap();
  context.set_source_rgb(0.0, 0.0, 0.0);
  context.paint().unwrap();
  buffer
}