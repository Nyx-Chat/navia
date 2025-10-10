//! Re-export AskarDB from navia_messaging
//!
//! This module simply re-exports the AskarDB implementation from navia-messaging
//! to avoid code duplication. All database operations are handled by navia-messaging.

// Re-export the AskarDB struct and related types
pub use navia_messaging::askardb::AskarDB;
