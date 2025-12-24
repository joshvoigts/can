use crate::fail;
use crate::shared::{get_all_trash_paths, get_home_trash_path};
use chrono;
use filetime;
use libc;
use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::SystemTime;
use urlencoding::{decode, encode};

pub fn empty_trash(verbose: bool) {
  let mut trash_paths = get_all_trash_paths();
  // Deduplicate paths (home trash may also be listed as a
  // per-device trash).
  trash_paths.sort_by_key(|p| {
    p.as_os_str()
      .to_os_string()
      .into_string()
      .unwrap_or_default()
  });
  trash_paths.dedup_by_key(|p| {
    p.as_os_str()
      .to_os_string()
      .into_string()
      .unwrap_or_default()
  });

  let mut had_errors = false;

  for trash_path in trash_paths {
    let files_dir = trash_path.join("files");
    let info_dir = trash_path.join("info");

    if files_dir.exists() {
      // Recursively remove all files and directories
      if let Ok(entries) = fs::read_dir(&files_dir) {
        for entry in entries {
          if let Ok(entry) = entry {
            let path = entry.path();
            let result = if path.is_dir() {
              fs::remove_dir_all(&path)
            } else {
              fs::remove_file(&path)
            };

            match result {
              Ok(_) => {}
              Err(e) => {
                if verbose {
                  eprintln!(
                    "Warning: Failed to remove {}: {}",
                    path.display(),
                    e
                  );
                }
                had_errors = true;
              }
            }
          }
        }
      }
    }

    if info_dir.exists() {
      // Remove all trashinfo files
      if let Ok(entries) = fs::read_dir(&info_dir) {
        for entry in entries {
          if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_file() {
              match fs::remove_file(&path) {
                Ok(_) => {}
                Err(e) => {
                  if verbose {
                    eprintln!(
                      "Warning: Failed to remove {}: {}",
                      path.display(),
                      e
                    );
                  }
                  had_errors = true;
                }
              }
            }
          }
        }
      }
    }

    // Remove directorysizes cache if it exists
    let sizes_path = trash_path.join("directorysizes");
    if sizes_path.exists() {
      match fs::remove_file(&sizes_path) {
        Ok(_) => {}
        Err(e) => {
          if verbose {
            eprintln!(
              "Warning: Failed to remove directorysizes cache: {}",
              e
            );
          }
          had_errors = true;
        }
      }
    }
  }

  if had_errors {
    eprintln!("Warning: Some items could not be removed from trash");
  }

  if verbose || !had_errors {
    println!("Trash emptied");
  }
}

pub fn get_topdir_trash_paths() -> Vec<PathBuf> {
  let mut trash_paths = Vec::new();
  let uid = get_current_uid();
  // Only applicable on Linux
  if env::consts::OS != "linux" {
    return trash_paths;
  }

  // Get all mount points
  if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
    for line in mounts.lines() {
      let parts: Vec<&str> = line.split_whitespace().collect();
      if parts.len() >= 2 {
        let mount_point = PathBuf::from(parts[1]);
        let fs_type = parts[2];

        // Skip pseudo filesystems
        if fs_type == "proc"
          || fs_type == "sysfs"
          || fs_type == "tmpfs"
          || fs_type == "devpts"
          || fs_type == "devtmpfs"
          || fs_type == "cgroup"
          || fs_type == "cgroup2"
          || fs_type == "securityfs"
          || fs_type == "pstore"
          || fs_type == "bpf"
          || fs_type == "tracefs"
          || fs_type == "debugfs"
          || fs_type == "hugetlbfs"
          || fs_type == "mqueue"
          || fs_type == "autofs"
          || fs_type == "configfs"
          || fs_type == "fusectl"
          || fs_type == "selinuxfs"
          || fs_type == "rpc_pipefs"
          || fs_type == "binfmt_misc"
        {
          continue;
        }

        // Try method (1): $topdir/.Trash/$uid
        let trash_method1 =
          mount_point.join(".Trash").join(&uid.to_string());
        if trash_method1.exists()
          && is_valid_trash_dir(&trash_method1)
        {
          trash_paths.push(trash_method1);
          continue;
        }

        // Try method (2): $topdir/.Trash-$uid
        let trash_method2 =
          mount_point.join(&format!(".Trash-{}", uid));
        if trash_method2.exists() {
          trash_paths.push(trash_method2);
        }
      }
    }
  }

  trash_paths
}

