use std::collections::BTreeMap;

use semver::Version;
use serde::{Deserialize, Serialize};

pub const STABLE_FEED_URL: &str =
    "https://github.com/getpasted/pasted/releases/download/updater-stable/latest.json";
pub const PRERELEASE_FEED_URL: &str =
    "https://github.com/getpasted/pasted/releases/download/updater-prerelease/latest.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannel {
    Stable,
    Prerelease,
}

impl UpdateChannel {
    pub fn feed_url(self) -> &'static str {
        match self {
            Self::Stable => STABLE_FEED_URL,
            Self::Prerelease => PRERELEASE_FEED_URL,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct StaticUpdateManifest {
    pub version: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub pub_date: Option<String>,
    pub platforms: BTreeMap<String, StaticUpdatePlatform>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StaticUpdatePlatform {
    pub url: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckReport {
    pub schema_version: u8,
    pub current_version: String,
    pub channel: UpdateChannel,
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub url: Option<String>,
}

pub fn channel_for_version(version: &str) -> Result<UpdateChannel, String> {
    let parsed = parse_version(version)?;
    Ok(if parsed.pre.is_empty() {
        UpdateChannel::Stable
    } else {
        UpdateChannel::Prerelease
    })
}

pub fn platform_key(os: &str, architecture: &str) -> Result<String, String> {
    let os = match os {
        "macos" | "darwin" => "darwin",
        "windows" => "windows",
        "linux" => "linux",
        value => return Err(format!("Updates are unavailable on platform '{value}'")),
    };
    let architecture = match architecture {
        "aarch64" | "arm64" => "aarch64",
        "x86_64" | "amd64" => "x86_64",
        "i686" => "i686",
        "armv7" => "armv7",
        value => {
            return Err(format!(
                "Updates are unavailable for architecture '{value}'"
            ))
        }
    };
    Ok(format!("{os}-{architecture}"))
}

pub fn evaluate_manifest(
    current_version: &str,
    platform: &str,
    contents: &str,
) -> Result<UpdateCheckReport, String> {
    let current = parse_version(current_version)?;
    let channel = channel_for_version(current_version)?;
    let manifest: StaticUpdateManifest = serde_json::from_str(contents)
        .map_err(|error| format!("Invalid update manifest: {error}"))?;
    let remote = parse_version(&manifest.version)?;
    if channel == UpdateChannel::Stable && !remote.pre.is_empty() {
        return Err("The stable update channel announced a prerelease".to_string());
    }
    let platform_release = manifest
        .platforms
        .get(platform)
        .ok_or_else(|| format!("The update manifest does not include platform '{platform}'"))?;
    if platform_release.signature.trim().is_empty() {
        return Err(format!("The update signature for '{platform}' is empty"));
    }
    let url = url::Url::parse(&platform_release.url)
        .map_err(|error| format!("Invalid update URL for '{platform}': {error}"))?;
    if url.scheme() != "https" {
        return Err(format!("The update URL for '{platform}' must use HTTPS"));
    }
    let available = remote > current;
    Ok(UpdateCheckReport {
        schema_version: 1,
        current_version: current_version.to_string(),
        channel,
        available,
        version: available.then_some(manifest.version),
        notes: available.then_some(manifest.notes).flatten(),
        pub_date: available.then_some(manifest.pub_date).flatten(),
        url: available.then_some(url.to_string()),
    })
}

#[cfg(feature = "cli")]
pub fn check_for_cli_update(current_version: &str) -> Result<UpdateCheckReport, String> {
    let channel = channel_for_version(current_version)?;
    let platform = platform_key(std::env::consts::OS, std::env::consts::ARCH)?;
    let endpoint =
        std::env::var("PASTED_UPDATE_ENDPOINT").unwrap_or_else(|_| channel.feed_url().to_string());
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent(format!("Pasted/{current_version}"))
        .build()
        .map_err(|error| format!("Could not prepare the update check: {error}"))?
        .get(endpoint)
        .send()
        .map_err(|error| format!("Could not check for updates: {error}"))?
        .error_for_status()
        .map_err(|error| format!("The update service returned an error: {error}"))?;
    let contents = response
        .text()
        .map_err(|error| format!("Could not read the update manifest: {error}"))?;
    evaluate_manifest(current_version, &platform, &contents)
}

fn parse_version(value: &str) -> Result<Version, String> {
    Version::parse(value.trim_start_matches('v'))
        .map_err(|error| format!("Invalid update version '{value}': {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(version: &str) -> String {
        serde_json::json!({
            "version": version,
            "notes": "Safer updates.",
            "pub_date": "2026-08-27T00:00:00Z",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://github.com/getpasted/pasted/releases/download/v1.0.0/Pasted.app.tar.gz",
                    "signature": "signed"
                }
            }
        })
        .to_string()
    }

    #[test]
    fn stable_and_prerelease_versions_select_separate_channels() {
        assert_eq!(channel_for_version("1.0.0").unwrap(), UpdateChannel::Stable);
        assert_eq!(
            channel_for_version("1.0.0-rc.6").unwrap(),
            UpdateChannel::Prerelease
        );
    }

    #[test]
    fn prerelease_can_advance_to_a_stable_release() {
        let report = evaluate_manifest("1.0.0-rc.6", "darwin-aarch64", &manifest("1.0.0")).unwrap();
        assert!(report.available);
        assert_eq!(report.version.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn stable_channel_rejects_prereleases_and_downgrades() {
        assert!(
            evaluate_manifest("1.0.0", "darwin-aarch64", &manifest("1.1.0-rc.1"))
                .unwrap_err()
                .contains("stable update channel")
        );
        let report = evaluate_manifest("1.1.0", "darwin-aarch64", &manifest("1.0.0")).unwrap();
        assert!(!report.available);
        assert!(report.url.is_none());
    }

    #[test]
    fn manifests_require_complete_signed_https_platform_entries() {
        let missing = serde_json::json!({ "version": "1.0.0", "platforms": {} }).to_string();
        assert!(evaluate_manifest("1.0.0-rc.6", "darwin-aarch64", &missing).is_err());
        let insecure = manifest("1.0.0").replace("https://", "http://");
        assert!(evaluate_manifest("1.0.0-rc.6", "darwin-aarch64", &insecure)
            .unwrap_err()
            .contains("HTTPS"));
    }

    #[test]
    fn platform_keys_normalize_supported_operating_system_names() {
        assert_eq!(platform_key("macos", "arm64").unwrap(), "darwin-aarch64");
        assert_eq!(platform_key("windows", "amd64").unwrap(), "windows-x86_64");
    }
}
