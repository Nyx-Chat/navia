//! Infrastructure layer - External system integrations
//!
//! This module re-exports implementations from navia-messaging to avoid code duplication.

// Re-export resolvers
pub use navia_messaging::resolvers::did::AskarDIDResolver;
pub use navia_messaging::resolvers::secrets::AskarSecretsResolver;

// Re-export storage
pub use navia_messaging::storage::AskarStorage;