use crate::{
  args::Color,
  shm::slot::{BufferSlot, BufferSlotPool},
};

pub const INDICATOR_BLOCK_COUNT: usize = 4;

pub fn draw_indicator(
  pool: &mut BufferSlotPool,
  block_colors: [Color; INDICATOR_BLOCK_COUNT],
  bg_color: Color,
) -> &mut BufferSlot {
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
  context.set_source_rgb(bg_color.r, bg_color.g, bg_color.b);
  context.paint().unwrap();
  for i in 0..INDICATOR_BLOCK_COUNT {
    let x = i * (block_size + block_spacing);
    context.rectangle(x as f64, 0.0, block_size as f64, block_size as f64);
    let color = block_colors[i];
    context.set_source_rgb(color.r, color.g, color.b);
    context.fill().unwrap();
  }
  buffer
}
