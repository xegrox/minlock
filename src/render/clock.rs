use cairo::{Antialias, FontOptions, HintStyle};
use chrono::Local;
use polonius_the_crab::{polonius, polonius_return};

use crate::{
  args::Color,
  shm::slot::{BufferSlot, BufferSlotPool},
};

pub fn draw_clock(
  mut pool: &mut BufferSlotPool,
  width: u32,
  height: u32,
  text_color: Color,
  font: String,
  font_size: f64,
  bg_color: Color,
) -> &mut BufferSlot {
  let (min_width, expected_height) = polonius!(|pool| -> &'polonius mut BufferSlot {
    let (buffer, data) = pool.get_next_buffer(width, height);
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
    let text = Local::now().format("%H:%M:%S").to_string();

    // Calculate expected surface height/width
    let context = cairo::Context::new(&surface).unwrap();
    context.set_font_size(font_size);
    context.select_font_face(&font, cairo::FontSlant::Normal, cairo::FontWeight::Normal);
    let text_extents = context.text_extents(text.as_str()).unwrap();
    let font_extents = context.font_extents().unwrap();
    let text_width = text_extents.x_advance.ceil() as u32;
    let text_height = font_extents.height.ceil() as u32;

    // Text height is always constant while width always changes
    // Accept if buffer width is longer than actual text width
    if buffer.width() >= text_width && buffer.height() == text_height {
      context.set_source_rgb(bg_color.r, bg_color.g, bg_color.b);
      context.paint().unwrap();
      context.set_source_rgb(text_color.r, text_color.g, text_color.b);
      let mut font_options = FontOptions::new().unwrap();
      font_options.set_hint_style(HintStyle::Full);
      font_options.set_antialias(Antialias::Subpixel);
      context.set_font_options(&font_options);
      let x = (buffer.width() - text_width) / 2;
      context.move_to(x as f64, font_extents.ascent);
      context.show_text(text.as_str()).unwrap();
      polonius_return!(buffer);
    }
    (std::cmp::max(text_width, buffer.width()), text_height)
  });
  draw_clock(pool, min_width, expected_height, text_color, font, font_size, bg_color)
}
