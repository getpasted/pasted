use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

static STORAGE_PROTECTION_CACHE: Lazy<Mutex<Option<(PathBuf, StorageProtectionInfo)>>> =
    Lazy::new(|| Mutex::new(None));

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageProtectionStatus {
    Protected,
    NotDetected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageProtectionInfo {
    pub status: StorageProtectionStatus,
    pub technology: Option<String>,
    pub summary: String,
    pub detail: String,
}

impl StorageProtectionInfo {
    fn protected(technology: &str) -> Self {
        Self {
            status: StorageProtectionStatus::Protected,
            technology: Some(technology.to_string()),
            summary: format!("{technology} is on"),
            detail: "The volume containing this database is encrypted.".to_string(),
        }
    }

    fn not_detected(technology: Option<&str>) -> Self {
        Self {
            status: StorageProtectionStatus::NotDetected,
            technology: technology.map(str::to_string),
            summary: technology
                .map(|name| format!("{name} is off"))
                .unwrap_or_else(|| "Volume encryption was not detected".to_string()),
            detail: "App Lock does not encrypt the database.".to_string(),
        }
    }

    fn unknown() -> Self {
        Self {
            status: StorageProtectionStatus::Unknown,
            technology: None,
            summary: "Volume encryption could not be determined".to_string(),
            detail: "Check the operating system’s storage security settings.".to_string(),
        }
    }
}

pub fn inspect(database_path: &Path) -> StorageProtectionInfo {
    #[cfg(target_os = "macos")]
    {
        inspect_macos(database_path)
    }
    #[cfg(target_os = "windows")]
    {
        inspect_windows(database_path)
    }
    #[cfg(target_os = "linux")]
    {
        inspect_linux(database_path)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = database_path;
        StorageProtectionInfo::unknown()
    }
}

pub fn inspect_cached(database_path: &Path) -> StorageProtectionInfo {
    inspect_cached_with(database_path, inspect)
}

fn inspect_cached_with(
    database_path: &Path,
    inspector: impl FnOnce(&Path) -> StorageProtectionInfo,
) -> StorageProtectionInfo {
    let mut cached = STORAGE_PROTECTION_CACHE.lock();
    if let Some((cached_path, cached_info)) = cached.as_ref() {
        if cached_path == database_path {
            return cached_info.clone();
        }
    }
    let info = inspector(database_path);
    *cached = Some((database_path.to_path_buf(), info.clone()));
    info
}

#[cfg(target_os = "macos")]
fn inspect_macos(database_path: &Path) -> StorageProtectionInfo {
    let target = database_path.parent().unwrap_or(database_path);
    let Ok(filesystem) = Command::new("/bin/df").arg("-P").arg(target).output() else {
        return StorageProtectionInfo::unknown();
    };
    if !filesystem.status.success() {
        return StorageProtectionInfo::unknown();
    }
    let Some(device) = parse_df_device(&filesystem.stdout) else {
        return StorageProtectionInfo::unknown();
    };
    let Ok(output) = Command::new("/usr/sbin/diskutil")
        .args(["info", "-plist"])
        .arg(device)
        .output()
    else {
        return StorageProtectionInfo::unknown();
    };
    if !output.status.success() {
        return StorageProtectionInfo::unknown();
    }
    parse_diskutil_plist(&output.stdout)
}

#[cfg(any(target_os = "macos", test))]
fn parse_df_device(bytes: &[u8]) -> Option<&str> {
    std::str::from_utf8(bytes)
        .ok()?
        .lines()
        .nth(1)?
        .split_whitespace()
        .next()
}

#[cfg(any(target_os = "macos", test))]
fn parse_diskutil_plist(bytes: &[u8]) -> StorageProtectionInfo {
    let Ok(value) = plist::Value::from_reader(std::io::Cursor::new(bytes)) else {
        return StorageProtectionInfo::unknown();
    };
    let Some(dictionary) = value.as_dictionary() else {
        return StorageProtectionInfo::unknown();
    };
    let file_vault = dictionary
        .get("FileVault")
        .and_then(plist::Value::as_boolean);
    let encrypted = dictionary
        .get("Encrypted")
        .and_then(plist::Value::as_boolean);
    let encryption = dictionary
        .get("Encryption")
        .and_then(plist::Value::as_boolean);
    if file_vault == Some(true) {
        return StorageProtectionInfo::protected("FileVault");
    }
    if encrypted == Some(true) || encryption == Some(true) {
        return StorageProtectionInfo::protected("APFS encryption");
    }
    if file_vault == Some(false) || encrypted == Some(false) || encryption == Some(false) {
        return StorageProtectionInfo::not_detected(Some("FileVault"));
    }
    StorageProtectionInfo::unknown()
}

#[cfg(target_os = "windows")]
fn inspect_windows(database_path: &Path) -> StorageProtectionInfo {
    let path = database_path.to_string_lossy();
    let Some(drive) = windows_drive(&path) else {
        return StorageProtectionInfo::unknown();
    };
    let script = "param([string]$MountPoint) $v = Get-BitLockerVolume -MountPoint $MountPoint -ErrorAction Stop; [pscustomobject]@{ protectionStatus = [int]$v.ProtectionStatus } | ConvertTo-Json -Compress";
    let Ok(output) = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script, drive])
        .output()
    else {
        return StorageProtectionInfo::unknown();
    };
    if !output.status.success() {
        return StorageProtectionInfo::unknown();
    }
    parse_bitlocker_json(&output.stdout)
}

