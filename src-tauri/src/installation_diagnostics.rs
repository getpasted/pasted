use serde::Serialize;
use std::path::{Path, PathBuf};

pub const APP_IDENTIFIER: &str = "software.jjj.pasted";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstallationDiagnostics {
    pub app_version: String,
    pub build_kind: String,
    pub platform: String,
    pub architecture: String,
    pub bundle_identifier: String,
    pub app_path: String,
    pub data_path: String,
    pub database_size_bytes: u64,
    pub signing_status: String,
    pub signing_identity: Option<String>,
    pub signing_team_id: Option<String>,
    pub notarization_status: String,
    pub cli_path: Option<String>,
}

impl InstallationDiagnostics {
    pub fn collect(app_path: PathBuf, data_path: PathBuf) -> Self {
        let database_path = data_path.join("pasted.db");
        Self::collect_with_database(app_path, data_path, database_path)
    }

    pub fn collect_with_database(
        app_path: PathBuf,
        data_path: PathBuf,
        database_path: PathBuf,
    ) -> Self {
        let database_size_bytes = database_disk_usage(&database_path);
        let cli_path = sibling_cli_path(&app_path).map(|path| display_path(&path));
        let signature = inspect_signature(&app_path);

        Self {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            build_kind: if cfg!(debug_assertions) {
                "Development".to_string()
            } else {
                "Release".to_string()
            },
            platform: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            bundle_identifier: APP_IDENTIFIER.to_string(),
            app_path: display_path(&app_path),
            data_path: display_path(&data_path),
            database_size_bytes,
            signing_status: signature.status,
            signing_identity: signature.identity,
            signing_team_id: signature.team_id,
            notarization_status: signature.notarization,
            cli_path,
        }
    }

    pub fn plain_text(&self) -> String {
        let mut lines = vec![
            format!("Pasted {} ({})", self.app_version, self.build_kind),
            format!("Platform: {} ({})", self.platform, self.architecture),
            format!("Bundle identifier: {}", self.bundle_identifier),
            format!("Application: {}", self.app_path),
            format!("Data: {}", self.data_path),
            format!("Database: {} bytes", self.database_size_bytes),
            format!("Code signing: {}", self.signing_status),
            format!("Notarization: {}", self.notarization_status),
        ];
        if let Some(identity) = &self.signing_identity {
            lines.push(format!("Signing identity: {identity}"));
        }
        if let Some(team_id) = &self.signing_team_id {
            lines.push(format!("Signing team: {team_id}"));
        }
        lines.push(format!(
            "CLI: {}",
            self.cli_path
                .as_deref()
                .unwrap_or("Not installed beside Pasted")
        ));
        lines.join("\n")
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn database_disk_usage(path: &Path) -> u64 {
    ["", "-wal", "-shm"]
        .iter()
        .filter_map(|suffix| std::fs::metadata(format!("{}{suffix}", path.display())).ok())
        .map(|metadata| metadata.len())
        .sum()
}

fn sibling_cli_path(app_path: &Path) -> Option<PathBuf> {
    let executable = if app_path.extension().is_some_and(|value| value == "app") {
        app_path.join("Contents/MacOS/pasted")
    } else {
        app_path.parent()?.join(if cfg!(windows) {
            "pasted.exe"
        } else {
            "pasted"
        })
    };
    executable.is_file().then_some(executable)
}

#[derive(Default)]
struct SignatureInspection {
    status: String,
    identity: Option<String>,
    team_id: Option<String>,
    notarization: String,
}

#[cfg(target_os = "macos")]
fn inspect_signature(app_path: &Path) -> SignatureInspection {
    use std::process::Command;

    let codesign = Command::new("/usr/bin/codesign")
        .args(["-d", "--verbose=4"])
        .arg(app_path)
        .output();
    let (status, identity, team_id) = match codesign {
        Ok(output) => {
            let details = String::from_utf8_lossy(&output.stderr);
            parse_codesign_details(&details)
        }
        Err(_) => ("Unavailable".to_string(), None, None),
    };

    let notarization = Command::new("/usr/sbin/spctl")
        .args(["-a", "-vv", "--type", "execute"])
        .arg(app_path)
        .output()
        .map(|output| {
            if output.status.success() {
                "Accepted by Gatekeeper".to_string()
            } else if cfg!(debug_assertions) {
                "Not expected for development builds".to_string()
            } else {
                "Not accepted by Gatekeeper".to_string()
            }
        })
        .unwrap_or_else(|_| "Unavailable".to_string());

    SignatureInspection {
        status,
        identity,
        team_id,
        notarization,
    }
}

#[cfg(not(target_os = "macos"))]
fn inspect_signature(_app_path: &Path) -> SignatureInspection {
    SignatureInspection {
        status: "Managed by the operating system".to_string(),
        notarization: "Not applicable".to_string(),
        ..Default::default()
    }
}

#[cfg(target_os = "macos")]
fn parse_codesign_details(details: &str) -> (String, Option<String>, Option<String>) {
    let authority = detail_value(details, "Authority=");
    let team_id = detail_value(details, "TeamIdentifier=")
        .filter(|value| value != "not set" && !value.is_empty());
    let signature = detail_value(details, "Signature=");
    let status = if authority.is_some() {
        "Developer ID".to_string()
    } else if signature.as_deref() == Some("adhoc") {
        "Ad hoc".to_string()
    } else if details.contains("not signed at all") {
        "Unsigned".to_string()
    } else {
        "Unknown".to_string()
    };
    (status, authority, team_id)
}

#[cfg(target_os = "macos")]
fn detail_value(details: &str, prefix: &str) -> Option<String> {
    details
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_contains_public_installation_fields() {
        let details = InstallationDiagnostics {
            app_version: "1.0.0".into(),
            build_kind: "Release".into(),
            platform: "macos".into(),
            architecture: "aarch64".into(),
            bundle_identifier: APP_IDENTIFIER.into(),
            app_path: "/Applications/Pasted.app".into(),
            data_path: "/tmp/Pasted".into(),
            database_size_bytes: 42,
            signing_status: "Developer ID".into(),
            signing_identity: Some("Developer ID Application: Example".into()),
            signing_team_id: Some("ABCDE12345".into()),
            notarization_status: "Accepted by Gatekeeper".into(),
            cli_path: None,
        };
        let text = details.plain_text();
        assert!(text.contains("Pasted 1.0.0 (Release)"));
        assert!(text.contains(APP_IDENTIFIER));
        assert!(text.contains("Signing team: ABCDE12345"));
        assert!(!text.contains("clipboard"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parses_developer_id_codesign_output() {
        let output = "Executable=/Applications/Pasted.app/Contents/MacOS/pasted-app\nAuthority=Developer ID Application: John Jacoby (ABCDE12345)\nTeamIdentifier=ABCDE12345\n";
        let (status, identity, team) = parse_codesign_details(output);
        assert_eq!(status, "Developer ID");
        assert_eq!(
            identity.as_deref(),
            Some("Developer ID Application: John Jacoby (ABCDE12345)")
        );
        assert_eq!(team.as_deref(), Some("ABCDE12345"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn recognizes_ad_hoc_codesign_output() {
        let (status, identity, team) =
            parse_codesign_details("Signature=adhoc\nTeamIdentifier=not set\n");
        assert_eq!(status, "Ad hoc");
        assert_eq!(identity, None);
        assert_eq!(team, None);
    }
}
