//! Core message types
//! 
//! These are the internal representations of DIDComm messages,
//! independent of any FFI concerns.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core representation of a DIDComm message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub body: MessageBody,
    pub from: Option<String>,
    pub to: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub headers: HashMap<String, String>,
}

/// Message body can contain arbitrary JSON data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageBody {
    String(String),
    Object(serde_json::Value),
}

impl Message {
    /// Create a new message with basic fields
    pub fn new(id: String, msg_type: String, body: MessageBody) -> Self {
        Self {
            id,
            msg_type,
            body,
            from: None,
            to: Vec::new(),
            headers: HashMap::new(),
        }
    }
    
    /// Builder method to set 'from' field
    pub fn from(mut self, from: String) -> Self {
        self.from = Some(from);
        self
    }
    
    /// Builder method to add a recipient
    pub fn to(mut self, to: String) -> Self {
        self.to.push(to);
        self
    }
    
    /// Builder method to set all recipients
    pub fn to_all(mut self, recipients: Vec<String>) -> Self {
        self.to = recipients;
        self
    }
}

/// Encrypted message representation
#[derive(Debug, Clone)]
pub struct EncryptedMessage {
    pub ciphertext: String,
    pub protected: Option<String>,
    pub recipients: Vec<Recipient>,
}

#[derive(Debug, Clone)]
pub struct Recipient {
    pub encrypted_key: String,
    pub header: RecipientHeader,
}

#[derive(Debug, Clone)]
pub struct RecipientHeader {
    pub kid: String,
    pub epk: Option<serde_json::Value>,
    pub iv: Option<String>,
}