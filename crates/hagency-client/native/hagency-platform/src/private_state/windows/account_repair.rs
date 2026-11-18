//! Handle-based recovery of individual account objects after verified sign-in.
use super::*;
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_FLAG_OPEN_REPARSE_POINT, FILE_RENAME_INFO, FILE_SHARE_DELETE, FileRenameInfo,
    SetFileInformationByHandle, WRITE_DAC,
};

fn open(path: &Path, access: u32, sharing: u32) -> Result<File, Error> {
    Ok(OpenOptions::new()
        .access_mode(access)
        .share_mode(sharing)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?)
}

fn plain(file: &File, directory: bool) -> Result<(), Error> {
    let meta = file.metadata()?;
    if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
        || if directory {
            !meta.is_dir()
        } else {
            !meta.is_file()
        }
    {
        return Err(Error::Private);
    }
    if !directory {
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        // SAFETY: the handle and output buffer live throughout this call.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if info.nNumberOfLinks != 1 {
            return Err(Error::Private);
        }
    }
    Ok(())
}

fn restrict(file: &File, directory: bool) -> Result<(), Error> {
    let inheritance = if directory { "OICI" } else { "" };
    let sddl: Vec<_> = format!("D:P(A;{inheritance};FA;;;{})\0", current_sid()?)
        .encode_utf16()
        .collect();
    // Set only this handle's DACL. SetSecurityInfo can propagate to existing
    // children, which have not yet been checked for ownership or links.
    unsafe {
        let mut descriptor = ptr::null_mut();
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            ptr::null_mut(),
        ) == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        let _allocation = Allocation(descriptor);
        if SetKernelObjectSecurity(
            file.as_raw_handle(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        ) == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    check_handle(file, false)
}

/// Keeps each ancestor from being renamed while its children are inspected.
/// Ordinary profile directories above the account are never resealed.
pub struct AccountRepairPath {
    path: std::path::PathBuf,
    handles: Vec<File>,
    repaired_parent: bool,
}

impl AccountRepairPath {
    pub fn at(root: &Path) -> Result<Self, Error> {
        if !root.is_absolute()
            || root
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(Error::Private);
        }
        let mut handles = Vec::new();
        let parents: Vec<_> = root.ancestors().collect();
        for ancestor in parents.into_iter().rev() {
            let handle = open(
                ancestor,
                windows_sys::Win32::Storage::FileSystem::FILE_READ_ATTRIBUTES,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
            )?;
            plain(&handle, true)?;
            handles.push(handle);
        }
        Ok(Self {
            path: root.into(),
            handles,
            repaired_parent: false,
        })
    }

    fn child(&self, name: &std::ffi::OsStr) -> Result<std::path::PathBuf, Error> {
        if Path::new(name).file_name() != Some(name) || name.to_string_lossy().contains(':') {
            return Err(Error::Private);
        }
        Ok(self.path.join(name))
    }

    /// Returns false for absent directories. The normal save path creates them.
    pub fn enter(&mut self, name: &std::ffi::OsStr) -> Result<bool, Error> {
        let path = self.child(name)?;
        let file = match open(&path, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE) {
            Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            result => result?,
        };
        plain(&file, true)?;
        match check_handle(&file, false) {
            Ok(()) => (),
            Err(Error::Private) => {
                let writable = open(&path, GENERIC_READ | WRITE_DAC, FILE_SHARE_READ)?;
                plain(&writable, true)?;
                check_current_owner(&writable)?;
                restrict(&writable, true)?;
                self.repaired_parent = true;
            }
            Err(error) => return Err(error),
        }
        self.path = path;
        self.handles.push(file);
        Ok(true)
    }

    /// Read-only custody for authenticated pre-authorization metadata. No ACL changes.
    pub fn open_owned_file(&self, name: &str) -> Result<File, Error> {
        let file = open(&self.child(name.as_ref())?, GENERIC_READ, FILE_SHARE_READ)?;
        plain(&file, false)?;
        check_current_owner(&file)?;
        Ok(file)
    }

    pub fn needs_file_repair(&self, name: &str) -> Result<bool, Error> {
        self.inspect_file(name, false)
    }

    pub fn needs_document_repair(&self, name: &str) -> Result<bool, Error> {
        self.inspect_file(name, self.repaired_parent)
    }

    fn inspect_file(&self, name: &str, verify_private: bool) -> Result<bool, Error> {
        let path = self.child(name.as_ref())?;
        let probe = match open(
            &path,
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
        ) {
            Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            result => result?,
        };
        plain(&probe, false)?;
        match check_handle(&probe, false) {
            Ok(()) => Ok(verify_private),
            Err(Error::Private) => Ok(true),
            Err(error) => Err(error),
        }
    }

    /// Finalize a protected staging file on its original handle, without replacement.
    pub fn rename_private_file(&self, name: &str, destination: &str) -> Result<(), Error> {
        let file = open(&self.child(name.as_ref())?, GENERIC_READ | DELETE, 0)?;
        plain(&file, false)?;
        check_handle(&file, false)?;
        rename_original(&file, &self.child(destination.as_ref())?)
    }

    /// Copy into an owner-only file before removing the original. A stopped copy
    /// leaves the original in place with its original trust level. Only a flushed,
    /// complete copy can acquire the pending archive name.
    fn archive_file(&self, source: &mut File, destination: &str) -> Result<(), Error> {
        use std::io::Seek;
        use windows_sys::Win32::Storage::FileSystem::{FILE_DISPOSITION_INFO, FileDispositionInfo};
        let destination_path = self.child(destination.as_ref())?;
        let reusable = match open(
            &destination_path,
            GENERIC_READ | GENERIC_WRITE,
            FILE_SHARE_READ,
        ) {
            Ok(mut pending) => {
                plain(&pending, false)?;
                check_handle(&pending, false)?;
                if same_contents(source, &mut pending)? {
                    // Keep the verified bytes pinned through sync and removal of the original.
                    pending.sync_all()?;
                    Some(pending)
                } else {
                    drop(pending);
                    // An independent source now occupies the live name. Keep both originals.
                    let mut suffix = [0u8; 16];
                    getrandom::fill(&mut suffix).map_err(|_| Error::PlatformUnavailable)?;
                    self.rename_private_file(
                        destination,
                        &format!("{destination}-{}", super::super::hex(&suffix)),
                    )?;
                    None
                }
            }
            Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        if reusable.is_none() {
            let copying = format!("{destination}.copying");
            let path = self.child(copying.as_ref())?;
            let mut output = match create_file(&path) {
                Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    open(&path, GENERIC_READ | GENERIC_WRITE, 0)?
                }
                result => result?,
            };
            plain(&output, false)?;
            check_handle(&output, false)?;
            output.set_len(0)?;
            source.rewind()?;
            std::io::copy(source, &mut output)?;
            output.sync_all()?;
            drop(output);
            self.rename_private_file(&copying, destination)?;
        }
        let directory =
            cap_std::fs::Dir::open_ambient_dir(&self.path, cap_std::ambient_authority())?;
        crate::private_state::sync_directory(&directory)?;
        let disposition = FILE_DISPOSITION_INFO {
            DeleteFile: true.into(),
        };
        // SAFETY: source is still the exclusively held, verified original with
        // DELETE access. The kernel removes this file when that handle closes.
        if unsafe {
            SetFileInformationByHandle(
                source.as_raw_handle(),
                FileDispositionInfo,
                (&disposition as *const FILE_DISPOSITION_INFO).cast(),
                size_of_val(&disposition) as u32,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(())
    }

    /// Run the caller's validation while writers and renames are excluded.
    pub fn repair_file(
        &self,
        name: &str,
        archive: Option<&str>,
        validate: impl FnOnce(&mut File) -> Result<bool, Error>,
    ) -> Result<bool, Error> {
        let path = self.child(name.as_ref())?;
        let verify_private = archive.is_some() && self.repaired_parent;
        if !self.inspect_file(name, verify_private)? {
            return Ok(false);
        }
        let mut file = open(&path, GENERIC_READ | WRITE_DAC | DELETE, 0)?;
        plain(&file, false)?;
        check_current_owner(&file)?;
        let already_private = match check_handle(&file, false) {
            Ok(()) if !verify_private => return Ok(false),
            Ok(()) => true,
            Err(Error::Private) => false,
            Err(error) => return Err(error),
        };
        let valid = validate(&mut file)?;
        if !valid {
            self.archive_file(&mut file, archive.ok_or(Error::Private)?)?;
        } else if !already_private {
            restrict(&file, false)?;
        }
        Ok(!valid)
    }
}

/// Both handles exclude writers and renames during this comparison.
fn same_contents(source: &mut File, pending: &mut File) -> Result<bool, Error> {
    use std::io::{Read, Seek};
    let mut remaining = source.metadata()?.len();
    if pending.metadata()?.len() != remaining {
        return Ok(false);
    }
    source.rewind()?;
    let (mut left, mut right) = ([0u8; 8192], [0u8; 8192]);
    while remaining > 0 {
        let n = remaining.min(left.len() as u64) as usize;
        source.read_exact(&mut left[..n])?;
        pending.read_exact(&mut right[..n])?;
        if left[..n] != right[..n] {
            return Ok(false);
        }
        remaining -= n as u64;
    }
    Ok(true)
}

fn rename_original(file: &File, destination: &Path) -> Result<(), Error> {
    let name = wide(destination)?;
    let name = &name[..name.len() - 1];
    let bytes = size_of_val(name);
    let size = size_of::<FILE_RENAME_INFO>() + bytes;
    let mut storage = vec![0usize; size.div_ceil(size_of::<usize>())];
    // SAFETY: aligned, zero-initialized storage includes the flexible UTF-16 tail.
    // RootDirectory is NULL for the absolute destination. Never replace a name.
    unsafe {
        let info = storage.as_mut_ptr().cast::<FILE_RENAME_INFO>();
        (*info).FileNameLength = bytes.try_into().map_err(|_| Error::Private)?;
        ptr::copy_nonoverlapping(
            name.as_ptr(),
            ptr::addr_of_mut!((*info).FileName).cast(),
            name.len(),
        );
        if SetFileInformationByHandle(
            file.as_raw_handle(),
            FileRenameInfo,
            info.cast(),
            size as u32,
        ) == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    Ok(())
}
