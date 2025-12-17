// Library interface for testing
// This allows integration tests to access internal modules
//
// Only modules used by tests are exposed as public.
// Internal-only modules remain private.

pub mod commands;
pub mod config;
pub mod constants;
pub mod error;
pub mod misc;

// Private modules (not used by tests, only by binary)
#[allow(dead_code)]
mod context;
#[allow(dead_code)]
mod prompt;
#[allow(dead_code)]
mod ui;
