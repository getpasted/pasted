#[cfg(any(target_os = "macos", target_os = "linux"))]
use base64::Engine;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::path::PathBuf;
use std::sync::Arc;

use tauri::{AppHandle, State};

use crate::db::DbState;

#[cfg(target_os = "macos")]
fn macos_application_icon_data_url(application_name: &str) -> Option<String> {
    fn application_bundle_path(name: &str) -> Option<PathBuf> {
        let mut roots = vec![
            PathBuf::from("/Applications"),
            PathBuf::from("/System/Applications"),
            PathBuf::from("/System/Applications/Utilities"),
            PathBuf::from("/System/Library/CoreServices"),
        ];
        if let Some(home) = dirs::home_dir() {
            roots.insert(0, home.join("Applications"));
        }
        for root in &roots {
            let direct = root.join(format!("{name}.app"));
            if direct.is_dir() {
                return Some(direct);
            }
        }
        for root in roots {
            let Ok(entries) = std::fs::read_dir(root) else {
                continue;
            };
            for entry in entries.filter_map(Result::ok).take(512) {
                let path = entry.path();
                if path.extension().is_some_and(|extension| extension == "app")
                    && path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .is_some_and(|stem| stem.eq_ignore_ascii_case(name))
                {
                    return Some(path);
                }
            }
        }
        None
    }

    fn bundle_icon_path(bundle: &std::path::Path) -> Option<PathBuf> {
        let resources = bundle.join("Contents/Resources");
        let info = plist::Value::from_file(bundle.join("Contents/Info.plist")).ok()?;
        if let Some(icon_name) = info
            .as_dictionary()
            .and_then(|dictionary| dictionary.get("CFBundleIconFile"))
            .and_then(plist::Value::as_string)
        {
            let mut icon = resources.join(icon_name);
            if icon.extension().is_none() {
                icon.set_extension("icns");
            }
            if icon.is_file() {
                return Some(icon);
            }
        }
        std::fs::read_dir(resources)
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "icns")
            })
    }

    let candidates = match application_name {
        "VS Code" => vec!["Visual Studio Code", "VS Code"],
        "System Clipboard" | "Unknown" | "Unknown Source" => Vec::new(),
        name => vec![name],
    };
    static TEMP_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    for icon_path in candidates
        .into_iter()
        .filter_map(application_bundle_path)
        .filter_map(|bundle| bundle_icon_path(&bundle))
    {
        let sequence = TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let output_path = std::env::temp_dir().join(format!(
            "pasted-source-icon-{}-{sequence}.png",
            std::process::id(),
        ));
        let status = std::process::Command::new("/usr/bin/sips")
            .args(["-s", "format", "png", "--resampleWidth", "32"])
            .arg(&icon_path)
            .arg("--out")
            .arg(&output_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .ok();
        let png = status
            .filter(std::process::ExitStatus::success)
            .and_then(|_| std::fs::read(&output_path).ok());
        let _ = std::fs::remove_file(&output_path);
        if let Some(png) = png.filter(|bytes| !bytes.is_empty() && bytes.len() <= 512 * 1024) {
            return Some(format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(png),
            ));
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn linux_application_icon_data_url(application_name: &str) -> Option<String> {
    use gtk::gdk_pixbuf::Pixbuf;

    fn desktop_files(root: &std::path::Path) -> Vec<PathBuf> {
        std::fs::read_dir(root)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "desktop")
            })
            .take(2048)
            .collect()
    }

    fn find_icon_file(icon: &str) -> Option<PathBuf> {
        let direct = PathBuf::from(icon);
        if direct.is_absolute() && direct.is_file() {
            return Some(direct);
        }
        static ICON_FILES: once_cell::sync::Lazy<std::collections::HashMap<String, PathBuf>> =
            once_cell::sync::Lazy::new(|| {
                let mut roots = vec![
                    PathBuf::from("/usr/share/icons"),
                    PathBuf::from("/usr/share/pixmaps"),
                ];
                if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
                    roots.insert(0, PathBuf::from(data_home).join("icons"));
                } else if let Some(home) = dirs::home_dir() {
                    roots.insert(0, home.join(".local/share/icons"));
                }
                let mut files = std::collections::HashMap::new();
                let mut pending = roots;
                let mut visited = 0usize;
                while let Some(directory) = pending.pop() {
                    let Ok(entries) = std::fs::read_dir(directory) else {
                        continue;
                    };
                    for entry in entries.filter_map(Result::ok) {
                        visited += 1;
                        if visited > 50_000 {
                            return files;
                        }
                        let path = entry.path();
                        if path.is_dir() {
                            pending.push(path);
                        } else if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                            if matches!(
                                path.extension().and_then(|extension| extension.to_str()),
                                Some("png" | "svg" | "xpm")
                            ) {
                                files.entry(name.to_string()).or_insert(path);
                            }
                        }
                    }
                }
                files
            });
        [
            format!("{icon}.png"),
            format!("{icon}.svg"),
            format!("{icon}.xpm"),
        ]
        .iter()
        .find_map(|candidate| ICON_FILES.get(candidate).cloned())
    }

    let mut roots = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        roots.insert(0, PathBuf::from(data_home).join("applications"));
    } else if let Some(home) = dirs::home_dir() {
        roots.insert(0, home.join(".local/share/applications"));
    }
    let source = application_name.trim().to_lowercase();
    if source.is_empty()
        || matches!(
            source.as_str(),
            "system clipboard" | "unknown" | "unknown source"
        )
    {
        return None;
    }
    let mut partial_match = None;
    for path in roots.iter().flat_map(|root| desktop_files(root)) {
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > 256 * 1024 {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(path) else {
            continue;
        };
        let name = contents.lines().find_map(|line| line.strip_prefix("Name="));
        let icon = contents.lines().find_map(|line| line.strip_prefix("Icon="));
        let (Some(name), Some(icon)) = (name, icon) else {
            continue;
        };
        let normalized = name.trim().to_lowercase();
        if normalized == source {
            partial_match = Some(icon.to_string());
            break;
        }
        if partial_match.is_none() && (source.contains(&normalized) || normalized.contains(&source))
        {
            partial_match = Some(icon.to_string());
        }
    }
    let icon_path = find_icon_file(partial_match?.trim())?;
    let pixbuf = Pixbuf::from_file_at_scale(icon_path, 32, 32, true).ok()?;
    let png = pixbuf.save_to_bufferv("png", &[]).ok()?;
    (png.len() <= 512 * 1024).then(|| {
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png)
        )
    })
}

