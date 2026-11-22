//! Guards against deleting locations that would break Windows or the user profile.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use super::fs_ext::is_dir_link;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyError {
    NotAbsolute,
    NotFound,
    NotADirectory,
    DriveRoot,
    SystemLocation(PathBuf),
    ProtectedLocation(PathBuf),
    Io(String),
}

impl fmt::Display for SafetyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAbsolute => write!(f, "path must be absolute"),
            Self::NotFound => write!(f, "folder does not exist"),
            Self::NotADirectory => write!(f, "path is not a folder"),
            Self::DriveRoot => write!(f, "deleting a whole drive is not allowed"),
            Self::SystemLocation(p) => write!(f, "system location is protected: {}", p.display()),
            Self::ProtectedLocation(p) => write!(f, "protected location: {}", p.display()),
            Self::Io(msg) => write!(f, "cannot access path: {msg}"),
        }
    }
}

impl std::error::Error for SafetyError {}

#[derive(Debug, Clone)]
pub struct SafetyRules {
    /// The location itself, everything inside it and all its ancestors are refused.
    system_roots: Vec<PathBuf>,
    /// The location itself and all its ancestors are refused; subfolders are allowed.
    protected: Vec<PathBuf>,
}

impl SafetyRules {
    pub fn new(system_roots: Vec<PathBuf>, protected: Vec<PathBuf>) -> Self {
        Self {
            system_roots,
            protected,
        }
    }

    /// Builds the rules from the current Windows environment.
    pub fn from_environment() -> Self {
        let system_roots = [
            "SystemRoot",
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramW6432",
        ]
        .into_iter()
        .filter_map(env_path)
        .collect();

        let mut protected: Vec<PathBuf> = [
            "ProgramData",
            "USERPROFILE",
            "APPDATA",
            "LOCALAPPDATA",
            "PUBLIC",
            "OneDrive",
        ]
        .into_iter()
        .filter_map(env_path)
        .collect();

        if let Some(app_dir) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
        {
            protected.push(app_dir);
        }

        Self::new(system_roots, protected)
    }

    /// Lexical check only, no file-system access. Returns the normalized absolute path.
    pub fn check_rules(&self, path: &Path) -> Result<PathBuf, SafetyError> {
        if !path.is_absolute() {
            return Err(SafetyError::NotAbsolute);
        }

        // Resolves `.` and `..` so tricks like `C:\Users\me\..\..\Windows` are caught.
        let normalized = std::path::absolute(path).map_err(|e| SafetyError::Io(e.to_string()))?;
        if normalized.parent().is_none() {
            return Err(SafetyError::DriveRoot);
        }

        let key = comparable(&normalized);

        for root in &self.system_roots {
            let root_key = comparable(root);
            if key.starts_with(&root_key) || root_key.starts_with(&key) {
                return Err(SafetyError::SystemLocation(root.clone()));
            }
        }

        for location in &self.protected {
            if comparable(location).starts_with(&key) {
                return Err(SafetyError::ProtectedLocation(location.clone()));
            }
        }

        Ok(normalized)
    }

