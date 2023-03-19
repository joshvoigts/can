use std::collections::{BTreeSet, HashMap};
use std::env;
use std::fs;
// use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::str::from_utf8;

use crate::opt::Opt;

mod fail;
mod opt;
mod p;

fn main() {
   let flags = [
      Opt {
         description: "Show help",
         handler: Some(help),
         long: "--help",
         short: "-h",
      },
      Opt {
         description: "Empty trash",
         handler: Some(empty),
         long: "--empty",
         short: "-e",
      },
      Opt {
         description: "List files in trash",
         handler: None, // TODO
         long: "--list",
         short: "-l",
      },
      Opt {
         description: "Run verbosely",
         handler: None,
         long: "--verbose",
         short: "-v",
      },
   ];

   let mut args: BTreeSet<String> = env::args().skip(1).collect();
   let mut opts = HashMap::new();

   for flag in &flags {
      let name = flag.name();
      if args.remove(flag.short) {
         opts.insert(name.clone(), flag);
      }
      if args.remove(flag.long) {
         opts.insert(name.clone(), flag);
      }
      if opts.get(&name).is_some() {
         if let Some(handler) = flag.handler {
            handler(&opts);
         }
      }
   }

   if args.len() > 0 {
      move_files_to_trash(&opts, &args);
   }

   help(&opts);
}

fn move_files_to_trash(
   opts: &HashMap<String, &Opt>,
   args: &BTreeSet<String>,
) {
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
         Err(_) => {
            fail!("can: {}: Could not canonicalize", arg);
         }
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
         run_applescript(as_cmd);
      }
      _ => {}
   }

   if opts.get("verbose").is_some() {
      for arg in args {
         println!("{}", arg);
      }
   }
   process::exit(0);
}

fn help(opts: &HashMap<String, &Opt>) {
   println!("Usage: can [options] file ...");
   for opt in opts.values() {
      println!(
         "  {}, {:<10} {}",
         opt.short, opt.long, opt.description
      )
   }
   process::exit(0);
}

fn empty(opts: &HashMap<String, &Opt>) {
   match env::consts::OS {
      "macos" => {
         let as_cmd = "tell application \"Finder\" to empty trash";
         run_applescript(as_cmd.to_string());
         if opts.get("verbose").is_some() {
            print!("Trash emptied");
         }
      }
      _ => (),
   }
   process::exit(0);
}

fn run_applescript(as_cmd: String) {
   let res = Command::new("osascript").args(["-e", &as_cmd]).output();
   match res {
      Ok(output) => {
         if output.stderr.len() > 0 {
            let err = from_utf8(&output.stderr).unwrap();
            fail!("applescript error: {}", err);
         }
      }
      Err(_) => fail!("could not execute command"),
   }
}

// fn list(opts: &[Opt]) {
//    print!("Not implemented");
//    //    let files = get_files(&get_trash_path());
// }
//
// fn get_files(dir_path: &Path) -> Vec<DirEntry> {
//    let mut files: Vec<DirEntry> = Vec::new();
//    if dir_path.is_dir() {
//       p!(dir_path);
//       p!(fs::read_dir(dir_path));
//       for maybe_entry in fs::read_dir(dir_path) {
//          //          let entry = maybe_entry.unwrap();
//          p!(maybe_entry);
//          //          let maybe_name = entry.file_name().into_string();
//          //          if let Ok(file_name) = maybe_name {
//          //             if !file_name.starts_with(".") {
//          //                files.push(entry);
//          //             }
//          //          }
//       }
//       //       files.sort_by_key(|entry| {
//       //          entry.metadata().unwrap().modified().unwrap()
//       //       });
//    }
//    return files;
// }

// fn get_trash_path() -> PathBuf {
//    match env::var("HOME") {
//       Ok(home) => {
//          let mut path_buf = PathBuf::new();
//          path_buf.push(&home);
//          path_buf.push(".local");
//          path_buf.push("share");
//          path_buf.push("Trash");
//          path_buf
//       }
//       Err(_) => {
//          panic!("Failed to find trash folder")
//       }
//    }
// }
