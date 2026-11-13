//! Test-only helpers shared across the crate's unit tests.

use std::sync::{Mutex, MutexGuard};

// Serializes every test that changes the process language (i18n::set_current and CHAUDE_LANG). Tests run in
// parallel threads of one binary, so an unguarded switch can leak into another test's output.
static LANG_LOCK: Mutex<()> = Mutex::new(());

// Holds the language lock for the caller's whole test. A poisoned lock (an earlier test panicked) is still usable.
pub fn lang_lock() -> MutexGuard<'static, ()> {
    LANG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}
