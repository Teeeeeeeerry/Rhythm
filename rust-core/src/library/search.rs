//! Search utilities for the library.
//!
//! The primary search implementation uses SQLite FTS5
//! (see `Library::search()` in mod.rs).
//!
//! This module is reserved for future enhancements:
//! - Fuzzy search fallback for non-FTS queries
//! - Pinyin search for Chinese characters
//! - Search history and suggestions