#[cfg(target_os = "windows")]
fn windows_application_icons(
    sources: &[String],
) -> Result<std::collections::HashMap<String, String>, String> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const SCRIPT: &str = r#"
$ErrorActionPreference = 'SilentlyContinue'
Add-Type -AssemblyName System.Drawing
$names = [Console]::In.ReadToEnd() | ConvertFrom-Json
$roots = @(
  "$env:APPDATA\Microsoft\Windows\Start Menu\Programs",
  "$env:ProgramData\Microsoft\Windows\Start Menu\Programs"
)
$links = @(Get-ChildItem -Path $roots -Filter '*.lnk' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 4096)
$shell = New-Object -ComObject WScript.Shell
$result = @{}
foreach ($name in $names) {
  $link = $links | Where-Object { $_.BaseName -ieq $name } | Select-Object -First 1
  if ($null -eq $link) {
    $link = $links | Where-Object { $name -like ('*' + $_.BaseName + '*') -or $_.BaseName -like ('*' + $name + '*') } | Select-Object -First 1
  }
  if ($null -eq $link) { continue }
  $target = $shell.CreateShortcut($link.FullName).TargetPath
  if ([string]::IsNullOrWhiteSpace($target) -or -not (Test-Path -LiteralPath $target)) { continue }
  $icon = [System.Drawing.Icon]::ExtractAssociatedIcon($target)
  if ($null -eq $icon) { continue }
  $bitmap = New-Object System.Drawing.Bitmap($icon.ToBitmap(), 32, 32)
  $stream = New-Object System.IO.MemoryStream
  $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
  $result[$name] = 'data:image/png;base64,' + [Convert]::ToBase64String($stream.ToArray())
  $stream.Dispose(); $bitmap.Dispose(); $icon.Dispose()
}
$result | ConvertTo-Json -Compress
"#;
    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", SCRIPT])
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Could not start the Windows icon resolver: {error}"))?;
    let input = serde_json::to_vec(sources).map_err(|error| error.to_string())?;
    child
        .stdin
        .take()
        .ok_or("Windows icon resolver input was unavailable.")?
        .write_all(&input)
        .map_err(|error| error.to_string())?;
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() || output.stdout.len() > 8 * 1024 * 1024 {
        return Ok(std::collections::HashMap::new());
    }
    serde_json::from_slice(&output.stdout).or_else(|_| Ok(std::collections::HashMap::new()))
}