    /// Full check before deletion: rules, existence, folder type and links in the parent chain.
    pub fn check_deletable(&self, path: &Path) -> Result<PathBuf, SafetyError> {
        let normalized = self.check_rules(path)?;

        let meta = std::fs::symlink_metadata(&normalized).map_err(io_error)?;
        if !(meta.is_dir() || is_dir_link(&meta.file_type())) {
            return Err(SafetyError::NotADirectory);
        }

        // A junction above the target could point into a protected location,
        // so the real location must pass the rules as well.
        let real_location = resolve_parent(&normalized)?;
        self.check_rules(&real_location)?;

        Ok(normalized)
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn io_error(error: io::Error) -> SafetyError {
    match error.kind() {
        io::ErrorKind::NotFound => SafetyError::NotFound,
        _ => SafetyError::Io(error.to_string()),
    }
}

/// Resolves links in the parent chain and keeps the final component untouched:
/// if the target itself is a link, only the link is deleted, never its target.
fn resolve_parent(path: &Path) -> Result<PathBuf, SafetyError> {
    let parent = path.parent().ok_or(SafetyError::DriveRoot)?;
    let name = path.file_name().ok_or(SafetyError::DriveRoot)?;
    let real_parent = std::fs::canonicalize(parent).map_err(io_error)?;
    Ok(real_parent.join(name))
}

/// Case-insensitive form without the `\\?\` prefix that `canonicalize` adds on Windows.
fn comparable(path: &Path) -> PathBuf {
    let raw = path.to_string_lossy().replace('/', "\\").to_lowercase();

    let plain = if let Some(rest) = raw.strip_prefix(r"\\?\unc\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = raw.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        raw
    };

    PathBuf::from(plain)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn rules() -> SafetyRules {
        SafetyRules::new(
            vec![
                PathBuf::from(r"C:\Windows"),
                PathBuf::from(r"C:\Program Files"),
            ],
            vec![
                PathBuf::from(r"C:\Users\bob"),
                PathBuf::from(r"C:\Users\bob\AppData\Local"),
            ],
        )
    }

    fn check(path: &str) -> Result<PathBuf, SafetyError> {
        rules().check_rules(Path::new(path))
    }

    #[test]
    fn rejects_relative_paths() {
        assert_eq!(check(r"some\folder"), Err(SafetyError::NotAbsolute));
    }

    #[test]
    fn rejects_drive_roots() {
        assert_eq!(check(r"C:\"), Err(SafetyError::DriveRoot));
        assert_eq!(check(r"D:\"), Err(SafetyError::DriveRoot));
    }

    #[test]
    fn rejects_system_roots_their_content_and_ancestors() {
        assert!(matches!(
            check(r"C:\Windows"),
            Err(SafetyError::SystemLocation(_))
        ));
        assert!(matches!(
            check(r"C:\Windows\Temp"),
            Err(SafetyError::SystemLocation(_))
        ));
        assert!(matches!(
            check(r"C:\Program Files\App"),
            Err(SafetyError::SystemLocation(_))
        ));
    }

    #[test]
    fn ignores_case_slashes_and_trailing_separators() {
        assert!(matches!(
            check("c:/WINDOWS/"),
            Err(SafetyError::SystemLocation(_))
        ));
    }

    #[test]
    fn catches_parent_dir_traversal() {
        let tricky = r"C:\Users\bob\AppData\..\..\..\Windows\Temp";
        assert!(matches!(check(tricky), Err(SafetyError::SystemLocation(_))));
    }

    #[test]
    fn rejects_protected_locations_and_their_ancestors() {
        assert!(matches!(
            check(r"C:\Users\bob"),
            Err(SafetyError::ProtectedLocation(_))
        ));
        assert!(matches!(
            check(r"C:\Users"),
            Err(SafetyError::ProtectedLocation(_))
        ));
        assert!(matches!(
            check(r"C:\Users\bob\AppData\Local"),
            Err(SafetyError::ProtectedLocation(_))
        ));
    }

    #[test]
    fn allows_subfolders_of_protected_locations() {
        let cache = r"C:\Users\bob\AppData\Local\Yarn\Cache";
        assert_eq!(check(cache), Ok(PathBuf::from(cache)));
        assert!(check(r"D:\Projects\old-build").is_ok());
    }

    #[test]
    fn check_deletable_validates_the_file_system() {
        let base = std::env::temp_dir().join(format!("fd-safety-{}", std::process::id()));
        let folder = base.join("folder");
        let file = base.join("file.txt");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(&file, b"x").unwrap();

        let rules = SafetyRules::new(Vec::new(), Vec::new());
        assert!(rules.check_deletable(&folder).is_ok());
        assert_eq!(
            rules.check_deletable(&file),
            Err(SafetyError::NotADirectory)
        );
        assert_eq!(
            rules.check_deletable(&base.join("missing")),
            Err(SafetyError::NotFound)
        );

        std::fs::remove_dir_all(&base).unwrap();
    }
}
