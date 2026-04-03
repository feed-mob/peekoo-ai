//! Shared utility functions for the Peekoo AI application
//!
//! This crate provides cross-platform utilities used across the peekoo workspace.

pub mod process;

// Re-export process utilities at crate root for convenience
pub use process::{command_available, resolve_command};
