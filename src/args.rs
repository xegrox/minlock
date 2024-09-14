use clap::Parser;
use hex_color::{HexColor, ParseHexColorError};

#[derive(Parser)]
#[command(version, about, long_about=None, after_help = "All <COLOR> options are in RRGGBB format")]
pub struct Args {
  #[arg(short, long, value_name="COLOR", value_parser=parse_color, default_value="04030B", hide_default_value=true)]
  pub bg_color: Color,

  #[arg(short, long, value_name="COLOR", value_parser=parse_color, default_value="FAFAFA", hide_default_value=true)]
  pub clock_color: Color,

  #[arg(long, value_name="COLOR", value_parser=parse_color, default_value="333333", hide_default_value=true)]
  pub indicator_idle_color: Color,

  #[arg(long, value_name="COLOR", value_parser=parse_color, default_value="B24C4C", hide_default_value=true)]
  pub indicator_wrong_color: Color,

  #[arg(long, value_name="COLOR", value_parser=parse_color, default_value="338080", hide_default_value=true)]
  pub indicator_clear_color: Color,

  #[arg(long, value_name="COLOR", value_parser=parse_color, default_value="998033", hide_default_value=true)]
  pub indicator_verifying_color: Color,

  #[arg(long, value_name="COLOR", value_parser=parse_color, default_value="606060", hide_default_value=true)]
  pub indicator_input_cursor_color: Color,

  #[arg(long, value_name="COLOR", value_parser=parse_color, default_value="191919", hide_default_value=true)]
  pub indicator_input_cursor_increment_color: Color,

  #[arg(long, value_name="COLOR", value_parser=parse_color, default_value="333333", hide_default_value=true)]
  pub indicator_input_trail_color: Color,

  #[arg(long, value_name="COLOR", value_parser=parse_color, default_value="191919", hide_default_value=true)]
  pub indicator_input_trail_increment_color: Color,
}

#[derive(Clone, Copy)]
pub struct Color {
  pub r: f64,
  pub g: f64,
  pub b: f64,
}

fn parse_color(str: &str) -> Result<Color, ParseHexColorError> {
  let hex_color = HexColor::parse(&(String::from("#") + str))?;
  Ok(Color {
    r: f64::from(hex_color.r) / 255f64,
    b: f64::from(hex_color.b) / 255f64,
    g: f64::from(hex_color.g) / 255f64,
  })
}
