//! Private inspection cwd, independent of persistent account/configuration paths.
use std::{
    io,
    path::{Path, PathBuf},
};

pub struct CodexInspectionDirectory {
    path: PathBuf,
    temporary: bool,
}

impl CodexInspectionDirectory {
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Remove an empty temporary cwd after its child has exited. Persistent
    /// non-Windows workspaces are retained. Never recursively delete files.
    pub fn cleanup(&mut self) -> io::Result<()> {
        if self.temporary {
            match std::fs::remove_dir(&self.path) {
                Ok(()) => self.temporary = false,
                Err(error) if error.kind() == io::ErrorKind::NotFound => self.temporary = false,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

impl Drop for CodexInspectionDirectory {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

pub fn codex_inspection_directory(namespace_home: &Path) -> io::Result<CodexInspectionDirectory> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let _ = namespace_home;
        // CreateProcessW still rejects a long cwd, even with LongPathsEnabled.
        // Derive this root from the OS token, never HOME/LOCALAPPDATA overrides.
        let root = crate::current_user_profile_dir()?.join(".hagency-probes");
        crate::private_state::directory(&root).map_err(private_error)?;
        for _ in 0..8 {
            let mut random = [0u8; 16];
            getrandom::fill(&mut random)
                .map_err(|_| io::Error::other("inspection randomness unavailable"))?;
            let name: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
            let path = root.join(name);
            match crate::private_state::create_directory_new(&path) {
                Err(crate::private_state::Error::Io(e))
                    if e.kind() == io::ErrorKind::AlreadyExists =>
                {
                    continue;
                }
                result => result.map_err(private_error)?,
            }
            // Own the directory before any fallible validation so errors clean up.
            let mut directory = CodexInspectionDirectory {
                path,
                temporary: true,
            };
            directory.path = directory.path.canonicalize()?;
            // Reserve the trailing separator and NUL required by CreateProcessW.
            if directory.path.as_os_str().encode_wide().count() > 257 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "inspection cwd exceeds Windows process limit",
                ));
            }
            return Ok(directory);
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "inspection directory allocation failed",
        ))
    }
    #[cfg(not(windows))]
    {
        let path = namespace_home.join("hagency-provider-inspection");
        if !path.exists() {
            std::fs::create_dir(&path)?;
        }
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(io::Error::other("inspection workspace invalid"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
        }
        Ok(CodexInspectionDirectory {
            path: path.canonicalize()?,
            temporary: false,
        })
    }
}

#[cfg(windows)]
fn private_error(error: crate::private_state::Error) -> io::Error {
    match error {
        crate::private_state::Error::Io(error) => error,
        _ => io::Error::new(
            io::ErrorKind::PermissionDenied,
            "inspection directory is not private",
        ),
    }
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn cleanup_preserves_provider_files_and_is_idempotent() {
        let home = tempfile::tempdir().unwrap();
        let mut directory = super::codex_inspection_directory(home.path()).unwrap();
        let path = directory.path().to_owned();
        let file = path.join("provider-created");
        std::fs::write(&file, b"preserved").unwrap();
        assert!(directory.cleanup().is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"preserved");
        std::fs::remove_file(file).unwrap();
        directory.cleanup().unwrap();
        directory.cleanup().unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn long_account_path_uses_short_unique_private_cwd_and_cleans_up() {
        let temporary = tempfile::tempdir().unwrap();
        let home = temporary.path().join("a".repeat(120)).join("b".repeat(120));
        std::fs::create_dir_all(&home).unwrap();
        let marker = home.join("preserved");
        std::fs::write(&marker, b"unchanged").unwrap();
        let a = super::codex_inspection_directory(&home).unwrap();
        let b = super::codex_inspection_directory(&home).unwrap();
        assert_ne!(a.path(), b.path());
        assert!(!a.path().starts_with(&home));
        assert!(!home.starts_with(a.path()));
        crate::private_state::directory(a.path()).unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--list")
            .current_dir(a.path())
            .output()
            .unwrap();
        assert!(child.status.success());
        use std::os::windows::ffi::OsStrExt;
        assert!(a.path().as_os_str().encode_wide().count() <= 257);
        let path = a.path().to_owned();
        drop(a);
        assert!(!path.exists());
        assert_eq!(std::fs::read(marker).unwrap(), b"unchanged");
    }
}
