// pub mod askardb;
// pub mod error;
// pub mod messaging;
// mod resolvers;
//
// pub use didcomm::did::DIDDoc;
// pub use didcomm::secrets::Secret;
// pub use didcomm::Message;
// pub use didcomm::PackEncryptedOptions;
// pub use didcomm::UnpackOptions;

mod askardb;
mod error;
mod messaging;
mod resolvers;

#[cfg(target_os = "android")]
mod jni_bridge;

pub use messaging::{DidComInterface, DidCommError};

// use didcomm::did::DIDDoc;
// use didcomm::secrets::Secret;
// use didcomm::Message;
// use didcomm::PackEncryptedOptions;
// use didcomm::UnpackOptions;

uniffi::setup_scaffolding!();