fn get_trash_path_for_file(file_path: &Path) -> Option<PathBuf> {
  let home_trash = get_home_trash_path();
  let uid = get_current_uid();

  let canonical_path = file_path.canonicalize().ok()?;
  let topdir = canonical_path.parent()?;

  let trash_method1 = topdir.join(".Trash").join(&uid.to_string());
  if trash_method1.exists() && is_valid_trash_dir(&trash_method1) {
    return Some(trash_method1);
  }

  let trash_method2 = topdir.join(&format!(".Trash-{}", uid));
  if trash_method2.exists() {
    return Some(trash_method2);
  }

  Some(home_trash)
}

fn is_valid_trash_dir(trash_dir: &Path) -> bool {
  let files_dir = trash_dir.join("files");
  let info_dir = trash_dir.join("info");
  files_dir.is_dir() && info_dir.is_dir()
}

fn get_current_uid() -> u32 {
  // Get current user ID
  unsafe { libc::getuid() }
}

fn copy_file_to_trash(source: &Path, dest: &Path) -> io::Result<()> {
  if source.is_dir() {
    fs::create_dir_all(dest)?;

    // Preserve directory metadata
    if let Ok(metadata) = fs::metadata(source) {
      let _ = fs::set_permissions(dest, metadata.permissions());
      if let Ok(times) = metadata.modified() {
        let _ = filetime::set_file_mtime(
          dest,
          filetime::FileTime::from_system_time(times),
        );
      }
      if let Ok(times) = metadata.accessed() {
        let _ = filetime::set_file_atime(
          dest,
          filetime::FileTime::from_system_time(times),
        );
      }
    }

    for entry in fs::read_dir(source)? {
      let entry = entry?;
      let src = entry.path();
      let dst = dest.join(entry.file_name());

      if src.is_dir() {
        copy_file_to_trash(&src, &dst)?;
      } else {
        // Use copy with proper error handling
        if let Err(e) = fs::copy(&src, &dst) {
          return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
              "Failed to copy {} to {}: {}",
              src.display(),
              dst.display(),
              e
            ),
          ));
        }

        // Preserve file metadata
        if let Ok(metadata) = fs::metadata(src) {
          let _ = fs::set_permissions(&dst, metadata.permissions());
          if let Ok(times) = metadata.modified() {
            let _ = filetime::set_file_mtime(
              &dst,
              filetime::FileTime::from_system_time(times),
            );
          }
          if let Ok(times) = metadata.accessed() {
            let _ = filetime::set_file_atime(
              &dst,
              filetime::FileTime::from_system_time(times),
            );
          }
        }
      }
    }
  } else {
    // Use copy with proper error handling
    if let Err(e) = fs::copy(source, dest) {
      return Err(io::Error::new(
        io::ErrorKind::Other,
        format!(
          "Failed to copy {} to {}: {}",
          source.display(),
          dest.display(),
          e
        ),
      ));
    }

    // Preserve file metadata
    if let Ok(metadata) = fs::metadata(source) {
      let _ = fs::set_permissions(dest, metadata.permissions());
      if let Ok(times) = metadata.modified() {
        let _ = filetime::set_file_mtime(
          dest,
          filetime::FileTime::from_system_time(times),
        );
      }
      if let Ok(times) = metadata.accessed() {
        let _ = filetime::set_file_atime(
          dest,
          filetime::FileTime::from_system_time(times),
        );
      }
    }
  }
  Ok(())
}

