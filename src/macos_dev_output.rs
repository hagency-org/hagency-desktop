//! Keep a stalled development launcher's FIFOs off synchronous library and UI writes.
use std::{
    fs::File,
    io::{self, Read, Write},
    os::{
        fd::{AsFd, AsRawFd},
        unix::{fs::FileTypeExt, net::UnixStream},
    },
    thread,
};

/// Called before starting the native UI. Only the Cargo macOS bundle runner sets
/// this variable. Its stdout and stderr FIFOs are private to this launch.
pub fn install() {
    if std::env::var_os("MAKEPAD_DEV_LAUNCH_DIR").is_some() {
        let _ = isolate_fifo(io::stdout());
        let _ = isolate_fifo(io::stderr());
    }
}

fn isolate_fifo(stream: impl AsFd) -> io::Result<()> {
    let inherited = stream.as_fd();
    let mut destination = File::from(inherited.try_clone_to_owned()?);
    if !destination.metadata()?.file_type().is_fifo() {
        return Ok(());
    }
    let (mut reader, writer) = UnixStream::pair()?;
    thread::Builder::new()
        .name("dev-output".into())
        .spawn(move || {
            let mut buf = [0; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(len) => {
                        // A full or disconnected launcher drops these bytes. Keep
                        // reading even after EPIPE so synchronous writers stay usable.
                        let _ = destination.write_all(&buf[..len]);
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        })?;
    let fd = inherited.as_raw_fd();
    // SAFETY: the inherited descriptor is still open. The relay's duplicate
    // shares its open file description and therefore its O_NONBLOCK flag.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags == -1 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: both descriptors are live. Atomically replace the standard
    // stream before starting the UI or its other threads.
    if unsafe { libc::dup2(writer.as_raw_fd(), fd) } == -1 {
        let error = io::Error::last_os_error();
        unsafe { libc::fcntl(fd, libc::F_SETFL, flags) };
        return Err(error);
    }
    Ok(())
}
