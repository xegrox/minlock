pub enum PasswordBufferEvent {
  Input(u32),
  Invalid,
  Clear,
  None
}

pub struct PasswordBuffer {
  password: String
}

impl PasswordBuffer {

  pub fn create() -> Self { Self { password: String::with_capacity(12) } }

  pub fn pop(&mut self) -> PasswordBufferEvent {
    self.password.pop().map_or(
      PasswordBufferEvent::None,
      |_| {
        let len = self.password.len();
        if len > 0 {
          PasswordBufferEvent::Input(self.password.len() as u32)
        } else {
          PasswordBufferEvent::Clear
        }
      }
    )
  }

  pub fn push(&mut self, ch: char) -> PasswordBufferEvent {
    self.password.push(ch);
    PasswordBufferEvent::Input(self.password.len() as u32)
  }

  pub fn clear(&mut self) -> PasswordBufferEvent {
    self.password.clear();
    PasswordBufferEvent::Clear
  }
}