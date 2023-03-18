use std::collections::HashMap;
use std::fmt;

#[derive(Clone)]
pub struct Opt {
   pub description: &'static str,
   pub handler: Option<fn(&HashMap<String, &Opt>)>,
   pub long: &'static str,
   pub short: &'static str,
}

impl Opt {
   pub fn name(&self) -> String {
      self.long[2..].to_string()
   }
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