pub fn move_file_to_trash(files: &[String]) {
  for file_path in files {
    let source_path = Path::new(file_path);

    // Determine which trash directory to use
    let trash_path = match get_trash_path_for_file(source_path) {
      Some(path) => path,
      None => {
        fail!("can: Could not determine trash path for {}", file_path)
      }
    };

    // Create trash directories if they don't exist
    create_trash_directories(&trash_path);

    let files_dir = trash_path.join("files");
    let info_dir = trash_path.join("info");

    let file_name = source_path
      .file_name()
      .and_then(|s| Some(s.to_string_lossy().to_string()))
      .unwrap_or_else(|| {
        fail!("can: Invalid file path for {}", file_path);
      });

    // Find unique name, checking both files/ and info/ directories
    let dest_name =
      find_unique_name(&files_dir, &info_dir, &file_name);
    let dest_path = files_dir.join(&dest_name);
    let info_path = info_dir.join(format!("{}.trashinfo", dest_name));

    // Ensure no name collision before we touch the filesystem
    if info_path.exists() || dest_path.exists() {
      fail!("can: Trash collision for {}", file_path);
    }

    // Move the file (or copy+remove on cross-fs rename failure)
    let move_res = fs::rename(source_path, &dest_path);
    if let Err(_rename_err) = move_res {
      // Cross‑filesystem: copy then delete the original
      if let Err(e) = copy_file_to_trash(source_path, &dest_path) {
        fail!("can: Failed to move {} to trash: {}", file_path, e);
      }
      if let Err(e) = fs::remove_file(source_path) {
        // Cleanup the partially copied file
        let _ = fs::remove_file(&dest_path);
        fail!(
          "can: Failed to remove original file {}: {}",
          file_path,
          e
        );
      }
    }

    // Create .trashinfo file atomically after move succeeds
    if let Err(err) =
      create_atomic_trashinfo(&info_path, source_path, &trash_path)
    {
      // If we cannot write the metadata, roll back the moved file
      let _ = fs::remove_file(&dest_path);
      fail!("can: Failed to create trashinfo: {}", err);
    }

    // Update directory‑sizes cache for moved directories
    if source_path.is_dir() {
      update_directorysizes_cache(&trash_path, &dest_name);
    }
  }
}

fn create_trash_directories(trash_path: &Path) {
  let files_dir = trash_path.join("files");
  let info_dir = trash_path.join("info");

  if !trash_path.exists() {
    fs::create_dir_all(trash_path)
      .expect("Failed to create trash directory");
  }
  if !files_dir.exists() {
    fs::create_dir_all(&files_dir)
      .expect("Failed to create trash files directory");
  }
  if !info_dir.exists() {
    fs::create_dir_all(&info_dir)
      .expect("Failed to create trash info directory");
  }
}

fn find_unique_name(
  files_dir: &Path,
  info_dir: &Path,
  original_name: &str,
) -> String {
  let mut name = original_name.to_string();
  let mut counter = 1;

  loop {
    let test_files_path = files_dir.join(&name);
    let test_info_path =
      info_dir.join(&format!("{}.trashinfo", name));

    if !test_files_path.exists() && !test_info_path.exists() {
      return name;
    }

    let extension_pos = original_name.rfind('.');
    let (base_name, extension) = match extension_pos {
      Some(pos) => {
        let base = &original_name[..pos];
        let ext = &original_name[pos..];
        (base.to_string(), ext.to_string())
      }
      None => (original_name.to_string(), String::new()),
    };

    name = format!("{}({}){}", base_name, counter, extension);
    counter += 1;
  }
}

fn create_atomic_trashinfo(
  info_path: &Path,
  original_path: &Path,
  trash_path: &Path,
) -> io::Result<()> {
  let temp_path = info_path.with_extension("trashinfo.tmp");

  // Determine if this is the home trash directory
  let home_trash = get_home_trash_path();
  let is_home_trash = trash_path == home_trash;

  let original_path_str = if is_home_trash {
    // For home trash, try to use relative paths as per spec
    let parent = trash_path.parent().unwrap_or(Path::new("/"));
    if let Ok(stripped) = original_path.strip_prefix(parent) {
      if stripped.as_os_str().is_empty()
        || stripped.as_os_str() == "."
      {
        original_path.display().to_string()
      } else {
        stripped.display().to_string()
      }
    } else {
      original_path.display().to_string()
    }
  } else {
    // For topdir trash, use absolute paths as per spec
    original_path.display().to_string()
  };

  // URL encode the path as required by the spec
  let encoded_path = encode(&original_path_str);

  let deletion_date = chrono::Local::now();
  let deletion_date_str =
    deletion_date.format("%Y-%m-%dT%H:%M:%S").to_string();

  let info_content = format!(
    "[Trash Info]\nPath={}\nDeletionDate={}\n",
    encoded_path, deletion_date_str
  );

  // Ensure parent directory exists
  if let Some(parent_dir) = info_path.parent() {
    fs::create_dir_all(parent_dir)?;
  }

  // Create temp file with exclusive access
  let file = fs::OpenOptions::new()
    .write(true)
    .create_new(true)
    .open(&temp_path)?;

  let mut writer = BufWriter::new(file);

  writer.write_all(info_content.as_bytes())?;
  writer.flush()?;

  // Atomic rename
  fs::rename(&temp_path, info_path)?;
  Ok(())
}

