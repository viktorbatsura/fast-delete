//! Logical drives for the navigator: letter, type, label, file system and free space.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DriveKind {
    Fixed,
    Removable,
    Network,
    Optical,
    RamDisk,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    /// Root path, e.g. `C:\`.
    pub path: String,
    pub letter: char,
    pub kind: DriveKind,
    pub label: Option<String>,
    pub file_system: Option<String>,
    pub total_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
}

#[cfg(windows)]
pub fn list_drives() -> Vec<DriveInfo> {
    win::list_drives()
}

#[cfg(not(windows))]
pub fn list_drives() -> Vec<DriveInfo> {
    Vec::new()
}

#[cfg(windows)]
mod win {
    use std::ptr;

    use windows_sys::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives, GetVolumeInformationW,
    };
    use windows_sys::Win32::System::Diagnostics::Debug::{SEM_FAILCRITICALERRORS, SetThreadErrorMode};

    use super::{DriveInfo, DriveKind};

    // Return values of GetDriveTypeW (WinBase.h).
    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    const DRIVE_REMOTE: u32 = 4;
    const DRIVE_CDROM: u32 = 5;
    const DRIVE_RAMDISK: u32 = 6;

    /// MAX_PATH + 1, the documented maximum for volume label and file system name buffers.
    const NAME_BUFFER_LEN: usize = 261;

    pub fn list_drives() -> Vec<DriveInfo> {
        // Without this an empty card reader can pop up a blocking "Insert a disk" dialog.
        // SAFETY: plain flag call for the current thread; the old mode is not needed.
        unsafe {
            SetThreadErrorMode(SEM_FAILCRITICALERRORS, ptr::null_mut());
        }

        // SAFETY: no arguments; returns a bitmask where bit 0 is A:, bit 1 is B: and so on.
        let mask = unsafe { GetLogicalDrives() };

        (0..26u8)
            .filter(|bit| mask & (1 << bit) != 0)
            .map(|bit| drive_info(char::from(b'A' + bit)))
            .collect()
    }

    fn drive_info(letter: char) -> DriveInfo {
        let path = format!("{letter}:\\");
        let root = to_wide(&path);

        // SAFETY: `root` is a NUL-terminated UTF-16 string that lives until the end of this function.
        let kind = match unsafe { GetDriveTypeW(root.as_ptr()) } {
            DRIVE_FIXED => DriveKind::Fixed,
            DRIVE_REMOVABLE => DriveKind::Removable,
            DRIVE_REMOTE => DriveKind::Network,
            DRIVE_CDROM => DriveKind::Optical,
            DRIVE_RAMDISK => DriveKind::RamDisk,
            _ => DriveKind::Unknown,
        };

        // Disconnected network shares and empty optical trays can block for seconds.
        let queryable = matches!(kind, DriveKind::Fixed | DriveKind::Removable | DriveKind::RamDisk);
        let (label, file_system) = if queryable { volume_names(&root) } else { (None, None) };
        let (total_bytes, free_bytes) = if queryable { disk_space(&root) } else { (None, None) };

        DriveInfo {
            path,
            letter,
            kind,
            label,
            file_system,
            total_bytes,
            free_bytes,
        }
    }

    fn volume_names(root: &[u16]) -> (Option<String>, Option<String>) {
        let mut label = [0u16; NAME_BUFFER_LEN];
        let mut file_system = [0u16; NAME_BUFFER_LEN];

        // SAFETY: both buffers are valid for the lengths passed; unused out-parameters are null.
        let ok = unsafe {
            GetVolumeInformationW(
                root.as_ptr(),
                label.as_mut_ptr(),
                NAME_BUFFER_LEN as u32,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                file_system.as_mut_ptr(),
                NAME_BUFFER_LEN as u32,
            )
        };

        if ok == 0 {
            return (None, None);
        }
        (from_wide(&label), from_wide(&file_system))
    }

    fn disk_space(root: &[u16]) -> (Option<u64>, Option<u64>) {
        let mut free_for_user = 0u64;
        let mut total = 0u64;

        // SAFETY: the out-pointers point to live locals; the last parameter is optional.
        let ok = unsafe {
            GetDiskFreeSpaceExW(root.as_ptr(), &mut free_for_user, &mut total, ptr::null_mut())
        };

        if ok == 0 {
            (None, None)
        } else {
            (Some(total), Some(free_for_user))
        }
    }

    fn to_wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn from_wide(buffer: &[u16]) -> Option<String> {
        let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
        (len > 0).then(|| String::from_utf16_lossy(&buffer[..len]))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn system_drive_is_listed_as_fixed_with_space_info() {
        let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let letter = system_drive.chars().next().unwrap().to_ascii_uppercase();

        let drives = list_drives();
        let system = drives
            .iter()
            .find(|d| d.letter == letter)
            .expect("system drive is missing");

        assert_eq!(system.kind, DriveKind::Fixed);
        assert_eq!(system.path, format!("{letter}:\\"));
        assert!(system.total_bytes.unwrap_or(0) > 0);
        assert!(system.free_bytes.unwrap_or(0) <= system.total_bytes.unwrap_or(0));
        assert_eq!(system.file_system.as_deref(), Some("NTFS"));
    }
}
