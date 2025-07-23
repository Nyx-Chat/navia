//! Global constants for the navia library

/// Maximum message size in bytes (1MB)
pub const MAX_MESSAGE_SIZE: usize = 1_048_576;

/// Minimum seed length in bytes for database encryption
pub const MIN_SEED_LENGTH: usize = 32;

/// Maximum seed length in bytes
pub const MAX_SEED_LENGTH: usize = 1024;

/// Maximum DID string length
pub const MAX_DID_LENGTH: usize = 2048;

/// Maximum URI string length  
pub const MAX_URI_LENGTH: usize = 2048;

/// Maximum storage key length
pub const MAX_KEY_LENGTH: usize = 256;

/// Maximum storage category name length
pub const MAX_CATEGORY_LENGTH: usize = 64;