static SOURCE_ICON_CACHE: once_cell::sync::Lazy<
    parking_lot::Mutex<std::collections::HashMap<String, String>>,
> = once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(std::collections::HashMap::new()));

fn cache_resolved_source_icons(
    mut existing: std::collections::HashMap<String, String>,
    resolved: std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, String> {
    let mut cache = SOURCE_ICON_CACHE.lock();
    for (name, icon) in resolved {
        if crate::resource_limits::validate_raster_data_url(&icon).is_err() {
            continue;
        }
        cache.insert(name.clone(), icon.clone());
        existing.insert(name, icon);
    }
    existing
}

#[tauri::command]
pub async fn get_source_icons(
    sources: Vec<String>,
    app: AppHandle,
) -> Result<std::collections::HashMap<String, String>, String> {
    if sources.len() > 128 || sources.iter().any(|name| name.len() > 256) {
        return Err("Source icon request exceeds the supported limit.".to_string());
    }
    let (cached_icons, uncached_sources) = {
        let cache = SOURCE_ICON_CACHE.lock();
        let cached = sources
            .iter()
            .filter_map(|name| cache.get(name).cloned().map(|icon| (name.clone(), icon)))
            .collect::<std::collections::HashMap<_, _>>();
        let uncached = sources
            .into_iter()
            .filter(|name| !cache.contains_key(name))
            .collect::<Vec<_>>();
        (cached, uncached)
    };
    if uncached_sources.is_empty() {
        return Ok(cached_icons);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = app;
        let resolved = tauri::async_runtime::spawn_blocking(move || {
            uncached_sources
                .into_iter()
                .filter_map(|name| macos_application_icon_data_url(&name).map(|icon| (name, icon)))
                .collect::<std::collections::HashMap<_, _>>()
        })
        .await
        .map_err(|error| error.to_string())?;
        Ok(cache_resolved_source_icons(cached_icons, resolved))
    }

    #[cfg(target_os = "linux")]
    {
        let _ = app;
        let resolved = tauri::async_runtime::spawn_blocking(move || {
            uncached_sources
                .into_iter()
                .filter_map(|name| linux_application_icon_data_url(&name).map(|icon| (name, icon)))
                .collect()
        })
        .await
        .map_err(|error| error.to_string())?;
        Ok(cache_resolved_source_icons(cached_icons, resolved))
    }

    #[cfg(target_os = "windows")]
    {
        let _ = app;
        let resolved = tauri::async_runtime::spawn_blocking(move || {
            windows_application_icons(&uncached_sources)
        })
        .await
        .map_err(|error| error.to_string())??;
        Ok(cache_resolved_source_icons(cached_icons, resolved))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = (uncached_sources, app);
        Ok(cached_icons)
    }
}

#[tauri::command]
pub fn get_installed_applications(db: State<'_, Arc<DbState>>) -> Result<Vec<String>, String> {
    let mut apps = std::collections::BTreeSet::new();

    if let Ok(history_apps) = db.get_distinct_sources() {
        for app in history_apps {
            if !app.trim().is_empty() {
                apps.insert(app);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let dirs = [
            "/Applications",
            "/System/Applications",
            "/System/Applications/Utilities",
        ];
        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "app") {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            apps.insert(name.to_string());
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        let dirs = ["/usr/share/applications", "/usr/local/share/applications"];
        for dir in &dirs {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "desktop") {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            let clean_name = name.trim_end_matches(".desktop");
                            apps.insert(clean_name.to_string());
                        }
                    }
                }
            }
        }
    }

    let common = [
        "1Password",
        "Bitwarden",
        "Safari",
        "Google Chrome",
        "Firefox",
        "Slack",
        "Signal",
        "Telegram",
        "VS Code",
        "Terminal",
        "Warp",
        "Xcode",
        "Discord",
        "Keychain Access",
        "Passwords",
    ];
    for c in &common {
        apps.insert(c.to_string());
    }

    Ok(apps.into_iter().collect())
}
