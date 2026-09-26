use parking_lot::Mutex;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

/// Shared across GUI and CLI lifetimes; exclusive during a location change.
pub struct LibrarySession {
    pub app_data: PathBuf,
    lease: Mutex<(File, File)>,
}

fn lock_file(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(|error| error.to_string())
}

struct RestoreLeaseOnUnwind<'a> {
    file: &'a File,
    gate: &'a File,
    armed: bool,
}

impl Drop for RestoreLeaseOnUnwind<'_> {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.file.unlock();
            let _ = self.file.lock_shared();
            let _ = self.gate.unlock();
        }
    }
}

impl LibrarySession {
    pub fn open(app_data: &Path) -> Result<Self, String> {
        let session = Self::open_unsettled(app_data)?;
        if session.app_data.join("library-move.json").exists() {
            session.exclusive(|| Ok(()))?;
        }
        Ok(session)
    }

    pub(super) fn open_unsettled(app_data: &Path) -> Result<Self, String> {
        fs::create_dir_all(app_data).map_err(|error| error.to_string())?;
        let app_data = fs::canonicalize(app_data).map_err(|error| error.to_string())?;
        let gate = lock_file(&app_data.join(".library-transition.lock"))?;
        gate.try_lock_shared()
            .map_err(|_| "The library is changing location".to_string())?;
        let file = lock_file(&app_data.join(".library-session.lock"))?;
        file.try_lock_shared()
            .map_err(|_| "The library is busy in another Pasted process".to_string())?;
        gate.unlock().map_err(|error| error.to_string())?;
        let session = Self {
            app_data,
            lease: Mutex::new((file, gate)),
        };
        Ok(session)
    }

    pub fn database_path(&self) -> Result<PathBuf, String> {
        super::resolve_database_path(&self.app_data)
    }

    /// Serialize work that depends on the current library path with in-process
    /// moves and restores while retaining the process's shared cross-process lease.
    pub fn stable<T>(&self, operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        let _files = self.lease.lock();
        operation()
    }

    pub fn exclusive<T>(&self, operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        self.exclusive_unsettled(|| {
            if self.app_data.join("library-move.json").exists() {
                super::transfer::reconcile(&self.app_data)?;
            }
            operation()
        })
    }

    pub(super) fn exclusive_unsettled<T>(
        &self,
        operation: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let files = self.lease.lock();
        let (file, gate) = &*files;
        // Serialize upgrades and session arrivals. A failed upgrade must restore
        // its shared lease before another process can change the active path.
        gate.try_lock()
            .map_err(|_| "The library is changing location".to_string())?;
        let mut restoration = RestoreLeaseOnUnwind {
            file,
            gate,
            armed: true,
        };
        let result = (|| {
            file.unlock().map_err(|error| error.to_string())?;
            let acquired = file.try_lock().is_ok();
            let result = if acquired {
                operation()
            } else {
                Err("Close other Pasted app or CLI sessions before moving or recovering the library".into())
            };
            if acquired {
                file.unlock().map_err(|error| error.to_string())?;
            }
            file.lock_shared().map_err(|error| error.to_string())?;
            result
        })();
        gate.unlock().map_err(|error| error.to_string())?;
        restoration.armed = false;
        result
    }
}
