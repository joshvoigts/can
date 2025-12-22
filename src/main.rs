use std::env;
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::str::from_utf8;

mod fail;
mod p;

use optz::{Opt, Optz};

fn main() {
  let optz = Optz::from_args("can", env::args().collect())
    .option(
      Opt::flag("verbose")
        .short("-v")
        .description("Run verbosely")
    )
    .option(
      Opt::flag("list")
        .short("-l")
        .description("List trash contents")
    )
    .option(
      Opt::flag("empty")
        .short("-E")
        .description("Empty trash")
    )
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
    println!("  {}, {:<10} {}", short_str, opt.long, opt.description.as_deref().unwrap_or(""))
  }
}

fn empty(_optz: &Optz, verbose: bool) {
  match env::consts::OS {
    "macos" => {
      let as_cmd = "tell application \"Finder\" to empty trash";
      let res = run_applescript(as_cmd.to_string());
      match res {
        Ok(_) => {
          if verbose {
            println!("Trash emptied");
          }
        }
        Err(_) => (), // Ignore, trash probably already empty
      }
    }
    _ => fail!("can: OS not supported"),
  }
}

fn list(_optz: &Optz, _verbose: bool) {
  let files = get_files(&get_trash_path());
  for file in files {
    match file.file_name().into_string() {
      Ok(name) => println!("{}", name),
      Err(_) => fail!("can: Could not list non-unicode filename"),
    }
  }
}

fn get_files(dir_path: &Path) -> Vec<DirEntry> {
  if !dir_path.exists() || !dir_path.is_dir() {
    fail!("can: Could not find trash folder");
  }
  let mut files: Vec<DirEntry> = Vec::new();
  match fs::read_dir(dir_path) {
    Ok(entries) => {
      for maybe_entry in entries {
        match maybe_entry {
          Ok(entry) => {
            let maybe_name = entry.file_name().into_string();
            if let Ok(file_name) = maybe_name {
              if !file_name.starts_with(".") {
                files.push(entry);
              }
            }
          }
          Err(err) => {
            fail!("can: Could not access trash file: {}", err)
          }
        }
      }
    }
    Err(err) => {
      fail!("can: Could not access trash directory: {}", err)
    }
  }
  files.sort_by_key(|entry| {
    entry.metadata().unwrap().modified().unwrap()
  });
  return files;
}

fn get_trash_path() -> PathBuf {
  match env::var("HOME") {
    Ok(home) => match env::consts::OS {
      "macos" => {
        let mut path_buf = PathBuf::new();
        path_buf.push(&home);
        path_buf.push(".Trash");
        path_buf
      }
      _ => {
        let mut path_buf = PathBuf::new();
        path_buf.push(&home);
        path_buf.push(".local");
        path_buf.push("share");
        path_buf.push("Trash");
        path_buf
      }
    },
    Err(_) => panic!("can: Could not find trash folder"),
  }
}

fn move_files_to_trash(optz: &Optz, verbose: bool) {
  let mut to_delete: Vec<String> = Vec::new();
  for arg in &optz.rest {
    let path = Path::new(&arg);
    if !path.exists() {
      fail!("can: {}: No such file or directory", arg);
    }
    match fs::canonicalize(path) {
      Ok(abs_path) => {
        let abs_str = abs_path.display().to_string();
        if abs_str.contains("\"") {
          fail!("can: {}: Could not escape path", arg);
        }
        to_delete.push(abs_str);
      }
      Err(_) => fail!("can: {}: Could not canonicalize", arg),
    }
  }
  match env::consts::OS {
    "macos" => {
      let mut as_list = "{ POSIX file \"".to_owned();
      as_list.push_str(&to_delete.join("\", POSIX file \""));
      as_list.push_str("\"}");
      let mut as_cmd =
        "tell application \"Finder\" to delete ".to_owned();
      as_cmd.push_str(&as_list);
      let res = run_applescript(as_cmd);
      if let Err(err) = res {
        fail!("can: Applescript error: {}", err);
      }
    }
    _ => fail!("can: OS not supported"),
  }
  if verbose {
    for arg in &optz.rest {
      println!("{}", arg);
    }
  }
  process::exit(0);
}

fn run_applescript(as_cmd: String) -> Result<String, String> {
  let res = Command::new("osascript").args(["-e", &as_cmd]).output();
  match res {
    Ok(output) => {
      if output.stderr.len() > 0 {
        let err = from_utf8(&output.stderr).unwrap().to_owned();
        return Err(err);
      }
      return Ok(from_utf8(&output.stdout).unwrap().to_owned());
    }
    Err(err) => return Err(err.to_string()),
  }
}
