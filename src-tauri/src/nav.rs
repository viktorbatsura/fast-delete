//! Lazy folder listing for the navigator: one level at a time, without calculating folder sizes.

use std::fs;
use std::io;
use std::path::Path;

use serde::Serialize;

use crate::engine::fs_ext::is_dir_link;

/// Rendering more rows than this makes the UI sluggish; the rest is reported as a count.
pub const MAX_DIRS: usize = 5000;
pub const MAX_SUGGESTIONS: usize = 20;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntryInfo {
    pub name: String,
    pub path: String,
    /// Junction or directory symbolic link.
    pub is_link: bool,
    pub is_hidden: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirListing {
    pub path: String,
    pub parent: Option<String>,
    pub dirs: Vec<DirEntryInfo>,
    /// Folders beyond `MAX_DIRS` that were not returned.
    pub more_dirs: usize,
    /// Files directly in this folder (not recursive). Free to count while listing.
    pub file_count: u64,
    pub file_bytes: u64,
}

pub fn list_dir(path: &Path) -> io::Result<DirListing> {
    let path = std::path::absolute(path)?;

    let mut dirs = Vec::new();
    let mut file_count = 0;
    let mut file_bytes = 0;

    // Unreadable entries are skipped: the navigator shows what it can.
    for entry in fs::read_dir(&path)?.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() || is_dir_link(&file_type) {
            dirs.push(DirEntryInfo {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path().to_string_lossy().into_owned(),
                is_link: file_type.is_symlink(),
                is_hidden: is_hidden(&entry),
            });
        } else {
            file_count += 1;
            file_bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }

    dirs.sort_by_cached_key(|dir| dir.name.to_lowercase());
    let more_dirs = dirs.len().saturating_sub(MAX_DIRS);
    dirs.truncate(MAX_DIRS);

    Ok(DirListing {
        parent: path.parent().map(|p| p.to_string_lossy().into_owned()),
        path: path.to_string_lossy().into_owned(),
        dirs,
        more_dirs,
        file_count,
        file_bytes,
    })
}

/// Folder suggestions for the path input: `C:\Us` → `C:\Users`, `C:\Users\` → its subfolders.
pub fn suggest(input: &str) -> Vec<String> {
    let input = input.replace('/', "\\");
    let (folder, prefix) = match input.rfind('\\') {
        Some(index) => (&input[..=index], &input[index + 1..]),
        None => return Vec::new(),
    };

    let Ok(listing) = list_dir(Path::new(folder)) else {
        return Vec::new();
    };

    let prefix = prefix.to_lowercase();
    listing
        .dirs
        .into_iter()
        .filter(|dir| dir.name.to_lowercase().starts_with(&prefix))
        .take(MAX_SUGGESTIONS)
        .map(|dir| dir.path)
        .collect()
}

#[cfg(windows)]
fn is_hidden(entry: &fs::DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;

    entry
        .metadata()
        .map(|m| m.file_attributes() & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) != 0)
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn is_hidden(entry: &fs::DirEntry) -> bool {
    entry.file_name().to_string_lossy().starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;

    fn sample(tmp: &TestDir) {
        for name in ["beta", "Alpha", "gamma"] {
            fs::create_dir(tmp.0.join(name)).unwrap();
        }
        fs::write(tmp.0.join("a.txt"), b"12345").unwrap();
        fs::write(tmp.0.join("b.txt"), b"123").unwrap();
    }

    #[test]
    fn lists_folders_sorted_and_counts_files() {
        let tmp = TestDir::new("nav-list");
        sample(&tmp);

        let listing = list_dir(&tmp.0).unwrap();
        let names: Vec<_> = listing.dirs.iter().map(|d| d.name.as_str()).collect();

        assert_eq!(names, ["Alpha", "beta", "gamma"]);
        assert_eq!(listing.file_count, 2);
        assert_eq!(listing.file_bytes, 8);
        assert_eq!(listing.more_dirs, 0);
        assert!(listing.parent.is_some());
    }

    #[test]
    fn missing_folder_is_an_error() {
        let tmp = TestDir::new("nav-missing");
        assert!(list_dir(&tmp.0.join("nope")).is_err());
    }

    #[test]
    fn suggests_matching_subfolders() {
        let tmp = TestDir::new("nav-suggest");
        sample(&tmp);
        let base = tmp.0.to_string_lossy().into_owned();

        let by_prefix = suggest(&format!("{base}\\g"));
        assert_eq!(by_prefix.len(), 1);
        assert!(by_prefix[0].ends_with("gamma"));

        let all = suggest(&format!("{base}\\"));
        assert_eq!(all.len(), 3);

        assert!(suggest("no-separator").is_empty());
    }
}
