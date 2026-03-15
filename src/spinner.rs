use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const INTERVAL: Duration = Duration::from_millis(80);
const POLL_STEP: Duration = Duration::from_millis(10);

pub struct Spinner {
    running: Arc<AtomicBool>,
    message: Arc<Mutex<String>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    pub fn start(msg: &str) -> Option<Self> {
        if !should_show_spinner() {
            return None;
        }

        let running = Arc::new(AtomicBool::new(true));
        let message = Arc::new(Mutex::new(msg.to_string()));

        let r = Arc::clone(&running);
        let m = Arc::clone(&message);

        let handle = thread::spawn(move || {
            let mut frame = 0;
            let mut stderr = io::stderr();
            while r.load(Ordering::Acquire) {
                let msg = m.lock().unwrap_or_else(|p| p.into_inner()).clone();
                let _ = write!(stderr, "\x1b[2K\r{} {}", FRAMES[frame], msg);
                let _ = stderr.flush();
                frame = (frame + 1) % FRAMES.len();

                // poll in short steps so stop() doesn't block the full 80ms
                for _ in 0..(INTERVAL.as_millis() / POLL_STEP.as_millis()) {
                    if !r.load(Ordering::Acquire) {
                        break;
                    }
                    thread::sleep(POLL_STEP);
                }
            }
            let _ = write!(stderr, "\x1b[2K\r");
            let _ = stderr.flush();
        });

        Some(Spinner {
            running,
            message,
            handle: Some(handle),
        })
    }

    pub fn set_message(&self, msg: &str) {
        if let Ok(mut guard) = self.message.lock() {
            *guard = msg.to_string();
        }
    }

    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            if handle.join().is_err() {
                let _ = write!(io::stderr(), "\x1b[2K\r");
                let _ = io::stderr().flush();
            }
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn should_show_spinner() -> bool {
    if !is_stderr_tty() {
        return false;
    }

    // respect NO_COLOR convention (https://no-color.org)
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }

    // dumb terminals don't support ANSI escapes
    if std::env::var("TERM").ok().as_deref() == Some("dumb") {
        return false;
    }

    // most CI systems set this; spinner is noise in log output
    if std::env::var_os("CI").is_some() {
        return false;
    }

    true
}

// inline libc isatty check to avoid adding a dependency
#[cfg(unix)]
extern "C" {
    fn isatty(fd: std::os::raw::c_int) -> std::os::raw::c_int;
}

#[cfg(unix)]
fn is_stderr_tty() -> bool {
    // SAFETY: isatty is a read only query on a valid fd, no UB possible
    unsafe { isatty(2) != 0 }
}

#[cfg(windows)]
fn is_stderr_tty() -> bool {
    extern "C" {
        fn _isatty(fd: std::os::raw::c_int) -> std::os::raw::c_int;
    }
    // SAFETY: _isatty is a read only query on a valid fd
    unsafe { _isatty(2) != 0 }
}

#[cfg(not(any(unix, windows)))]
fn is_stderr_tty() -> bool {
    false
}
