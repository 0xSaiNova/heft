use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const INTERVAL: Duration = Duration::from_millis(80);

pub struct Spinner {
    running: Arc<AtomicBool>,
    message: Arc<Mutex<String>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    /// Start a spinner on stderr. Returns None if stderr is not a terminal.
    pub fn start(msg: &str) -> Option<Self> {
        if !atty_stderr() {
            return None;
        }

        let running = Arc::new(AtomicBool::new(true));
        let message = Arc::new(Mutex::new(msg.to_string()));

        let r = Arc::clone(&running);
        let m = Arc::clone(&message);

        let handle = thread::spawn(move || {
            let mut frame = 0;
            let mut stderr = io::stderr();
            while r.load(Ordering::Relaxed) {
                let msg = m.lock().unwrap().clone();
                let _ = write!(stderr, "\r\x1b[2K{} {}", FRAMES[frame], msg);
                let _ = stderr.flush();
                frame = (frame + 1) % FRAMES.len();
                thread::sleep(INTERVAL);
            }
            // Clear the spinner line
            let _ = write!(stderr, "\r\x1b[2K");
            let _ = stderr.flush();
        });

        Some(Spinner {
            running,
            message,
            handle: Some(handle),
        })
    }

    /// Update the spinner message.
    pub fn set_message(&self, msg: &str) {
        *self.message.lock().unwrap() = msg.to_string();
    }

    /// Stop the spinner and clear the line.
    pub fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn atty_stderr() -> bool {
    unsafe { libc_isatty(2) != 0 }
}

// Inline libc isatty check to avoid adding a dependency.
#[cfg(unix)]
extern "C" {
    fn isatty(fd: std::os::raw::c_int) -> std::os::raw::c_int;
}

#[cfg(unix)]
unsafe fn libc_isatty(fd: std::os::raw::c_int) -> std::os::raw::c_int {
    unsafe { isatty(fd) }
}

#[cfg(windows)]
unsafe fn libc_isatty(fd: std::os::raw::c_int) -> std::os::raw::c_int {
    // On Windows, _isatty is in the CRT
    extern "C" {
        fn _isatty(fd: std::os::raw::c_int) -> std::os::raw::c_int;
    }
    unsafe { _isatty(fd) }
}
