use crate::fail;
use crate::linux;
use crate::macos;
use optz::Optz;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use urlencoding::decode;
use xdg::BaseDirectories;

#[derive(Debug, Clone)]
struct TrashEntry {
  name: String,
  #[allow(dead_code)]
  path: PathBuf,
  #[allow(dead_code)]
  info_path: PathBuf,
  original_path: String,
  deletion_date: String,
}

impl TrashEntry {
  fn from_paths(
    files_dir: &Path,
    info_path: &Path,
    home_trash: &Path,
  ) -> Option<Self> {
    let info_name =
      info_path.file_name()?.to_string_lossy().to_string();
    if !info_name.ends_with(".trashinfo") {
      return None;
    }

    let name = info_name.trim_end_matches(".trashinfo").to_string();
    let files_path = files_dir.join(&name);

    if !files_path.exists() {
      return None;
    }

    // Read the trashinfo file to get original path and deletion date
    let content = fs::read_to_string(info_path).ok()?;
    let mut original_path = String::new();
    let mut deletion_date = String::new();

    for line in content.lines() {
      if line.starts_with("Path=") {
        let path_value = line.trim_start_matches("Path=").to_string();
        // URL decode the path as per spec requirements
        original_path = decode(&path_value)
          .expect("Failed to decode path")
          .to_string();
      } else if line.starts_with("DeletionDate=") {
        deletion_date =
          line.trim_start_matches("DeletionDate=").to_string();
      }
    }

    // Convert relative paths to absolute
    let full_original_path = if original_path.starts_with('/') {
      PathBuf::from(original_path)
    } else {
      // For relative paths:
      // - In home trash: relative to parent of trash directory
      // - In topdir trash: should not have relative paths per spec
      // But if they exist, treat them as relative to parent of trash directory
      let trash_parent =
        home_trash.parent().unwrap_or(Path::new("/"));
      trash_parent.join(&original_path)
    };

    Some(TrashEntry {
      name,
      path: files_path,
      info_path: info_path.to_path_buf(),
      original_path: full_original_path.display().to_string(),
      deletion_date,
    })
  }
}

pub fn empty(_optz: &Optz, verbose: bool) {
  match env::consts::OS {
    "macos" => macos::empty_trash(verbose),
    "linux" => linux::empty_trash(verbose),
    _ => fail!("can: OS not supported"),
  }
}

pub fn list(_optz: &Optz, verbose: bool) {
  let mut entries = get_all_trash_entries();

  // Sort entries by name for consistent output
  entries.sort_by(|a, b| a.name.cmp(&b.name));

  if entries.is_empty() {
    println!("Trash is empty");
    return;
  }

  for entry in entries {
    if verbose {
      println!(
        "{} (deleted: {}, original: {})",
        entry.name, entry.deletion_date, entry.original_path
      );
    } else {
      println!("{}", entry.name);
    }
  }
}

pub fn get_all_trash_paths() -> Vec<PathBuf> {
  let mut trash_paths = Vec::new();

  // Always include home trash
  let home_trash = get_home_trash_path();
  trash_paths.push(home_trash);

  if env::consts::OS == "linux" {
    // Add per-device trash directories
    trash_paths.extend(linux::get_topdir_trash_paths());
  }

  trash_paths
}

fn get_all_trash_entries() -> Vec<TrashEntry> {
  let mut entries = Vec::new();
  let trash_paths = get_all_trash_paths();

  for trash_path in trash_paths {
    let files_dir = trash_path.join("files");
    let info_dir = trash_path.join("info");

    if !files_dir.exists() || !info_dir.exists() {
      continue;
    }

    if let Ok(info_entries) = fs::read_dir(&info_dir) {
      for info_entry in info_entries {
        if let Ok(info_entry) = info_entry {
          if let Some(entry) = TrashEntry::from_paths(
            &files_dir,
            &info_entry.path(),
            &trash_path,
          ) {
            entries.push(entry);
          }
        }
      }
    }
  }

  entries
}

pub fn get_home_trash_path() -> PathBuf {
  match env::consts::OS {
    "macos" => {
      let home = env::var("HOME").unwrap_or_else(|_| {
        fail!("can: HOME not set");
      });
      PathBuf::from(home).join(".Trash")
    }
    "linux" => {
      let xdg = BaseDirectories::new();
      let home = xdg
        .get_data_home()
        .unwrap_or_else(|| fail!("can: Can't find HOME directory"));
      PathBuf::from(home).join("Trash")
    }
    _ => fail!("can: OS not supported"),
  }
}

pub fn move_files_to_trash(optz: &Optz, verbose: bool) {
  let mut to_delete: Vec<String> = Vec::new();

  // Validate all paths before processing
  for arg in &optz.rest {
    let path = Path::new(&arg);

    if !path.exists() {
      fail!("can: {}: No such file or directory", arg);
    }

    // Canonicalize to resolve symlinks and relative components
    match fs::canonicalize(path) {
      Ok(abs_path) => to_delete.push(abs_path.display().to_string()),
      Err(e) => fail!("can: {}: Canonicalize failed: {}", arg, e),
    }
  }

  // Early exit if no valid files to delete
  if to_delete.is_empty() {
    if verbose {
      println!("No valid files to delete");
    }
    process::exit(0);
  }

  match env::consts::OS {
    "macos" => {
      macos::move_file_to_trash(&to_delete);
    }
    "linux" => {
      linux::move_file_to_trash(&to_delete);
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
