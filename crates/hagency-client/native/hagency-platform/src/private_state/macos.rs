//! Repair mode drift only on the fixed parents of canonical owner roots.
//! A directory that other users could write may already contain substituted
//! entries. Changing its mode cannot establish the integrity of those entries.
use super::Error;
use rustix::fs::{AtFlags, Mode, OFlags, fchmod, fstat, mkdirat, open, openat, statat};
use std::{
    ffi::{OsStr, c_int, c_void},
    io,
    os::fd::{AsRawFd, OwnedFd},
    path::Path,
};

const DIRECTORY: OFlags = OFlags::RDONLY
    .union(OFlags::DIRECTORY)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

// Darwin's sys/acl.h. An extended allow entry can grant access independently
// of the POSIX mode. The standard user home may carry a deny-delete entry.
unsafe extern "C" {
    fn acl_get_fd(fd: c_int) -> *mut c_void;
    fn acl_get_entry(acl: *mut c_void, index: c_int, entry: *mut *mut c_void) -> c_int;
    fn acl_get_tag_type(entry: *mut c_void, tag: *mut c_int) -> c_int;
    fn acl_free(acl: *mut c_void) -> c_int;
}

const ACL_EXTENDED_DENY: c_int = 2;

fn check_acl(fd: &OwnedFd) -> Result<(), Error> {
    // SAFETY: fd remains open throughout. Entries belong to the returned ACL,
    // which is freed exactly once after iteration. No pointers escape.
    unsafe {
        let acl = acl_get_fd(fd.as_raw_fd());
        if acl.is_null() {
            let err = io::Error::last_os_error();
            return if err.raw_os_error() == Some(libc::ENOENT) {
                Ok(())
            } else {
                Err(err.into())
            };
        }
        let mut index = 0; // ACL_FIRST_ENTRY
        let result = loop {
            let mut entry = std::ptr::null_mut();
            if acl_get_entry(acl, index, &mut entry) != 0 {
                // Darwin reports EINVAL when the last entry has been visited.
                let err = io::Error::last_os_error();
                break if err.raw_os_error() == Some(libc::EINVAL) {
                    Ok(())
                } else {
                    Err(err.into())
                };
            }
            let mut tag = 0;
            if acl_get_tag_type(entry, &mut tag) != 0 || tag != ACL_EXTENDED_DENY {
                break Err(Error::Private);
            }
            index = -1; // ACL_NEXT_ENTRY
        };
        acl_free(acl);
        result
    }
}

fn checked_mode(fd: &OwnedFd) -> Result<u32, Error> {
    let stat = fstat(fd).map_err(io::Error::from)?;
    let mode = u32::from(stat.st_mode) & 0o7777;
    if stat.st_uid != rustix::process::geteuid().as_raw() || mode & 0o7022 != 0 {
        return Err(Error::Private);
    }
    check_acl(fd)?;
    Ok(mode)
}

fn private_child(parent: &OwnedFd, name: &OsStr) -> Result<OwnedFd, Error> {
    let private = Mode::RWXU;
    match mkdirat(parent, name, private) {
        Ok(()) | Err(rustix::io::Errno::EXIST) => {}
        Err(err) => return Err(io::Error::from(err).into()),
    }
    let child = openat(parent, name, DIRECTORY, Mode::empty()).map_err(io::Error::from)?;
    let mode = checked_mode(&child)?;
    if mode & 0o077 != 0 {
        if mode & 0o700 != 0o700 {
            return Err(Error::Private);
        }
        fchmod(&child, private).map_err(io::Error::from)?;
        if checked_mode(&child)? != 0o700 {
            return Err(Error::Private);
        }
    }
    let held = fstat(&child).map_err(io::Error::from)?;
    let named = statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
    if (held.st_dev, held.st_ino) != (named.st_dev, named.st_ino) {
        return Err(Error::Private);
    }
    Ok(child)
}

/// Prepare the registry base and optionally its fixed `owners` child.
/// The caller supplies the base derived from the OS account's home, or an
/// isolated test installation. No account directory or file is traversed.
pub fn prepare_owner_parents(base: &Path, include_owners: bool) -> Result<(), Error> {
    let parent = base.parent().ok_or(Error::Private)?;
    let name = base.file_name().ok_or(Error::Private)?;
    let home = open(parent, DIRECTORY, Mode::empty()).map_err(io::Error::from)?;
    checked_mode(&home)?;
    let registry = private_child(&home, name)?;
    if include_owners {
        private_child(&registry, OsStr::new("owners"))?;
    }
    Ok(())
}
