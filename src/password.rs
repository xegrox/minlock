use pam::{Authenticator, PasswordConv};
use users::{get_user_by_uid, get_current_uid};

pub struct PasswordBuffer {
  username: String,
  password: String,
  authenticator: Authenticator<'static, PasswordConv>
}

impl PasswordBuffer {

  pub fn create() -> Self {
    let user = get_user_by_uid(get_current_uid()).unwrap();
    Self {
      username: user.name().to_os_string().into_string().unwrap(),
      password: String::with_capacity(12),
      authenticator: Authenticator::with_password("lockscreen").expect("Failed to init PAM client")
    }
  }

  pub fn len(&self) -> usize {
    self.password.len()
  }

  pub fn pop(&mut self) -> bool {
    self.password.pop().is_some()
  }

  pub fn push(&mut self, ch: char) {
    self.password.push(ch);
  }

  pub fn clear(&mut self) {
    self.password.clear();
  }

  pub fn authenticate(&mut self) -> bool {
    self.authenticator.get_handler().set_credentials(&self.username, &self.password);
    self.authenticator.authenticate().is_ok()
  }
}