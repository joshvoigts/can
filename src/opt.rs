use std::collections::HashMap;
use std::fmt;

pub type Opts = HashMap<&'static str, Opt>;

#[derive(Clone)]
pub struct Opt {
  pub description: &'static str,
  pub enabled: bool,
  pub handler: Option<fn(&Opts)>,
  pub long: &'static str,
  pub short: &'static str,
}

impl fmt::Debug for Opt {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Opt")
      .field("description", &self.description)
      .field("handler", &"handler")
      .field("long", &self.long)
      .field("short", &self.short)
      .finish()
  }
}
