use crate::shm::slot::{BufferSlot, BufferSlotPool};

pub const INDICATOR_BLOCK_COUNT: usize = 4;

#[derive(Clone)]
pub struct RGB {
  pub r: f64,
  pub g: f64,
  pub b: f64,
}

pub fn draw_indicator(pool: &mut BufferSlotPool, block_colors: Vec<RGB>) -> &mut BufferSlot {
  let block_size = 10;
  let block_spacing = 30;
  let indicator_width = (INDICATOR_BLOCK_COUNT * block_size + (INDICATOR_BLOCK_COUNT - 1) * block_spacing) as u32;
  let indicator_height = block_size as u32;
  let (buffer, data) = pool.get_next_buffer(indicator_width, indicator_height);
  let surface = unsafe {
    cairo::ImageSurface::create_for_data_unsafe(
      data.first_mut().unwrap(),
      cairo::Format::ARgb32,
      buffer.width().try_into().unwrap(),
      buffer.height().try_into().unwrap(),
      buffer.stride().try_into().unwrap(),
    )
    .unwrap()
  };
  let context = cairo::Context::new(&surface).unwrap();
  for i in 0..INDICATOR_BLOCK_COUNT {
    let x = i * (block_size + block_spacing);
    context.rectangle(x as f64, 0.0, block_size as f64, block_size as f64);
    if let Some(color) = block_colors.get(i) {
      context.set_source_rgb(color.r, color.g, color.b);
    }
    context.fill().unwrap();
  }
  buffer
}
