//! Compare two live directory objects. This is a consistency observation, not
//! a portable identity, namespace lock, or replacement for retaining the objects.
use std::{fs::File, io};

/// A local physical observation. Serialized identity is never a directory grant.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryIdentity {
    platform: String,
    volume: String,
    object: [u8; 16],
}

pub fn directory_identity(file: &File) -> io::Result<DirectoryIdentity> {
    if !file.metadata()?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "directory required",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = file.metadata()?;
        let mut object = [0; 16];
        object[8..].copy_from_slice(&meta.ino().to_be_bytes());
        Ok(DirectoryIdentity {
            platform: "unix-v1".into(),
            volume: format!("{:016x}", meta.dev()),
            object,
        })
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ID_INFO, FileIdInfo, GetFileInformationByHandleEx,
        };
        let mut value = FILE_ID_INFO::default();
        // SAFETY: the borrowed File retains a live handle; the initialized output
        // has the exact Win32 size/alignment and is read only after success.
        if unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                FileIdInfo,
                (&mut value as *mut FILE_ID_INFO).cast(),
                std::mem::size_of::<FILE_ID_INFO>() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(DirectoryIdentity {
            platform: "windows-v1".into(),
            volume: format!("{:016x}", value.VolumeSerialNumber),
            object: value.FileId.Identifier,
        })
    }
    #[cfg(not(any(unix, windows)))]
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "directory identity unavailable",
    ))
}

/// Identity stored in the canonical owner registry across process and OS restarts.
/// Other callers still compare their live directory handles using `directory_identity`.
pub fn registry_directory_identity(file: &File) -> io::Result<DirectoryIdentity> {
    let identity = directory_identity(file)?;
    #[cfg(target_os = "macos")]
    if let Some(uuid) = volume_uuid(file)? {
        return Ok(DirectoryIdentity {
            platform: "macos-volume-uuid-v1".into(),
            volume: format!("{:032x}", u128::from_be_bytes(uuid)),
            object: identity.object,
        });
    }
    Ok(identity)
}

#[cfg(target_os = "macos")]
fn volume_uuid(file: &File) -> io::Result<Option<[u8; 16]>> {
    use std::os::fd::AsRawFd;

    let mut attrs = libc::attrlist {
        bitmapcount: libc::ATTR_BIT_MAP_COUNT,
        reserved: 0,
        commonattr: 0,
        volattr: libc::ATTR_VOL_INFO | libc::ATTR_VOL_UUID,
        dirattr: 0,
        fileattr: 0,
        forkattr: 0,
    };
    // getattrlist packs a native-endian u32 length followed by the requested uuid_t.
    let mut result = [0u8; 20];
    // SAFETY: the descriptor remains open; both buffers live through the call and
    // the kernel receives the exact writable result size.
    let status = unsafe {
        libc::fgetattrlist(
            file.as_raw_fd(),
            (&raw mut attrs).cast(),
            result.as_mut_ptr().cast(),
            result.len(),
            0,
        )
    };
    if status != 0 {
        let error = io::Error::last_os_error();
        // Some filesystems have no UUID or only expose volume attributes at
        // the mount root. Keep their previous, mount-scoped identity.
        return match error.raw_os_error() {
            Some(libc::ENOTSUP | libc::EINVAL | libc::ENOTTY) => Ok(None),
            _ => Err(error),
        };
    }
    let (length, uuid) = result.split_at(4);
    if u32::from_ne_bytes(length.try_into().unwrap()) != result.len() as u32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid volume UUID record",
        ));
    }
    let uuid: [u8; 16] = uuid.try_into().unwrap();
    Ok((uuid != [0; 16]).then_some(uuid))
}

pub fn same_directory(left: &File, right: &File) -> io::Result<bool> {
    Ok(directory_identity(left)? == directory_identity(right)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn open(path: &std::path::Path) -> File {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS;
            options.custom_flags(FILE_FLAG_BACKUP_SEMANTICS);
        }
        options.open(path).unwrap()
    }
    #[test]
    fn native_workspace_directory_identity() {
        let root = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let first = open(root.path());
        assert!(same_directory(&first, &first.try_clone().unwrap()).unwrap());
        assert!(same_directory(&first, &open(root.path())).unwrap());
        assert!(!same_directory(&first, &open(other.path())).unwrap());
        let file = File::create(root.path().join("ordinary")).unwrap();
        assert_eq!(
            same_directory(&first, &file).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn registry_uses_volume_uuid_and_directory_inode_when_available() {
        use std::os::unix::fs::MetadataExt;

        let root = tempfile::tempdir().unwrap();
        let directory = open(root.path());
        let actual = registry_directory_identity(&directory).unwrap();
        if let Some(uuid) = volume_uuid(&directory).unwrap() {
            assert_eq!(actual.platform, "macos-volume-uuid-v1");
            assert_eq!(actual.volume, format!("{:032x}", u128::from_be_bytes(uuid)));
            assert_eq!(
                actual.object,
                u128::from(directory.metadata().unwrap().ino()).to_be_bytes()
            );
        } else {
            assert_eq!(actual, directory_identity(&directory).unwrap());
        }
        assert_eq!(
            actual,
            registry_directory_identity(&open(root.path())).unwrap()
        );
        let child = root.path().join("child");
        std::fs::create_dir(&child).unwrap();
        let other = registry_directory_identity(&open(&child)).unwrap();
        assert_eq!(actual.volume, other.volume);
        assert_ne!(actual.object, other.object);
    }
}
