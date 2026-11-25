//! The SIGINT handler. It only stores a flag, which is async-signal-safe.
//!
//! The library reads that flag between reads and stops with an error, and
//! `main` turns it into exit code 130. Only Unix installs the handler;
//! Windows keeps its default Ctrl-C behaviour.

/// Exit status after SIGINT, as a shell reports it: 128 + 2.
pub const EXIT_INTERRUPTED: u8 = 130;

#[cfg(unix)]
mod imp {
    const SIGINT: i32 = 2;

    extern "C" {
        fn signal(sig: i32, handler: usize) -> usize;
    }

    extern "C" fn on_sigint(_sig: i32) {
        cur_top::request_stop();
    }

    pub fn install() {
        // SAFETY: `on_sigint` has the C signature `signal` expects and only
        // does an atomic store. A failed call leaves the default behaviour.
        unsafe {
            let _ = signal(SIGINT, on_sigint as extern "C" fn(i32) as usize);
        }
    }
}

#[cfg(not(unix))]
mod imp {
    pub fn install() {}
}

pub use imp::install;
