use crate::fail;
use std::process;
use std::process::Command;
use std::str::from_utf8;

pub fn empty_trash(verbose: bool) {
  let as_cmd = "tell application \"Finder\" to empty trash";
  let res = run_applescript(as_cmd.to_string());
  if res.is_ok() && verbose {
    println!("Trash emptied");
  }
}

pub fn move_file_to_trash(files: &[String]) {
  let mut as_list = "{ POSIX file \"".to_owned();
  as_list.push_str(&files.join("\", POSIX file \""));
  as_list.push_str("\"}");
  let mut as_cmd =
    "tell application \"Finder\" to delete ".to_owned();
  as_cmd.push_str(&as_list);
  let res = run_applescript(as_cmd);
  if let Err(err) = res {
    fail!("can: Applescript error: {}", err);
  }
}

fn run_applescript(as_cmd: String) -> Result<String, String> {
  let res = Command::new("osascript").args(["-e", &as_cmd]).output();
  match res {
    Ok(output) => {
      if output.stderr.len() > 0 {
        let err = from_utf8(&output.stderr)
          .unwrap_or_else(|_| "Unknown UTF-8 error".into())
          .to_owned();
        return Err(err);
      }
      return Ok(
        from_utf8(&output.stdout)
          .unwrap_or_else(|_| "".into())
          .to_owned(),
      );
    }
    Err(err) => return Err(err.to_string()),
  }
}
