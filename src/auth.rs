use std::sync::{Arc, Mutex};
use std::thread;
use users::{get_current_uid, get_user_by_uid};

pub struct Authenticator {
  username: String,
  pam_auth: Arc<Mutex<pam::Authenticator<'static, pam::PasswordConv>>>,
}

impl Authenticator {
  pub fn new() -> Self {
    let user = get_user_by_uid(get_current_uid()).unwrap();
    let username = user.name().to_owned().into_string().unwrap();
    let pam_auth = Arc::new(Mutex::new(
      pam::Authenticator::with_password("lockscreen").expect("Failed to init PAM client"),
    ));
    Self { username, pam_auth }
  }

  pub fn authenticate(&self, password: String, sender: calloop::channel::Sender<bool>) {
    let pam_auth = Arc::clone(&self.pam_auth);
    let username = self.username.clone();
    thread::spawn(move || {
      let mut pam_auth = pam_auth.lock().unwrap();
      pam_auth.get_handler().set_credentials(&username, password);
      let success = pam_auth.authenticate().is_ok();
      sender.send(success).unwrap();
    });
  }
}