fn update_directorysizes_cache(trash_path: &Path, dir_name: &str) {
  let cache_path = trash_path.join("directorysizes");
  let dir_path = trash_path.join("files").join(dir_name);

  if let Ok(dir_size) = calculate_directory_size(&dir_path) {
    let mtime = get_trashinfo_mtime(trash_path, dir_name);

    // Read existing cache
    let mut cache_entries: HashMap<String, (u64, i64)> =
      HashMap::new();
    let mut cache_names: HashMap<String, String> = HashMap::new(); // Store URL-encoded names
    if let Ok(content) = fs::read_to_string(&cache_path) {
      for line in content.lines() {
        let mut iter = line.splitn(3, ' ');
        let size = iter.next().and_then(|s| s.parse::<u64>().ok());
        let mtime = iter.next().and_then(|s| s.parse::<i64>().ok());
        let encoded_name = iter.next().map(|s| s.to_string());
        if let (Some(size), Some(mtime), Some(encoded_name)) =
          (size, mtime, encoded_name)
        {
          let decoded_name =
            decode(&encoded_name).expect("Failed to decode path");
          cache_entries
            .insert(decoded_name.to_string(), (size, mtime));
          cache_names.insert(decoded_name.to_string(), encoded_name); // Keep the original encoded form
        }
      }
    }

    // Update entry (URL encode the directory name as per spec)
    let encoded_dir_name = encode(dir_name);
    cache_entries.insert(dir_name.to_string(), (dir_size, mtime));
    cache_names
      .insert(dir_name.to_string(), encoded_dir_name.to_string());

    // Write to temp then rename using buffered writer
    let temp_path = cache_path.with_extension("tmp");
    match fs::File::create(&temp_path).and_then(|file| {
      let mut writer = BufWriter::new(file);

      // First pass: scan the files directory to mark seen entries
      let files_dir = trash_path.join("files");
      let mut seen_entries = HashSet::new();
      if let Ok(entries) = fs::read_dir(&files_dir) {
        for entry in entries {
          if let Ok(entry) = entry {
            let entry_name = entry.file_name();
            let name_str = entry_name.to_string_lossy();
            if let Ok(decoded_name) = decode(&name_str) {
              seen_entries.insert(decoded_name.to_string());
            }
          }
        }
      }

      // Second pass: write entries, removing unseen ones per spec's algorithm
      for (name, (size, mtime)) in cache_entries {
        if seen_entries.contains(&name) {
          // Use the URL-encoded name from cache_names
          let encoded_name = cache_names.get(&name).unwrap_or(&name);
          let line = format!("{} {} {}\n", size, mtime, encoded_name);
          if let Err(e) = writer.write_all(line.as_bytes()) {
            return Err(e);
          }
        }
        // If not seen, the entry will be removed from cache (as per spec)
      }

      writer.flush()
    }) {
      Ok(_) => {
        if let Err(e) = fs::rename(&temp_path, &cache_path) {
          eprintln!(
            "Warning: failed to update directorysizes: {}",
            e
          );
          let _ = fs::remove_file(&temp_path); // Clean up temp file
        }
      }
      Err(e) => {
        eprintln!("Warning: failed to write directorysizes: {}", e);
        let _ = fs::remove_file(&temp_path); // Clean up temp file on error
      }
    }
  }
}

fn calculate_directory_size(dir_path: &Path) -> io::Result<u64> {
  let mut total_size = 0u64;

  if let Ok(entries) = fs::read_dir(dir_path) {
    for entry in entries {
      if let Ok(entry) = entry {
        let path = entry.path();
        if path.is_dir() {
          match calculate_directory_size(&path) {
            Ok(size) => total_size += size,
            Err(e) => {
              // Log the error but continue with other entries
              eprintln!(
                "Warning: Failed to calculate size of {}: {}",
                path.display(),
                e
              );
            }
          }
        } else {
          if let Ok(metadata) = fs::metadata(&path) {
            total_size += metadata.len();
          }
        }
      }
    }
  }

  Ok(total_size)
}

fn get_trashinfo_mtime(trash_path: &Path, dir_name: &str) -> i64 {
  let info_path = trash_path
    .join("info")
    .join(format!("{}.trashinfo", dir_name));
  if let Ok(metadata) = fs::metadata(&info_path) {
    if let Ok(modified) = metadata.modified() {
      // Convert to seconds since UNIX epoch
      if let Ok(duration) =
        modified.duration_since(SystemTime::UNIX_EPOCH)
      {
        return duration.as_secs() as i64;
      }
    }
  }
  0
}
