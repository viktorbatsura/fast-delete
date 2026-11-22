//! Small file-system helpers shared by the engine and the navigator.

use std::fs::FileType;

/// True for junctions and directory symbolic links. On Windows `FileType::is_dir()` is false for them.
#[cfg(windows)]
pub fn is_dir_link(file_type: &FileType) -> bool {
    use std::os::windows::fs::FileTypeExt;
    file_type.is_symlink_dir()
}

#[cfg(not(windows))]
pub fn is_dir_link(_file_type: &FileType) -> bool {
    false
}
