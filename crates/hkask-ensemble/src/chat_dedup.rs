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

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::WebID;

    #[test]
    fn chat_dedup_new_is_empty() {
        let dedup = ChatDedup::new();
        assert!(dedup.is_empty());
        assert_eq!(dedup.len(), 0);
    }

    #[test]
    fn chat_dedup_check_and_register_first_returns_true() {
        let mut dedup = ChatDedup::new();
        let msg = ChatMessage::new(WebID::new(), "hello".to_string());
        assert!(dedup.check_and_register(&msg));
    }

    #[test]
    fn chat_dedup_check_and_register_duplicate_returns_false() {
        let mut dedup = ChatDedup::new();
        let webid = WebID::new();
        let msg = ChatMessage::new(webid, "hello".to_string());
        assert!(dedup.check_and_register(&msg));
        let msg2 = ChatMessage {
            from: webid,
            content: "hello".to_string(),
            timestamp: chrono::Utc::now(),
            template_id: None,
        };
        assert!(!dedup.check_and_register(&msg2));
    }

    #[test]
    fn chat_dedup_register_allows_later_check_and_register_false() {
        let mut dedup = ChatDedup::new();
        let msg = ChatMessage::new(WebID::new(), "hello".to_string());
        dedup.register(&msg);
        let msg2 = ChatMessage {
            from: msg.from,
            content: "hello".to_string(),
            timestamp: chrono::Utc::now(),
            template_id: None,
        };
        assert!(!dedup.check_and_register(&msg2));
    }

    #[test]
    fn chat_dedup_is_duplicate_without_registering() {
        let mut dedup = ChatDedup::new();
        let msg = ChatMessage::new(WebID::new(), "hello".to_string());
        dedup.register(&msg);
        let msg2 = ChatMessage {
            from: msg.from,
            content: "hello".to_string(),
            timestamp: chrono::Utc::now(),
            template_id: None,
        };
        assert!(dedup.is_duplicate(&msg2));
        // is_duplicate should not register, so len stays the same
        assert_eq!(dedup.len(), 1);
    }

    #[test]
    fn chat_dedup_clear_resets_filter() {
        let mut dedup = ChatDedup::new();
        dedup.register(&ChatMessage::new(WebID::new(), "a".to_string()));
        dedup.register(&ChatMessage::new(WebID::new(), "b".to_string()));
        assert!(!dedup.is_empty());
        dedup.clear();
        assert!(dedup.is_empty());
    }

    #[test]
    fn message_hash_deterministic() {
        let webid = WebID::new();
        let msg = ChatMessage {
            from: webid,
            content: "hello".to_string(),
            timestamp: chrono::Utc::now(),
            template_id: None,
        };
        let h1 = message_hash(&msg);
        let h2 = message_hash(&msg);
        assert_eq!(h1, h2);
    }

    #[test]
    fn message_hash_different_content_different_hash() {
        let webid = WebID::new();
        let msg_a = ChatMessage {
            from: webid,
            content: "hello".to_string(),
            timestamp: chrono::Utc::now(),
            template_id: None,
        };
        let msg_b = ChatMessage {
            from: webid,
            content: "world".to_string(),
            timestamp: chrono::Utc::now(),
            template_id: None,
        };
        assert_ne!(message_hash(&msg_a), message_hash(&msg_b));
    }

    #[test]
    fn message_hash_ignores_timestamp() {
        let webid = WebID::new();
        let msg1 = ChatMessage {
            from: webid,
            content: "hello".to_string(),
            timestamp: chrono::DateTime::from_timestamp(1000, 0).unwrap(),
            template_id: None,
        };
        let msg2 = ChatMessage {
            from: webid,
            content: "hello".to_string(),
            timestamp: chrono::DateTime::from_timestamp(9999, 0).unwrap(),
            template_id: None,
        };
        assert_eq!(message_hash(&msg1), message_hash(&msg2));
    }

    #[test]
    fn dedup_messages_filters_duplicates_preserves_order() {
        let webid = WebID::new();
        let messages = vec![
            ChatMessage::new(webid, "first".to_string()),
            ChatMessage::new(webid, "second".to_string()),
            ChatMessage {
                from: webid,
                content: "first".to_string(),
                timestamp: chrono::Utc::now(),
                template_id: None,
            },
        ];
        let unique = dedup_messages(messages);
        assert_eq!(unique.len(), 2);
        assert_eq!(unique[0].content, "first");
        assert_eq!(unique[1].content, "second");
    }
}