#[cfg(any(target_os = "windows", test))]
fn windows_drive(path: &str) -> Option<&str> {
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        Some(&path[..2])
    } else {
        None
    }
}

#[cfg(any(target_os = "windows", test))]
fn parse_bitlocker_json(bytes: &[u8]) -> StorageProtectionInfo {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return StorageProtectionInfo::unknown();
    };
    match value
        .get("protectionStatus")
        .and_then(serde_json::Value::as_i64)
    {
        Some(1) => StorageProtectionInfo::protected("BitLocker"),
        Some(0) => StorageProtectionInfo::not_detected(Some("BitLocker")),
        _ => StorageProtectionInfo::unknown(),
    }
}

#[cfg(target_os = "linux")]
fn inspect_linux(database_path: &Path) -> StorageProtectionInfo {
    let target = database_path.parent().unwrap_or(database_path);
    let Ok(mount) = Command::new("findmnt")
        .args(["--noheadings", "--output", "SOURCE,FSTYPE", "--target"])
        .arg(target)
        .output()
    else {
        return StorageProtectionInfo::unknown();
    };
    if !mount.status.success() {
        return StorageProtectionInfo::unknown();
    }
    let mount = String::from_utf8_lossy(&mount.stdout);
    let mut fields = mount.split_whitespace();
    let Some(source) = fields
        .next()
        .map(|value| value.split_once('[').map_or(value, |(device, _)| device))
    else {
        return StorageProtectionInfo::unknown();
    };
    let filesystem = fields.next().unwrap_or_default();
    if matches!(filesystem, "ecryptfs" | "encfs") {
        return StorageProtectionInfo::protected("filesystem encryption");
    }
    if !source.starts_with("/dev/") {
        return StorageProtectionInfo::unknown();
    }
    let Ok(blocks) = Command::new("lsblk")
        .args([
            "--inverse",
            "--noheadings",
            "--output",
            "TYPE,FSTYPE",
            source,
        ])
        .output()
    else {
        return StorageProtectionInfo::unknown();
    };
    if !blocks.status.success() {
        return StorageProtectionInfo::unknown();
    }
    parse_lsblk_output(&blocks.stdout)
}

#[cfg(any(target_os = "linux", test))]
fn parse_lsblk_output(bytes: &[u8]) -> StorageProtectionInfo {
    let output = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    if output.lines().any(|line| {
        line.split_whitespace()
            .any(|field| field == "crypt" || field == "crypto_luks")
    }) {
        StorageProtectionInfo::protected("LUKS")
    } else if output.trim().is_empty() {
        StorageProtectionInfo::unknown()
    } else {
        StorageProtectionInfo::not_detected(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_file_vault_and_plain_apfs_fixtures() {
        let protected = br#"<?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0"><dict><key>FileVault</key><true/></dict></plist>"#;
        assert_eq!(
            parse_diskutil_plist(protected).status,
            StorageProtectionStatus::Protected
        );

        let unprotected = br#"<?xml version="1.0" encoding="UTF-8"?>
        <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
        <plist version="1.0"><dict><key>FileVault</key><false/></dict></plist>"#;
        assert_eq!(
            parse_diskutil_plist(unprotected).status,
            StorageProtectionStatus::NotDetected
        );
    }

    #[test]
    fn resolves_the_device_from_posix_df_output() {
        let output = b"Filesystem 512-blocks Used Available Capacity Mounted on\n/dev/disk3s5 100 50 50 50% /System/Volumes/Data\n";
        assert_eq!(parse_df_device(output), Some("/dev/disk3s5"));
        assert_eq!(parse_df_device(b"Filesystem only\n"), None);
    }

    #[test]
    fn parses_bitlocker_fixtures() {
        assert_eq!(
            parse_bitlocker_json(br#"{"protectionStatus":1}"#).status,
            StorageProtectionStatus::Protected
        );
        assert_eq!(
            parse_bitlocker_json(br#"{"protectionStatus":0}"#).status,
            StorageProtectionStatus::NotDetected
        );
        assert_eq!(
            parse_bitlocker_json(b"not json").status,
            StorageProtectionStatus::Unknown
        );
    }

    #[test]
    fn parses_linux_block_device_fixtures() {
        assert_eq!(
            parse_lsblk_output(b"part ext4\ncrypt crypto_LUKS\npart\n").status,
            StorageProtectionStatus::Protected
        );
        assert_eq!(
            parse_lsblk_output(b"part ext4\ndisk\n").status,
            StorageProtectionStatus::NotDetected
        );
        assert_eq!(
            parse_lsblk_output(b"").status,
            StorageProtectionStatus::Unknown
        );
    }

    #[test]
    fn accepts_only_windows_drive_paths() {
        assert_eq!(windows_drive(r"C:\Users\Pasted\pasted.db"), Some("C:"));
        assert_eq!(windows_drive(r"\\server\share\pasted.db"), None);
        assert_eq!(windows_drive("/tmp/pasted.db"), None);
    }

    #[test]
    fn caches_one_result_for_the_active_database_path() {
        let path = Path::new("/storage-protection-cache-test/pasted.db");
        let first = inspect_cached_with(path, |_| StorageProtectionInfo::protected("Test"));
        let second = inspect_cached_with(path, |_| StorageProtectionInfo::unknown());
        assert_eq!(first, second);
        assert_eq!(second.technology.as_deref(), Some("Test"));
    }
}
