//! Starts the renderer: the agent-desktop binary itself, detached, with an
//! allowlisted environment and no inherited descriptors beyond null stdio.
use std::os::fd::BorrowedFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use agent_desktop_core::INTERACTION_LEASE_FD_ENV;
use rustix::io::{FdFlags, fcntl_getfd, fcntl_setfd};

use super::child::{CHILD_ARGUMENT, CHILD_MARKER, DISPLAY_ENV, GENERATION};

/// The name of the binary the renderer runs as.
pub(crate) const IMAGE_NAME: &str = "agent-desktop";
const INHERITED: [&str; 7] = [
    "DISPLAY",
    "XAUTHORITY",
    "XDG_RUNTIME_DIR",
    "AGENT_DESKTOP_HOME",
    "HOME",
    "PATH",
    "LANG",
];
const LOWEST_INHERITED_FD: i32 = 3;

/// Whether `path` is the agent-desktop binary. Under an FFI host the running
/// image is the host's, which must never be started again as a renderer,
/// and a test binary must not start one either.
pub(crate) fn is_renderer_image(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some(IMAGE_NAME)
}

/// The running binary when it can start the renderer.
pub(crate) fn renderer_image() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .filter(|path| is_renderer_image(path))
}

/// The command that starts the renderer for `display`.
pub(crate) fn build_command(
    image: &Path,
    display: &str,
    parent_env: &dyn Fn(&str) -> Option<String>,
) -> Command {
    let mut command = Command::new(image);
    command
        .arg(CHILD_ARGUMENT)
        .env_clear()
        .envs(
            INHERITED
                .into_iter()
                .filter_map(|name| parent_env(name).map(|value| (name, value))),
        )
        .env("DISPLAY", display)
        .env(CHILD_MARKER, GENERATION)
        .env(DISPLAY_ENV, display)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(lease) = parent_env(INTERACTION_LEASE_FD_ENV)
        .and_then(|value| value.parse::<i32>().ok())
        .filter(|number| *number >= LOWEST_INHERITED_FD)
    {
        unsafe {
            command.pre_exec(move || withhold(lease));
        }
    }
    command
}

/// Keeps the interaction lease out of the renderer, which outlives the
/// command holding it. Runs in the forked child only.
fn withhold(descriptor: i32) -> std::io::Result<()> {
    let descriptor = unsafe { BorrowedFd::borrow_raw(descriptor) };
    match fcntl_getfd(descriptor) {
        Ok(flags) => {
            fcntl_setfd(descriptor, flags | FdFlags::CLOEXEC).map_err(std::io::Error::from)
        }
        Err(_) => Ok(()),
    }
}

/// Starts the renderer and collects its exit status in the background, so a
/// long-lived caller keeps no zombie.
pub(crate) fn start(image: &Path, display: &str) -> std::io::Result<()> {
    let mut child = build_command(image, display, &|name| std::env::var(name).ok()).spawn()?;
    std::thread::Builder::new()
        .name("cursor-renderer-reaper".to_owned())
        .spawn(move || {
            let _ = child.wait();
        })
        .map(|_| ())
}

#[cfg(test)]
#[path = "spawn_tests.rs"]
mod tests;
