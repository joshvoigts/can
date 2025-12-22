use optz::{Opt, Optz};
use shared::*;
use std::env;
use std::process;

mod fail;
mod linux;
mod macos;
mod shared;

fn main() {
  let optz = Optz::from_args("can", env::args().collect())
    .option(
      Opt::flag("verbose")
        .short("-v")
        .description("Run verbosely"),
    )
    .option(
      Opt::flag("list")
        .short("-l")
        .description("List trash contents"),
    )
    .option(Opt::flag("empty").short("-E").description("Empty trash"))
    .parse()
    .unwrap();

  let verbose = match optz.get::<bool>("verbose") {
    Ok(Some(v)) => v,
    Ok(None) => false,
    Err(_) => false,
  };

  if optz.has("list").unwrap_or(false) {
    list(&optz, verbose);
    process::exit(0);
  }

  if optz.has("empty").unwrap_or(false) {
    empty(&optz, verbose);
    process::exit(0);
  }

  // Get file arguments (non-option arguments)
  if !optz.rest.is_empty() {
    move_files_to_trash(&optz, verbose);
  } else {
    help(&optz);
  }
}

fn help(optz: &Optz) {
  println!("Usage: can [options] file ...");
  for opt in &optz.options {
    let short_str = opt.short.as_deref().unwrap_or("");
    println!(
      "  {}, {:<10} {}",
      short_str,
      opt.long,
      opt.description.as_deref().unwrap_or("")
    )
  }
}
