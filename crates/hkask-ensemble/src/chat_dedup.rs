//! Session message deduplication — Layer 2 of the three-layer DRY system
//!
//! Implements the `entity_attribute_value_hash` deduplication strategy declared
//! in `standing-ensemble-session.yaml`. Filters duplicate messages at session
//! insertion time by computing a BLAKE3 hash of canonical message attributes.
//!
//! Three-Layer DRY System:
//! - Layer 1: Memory recall dedup (`hkask-memory/src/recall_dedup.rs`)
//! - Layer 2: Session message dedup (this module)
//! - Layer 3: Prompt assembly dedup (`hkask-templates/src/context_assembly.rs`)

use crate::chat::ChatMessage;
use hkask_types::blake3_hash;
use std::collections::HashSet;

/// Compute a canonical content hash for a chat message.
///
/// The hash covers from (WebID) + content + template_id (if present),
/// intentionally excluding timestamp so that the same message sent
/// multiple times is recognized as a duplicate.
pub fn message_hash(msg: &ChatMessage) -> [u8; 32] {
    let canonical = format!(
        "{}\x00{}\x00{}",
        msg.from,
        msg.content,
        msg.template_id.as_deref().unwrap_or("")
    );
    blake3_hash(canonical.as_bytes())
}

/// Dedup filter for chat messages.
///
/// Tracks seen message hashes and filters duplicates.
/// First-seen wins — preserves original ordering.
pub struct ChatDedup {
    seen: HashSet<[u8; 32]>,
}

impl ChatDedup {
    /// Create a new dedup filter.
    pub fn new() -> Self {
        Self {
            seen: HashSet::new(),
        }
    }

    /// Check if a message is a duplicate. Returns `true` if the message
    /// is new (not seen before), `false` if it's a duplicate.
    /// New messages are automatically registered.
    pub fn check_and_register(&mut self, msg: &ChatMessage) -> bool {
        let hash = message_hash(msg);
        self.seen.insert(hash)
    }

    /// Pre-register a message hash (e.g., when loading from storage).
    pub fn register(&mut self, msg: &ChatMessage) {
        let hash = message_hash(msg);
        self.seen.insert(hash);
    }

    /// Check if a message would be a duplicate without registering it.
    pub fn is_duplicate(&self, msg: &ChatMessage) -> bool {
        let hash = message_hash(msg);
        self.seen.contains(&hash)
    }

    /// Get the number of unique messages seen.
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// Check if no messages have been seen.
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }

    /// Clear the dedup filter.
    pub fn clear(&mut self) {
        self.seen.clear();
    }
}

impl Default for ChatDedup {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter duplicate messages from a vector.
///
/// Returns only the first occurrence of each unique message.
/// Preserves the original ordering (first-seen wins).
pub fn dedup_messages(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let mut dedup = ChatDedup::new();
    messages
        .into_iter()
        .filter(|msg| dedup.check_and_register(msg))
        .collect()
}
