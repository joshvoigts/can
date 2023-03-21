use std::collections::BTreeSet;
use std::env;
use std::fs::{self, DirEntry};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::str::from_utf8;

use crate::opt::*;

mod fail;
mod opt;
mod p;

fn main() {
   let mut opts = Opts::new();
   opts.insert(
      "help",
      Opt {
         description: "Show help",
         enabled: false,
         handler: Some(help),
         long: "--help",
         short: "-h",
      },
   );
   opts.insert(
      "empty",
      Opt {
         description: "Empty trash",
         enabled: false,
         handler: Some(empty),
         long: "--empty",
         short: "-e",
      },
   );
   opts.insert(
      "list",
      Opt {
         description: "List files in trash",
         enabled: false,
         handler: Some(list),
         long: "--list",
         short: "-l",
      },
   );
   opts.insert(
      "verbose",
      Opt {
         description: "Run verbosely",
         enabled: false,
         handler: None,
         long: "--verbose",
         short: "-v",
      },
   );

   let mut args: BTreeSet<String> = env::args().skip(1).collect();

   for mut opt in opts.values_mut() {
      if args.remove(opt.short) {
         opt.enabled = true;
      }
      if args.remove(opt.long) {
         opt.enabled = true;
      }
   }

   for opt in opts.values() {
      if !opt.enabled {
         continue;
      }
      if let Some(handler) = opt.handler {
         handler(&opts);
         process::exit(0);
      }
   }

   if args.len() > 0 {
      move_files_to_trash(&opts, &args);
   }

   help(&opts);
}

fn help(opts: &Opts) {
   println!("Usage: can [options] file ...");
   for opt in opts.values() {
      println!(
         "  {}, {:<10} {}",
         opt.short, opt.long, opt.description
      )
   }
}

fn empty(opts: &Opts) {
   match env::consts::OS {
      "macos" => {
         let as_cmd = "tell application \"Finder\" to empty trash";
         let res = run_applescript(as_cmd.to_string());
         match res {
            Ok(_) => {
               if let Some(opt) = opts.get("verbose") {
                  if opt.enabled {
                     println!("Trash emptied");
                  }
               }
            }
            Err(_) => (), // Ignore, trash probably already empty
         }
      }
      _ => fail!("can: OS not supported"),
   }
}

fn list(_opts: &Opts) {
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

fn move_files_to_trash(opts: &Opts, args: &BTreeSet<String>) {
   let mut to_delete: Vec<String> = Vec::new();
   for arg in args {
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
   if let Some(opt) = opts.get("verbose") {
      if opt.enabled {
         for arg in args {
            println!("{}", arg);
         }
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
