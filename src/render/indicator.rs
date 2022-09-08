use crate::shm::slot::{BufferSlotPool, BufferSlot};

pub enum IndicatorState {
  Input(u32),
  Invalid,
  Clear,
  Idle
}

pub fn draw_indicator(pool: &mut BufferSlotPool, state: IndicatorState) -> &mut BufferSlot {
  let block_count = 4;
  let block_size = 10;
  let block_spacing = 30;
  let indicator_width = block_count * block_size + (block_count - 1) * block_spacing;
  let indicator_height = block_size;
  let (buffer, data) = pool.get_next_buffer(indicator_width, indicator_height);
  let surface = unsafe {
    cairo::ImageSurface::create_for_data_unsafe(
      data.first_mut().unwrap(),
      cairo::Format::ARgb32,
      buffer.width().try_into().unwrap(),
      buffer.height().try_into().unwrap(),
      buffer.stride().try_into().unwrap()
    ).unwrap()
  };
  let position = if let IndicatorState::Input(length) = state {
    Some((length - 1) % block_count)
  } else {
    None
  };
  let context = cairo::Context::new(&surface).unwrap();
  if let IndicatorState::Clear = state {
    context.set_source_rgb(0.0, 0.3, 0.5);
  } else {
    context.set_source_rgb(0.2, 0.2, 0.2);
  }
  for i in 0..block_count {
    let x = i * (block_size + block_spacing);
    context.rectangle(x as f64, 0.0, block_size as f64, block_size as f64);
    if let Some(position) = position {
      if i == position {
        context.save().unwrap();
        context.set_source_rgb(0.4, 0.4, 0.4);
        context.fill().unwrap();
        context.restore().unwrap();
        continue;
      }
    }
    context.fill().unwrap();
  }
  buffer
}