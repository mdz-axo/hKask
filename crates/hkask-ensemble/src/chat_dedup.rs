//! Session message deduplication
//!
//! Detects and filters repetitive messages in multi-agent chat sessions.
//! Uses BLAKE3 content hashing for exact dedup and sliding window
//! condensation for long sessions.
//!
//! This is Layer 2 of the three-layer DRY system:
//! - Layer 1: Memory recall dedup (hkask-memory/src/recall_dedup.rs)
//! - Layer 2: Session message dedup (this module)
//! - Layer 3: Prompt assembly dedup (hkask-templates/src/context_assembly.rs)
//!
//! # Usage
//!
//! ```ignore
//! use hkask_ensemble::chat_dedup::SessionDedup;
//!
//! let mut dedup = SessionDedup::new(1000); // keep last 1000 unique messages
//!
//! for message in chat_history {
//!     if dedup.accept(&message.content) {
//!         // Message is novel, include in context
//!     }
//! }
//!
//! let stats = dedup.stats();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

/// Session-level deduplication with sliding window.
///
/// Tracks message content hashes within a sliding window and rejects
/// exact duplicates. When the window is full, oldest messages are evicted.
pub struct SessionDedup {
    /// Content hashes of messages in the current window
    seen_hashes: HashSet<[u8; 32]>,
    /// Ordered queue of hashes for FIFO eviction
    hash_queue: VecDeque<[u8; 32]>,
    /// Maximum number of unique messages to track
    max_window: usize,
    /// Statistics
    stats: DedupStats,
}

/// Statistics for session deduplication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupStats {
    pub messages_offered: usize,
    pub messages_accepted: usize,
    pub duplicates_rejected: usize,
    pub evictions: usize,
}

impl SessionDedup {
    /// Create a new session dedup with the given window size.
    ///
    /// The window size determines how many unique messages are tracked
    /// before the oldest are evicted (FIFO).
    pub fn new(max_window: usize) -> Self {
        Self {
            seen_hashes: HashSet::with_capacity(max_window),
            hash_queue: VecDeque::with_capacity(max_window),
            max_window,
            stats: DedupStats {
                messages_offered: 0,
                messages_accepted: 0,
                duplicates_rejected: 0,
                evictions: 0,
            },
        }
    }

    /// Check if a message is novel (not a duplicate within the window).
    ///
    /// Returns `true` if the message should be included, `false` if it's
    /// a duplicate that should be filtered.
    pub fn accept(&mut self, content: &str) -> bool {
        self.stats.messages_offered += 1;

        let hash = content_hash(content);

        if self.seen_hashes.contains(&hash) {
            self.stats.duplicates_rejected += 1;
            return false;
        }

        // Evict oldest if window is full
        if self.hash_queue.len() >= self.max_window
            && let Some(old_hash) = self.hash_queue.pop_front()
        {
            self.seen_hashes.remove(&old_hash);
            self.stats.evictions += 1;
        }

        self.seen_hashes.insert(hash);
        self.hash_queue.push_back(hash);
        self.stats.messages_accepted += 1;
        true
    }

    /// Filter a slice of messages, returning only novel ones.
    ///
    /// Preserves ordering (first-seen wins for duplicates).
    pub fn filter_messages<'a, I>(&mut self, messages: I) -> Vec<&'a str>
    where
        I: IntoIterator<Item = &'a str>,
    {
        messages
            .into_iter()
            .filter(|content| self.accept(content))
            .collect()
    }

    /// Get deduplication statistics.
    pub fn stats(&self) -> &DedupStats {
        &self.stats
    }

    /// Get the number of unique messages currently tracked.
    pub fn window_size(&self) -> usize {
        self.seen_hashes.len()
    }

    /// Reset the dedup state and statistics.
    pub fn reset(&mut self) {
        self.seen_hashes.clear();
        self.hash_queue.clear();
        self.stats = DedupStats {
            messages_offered: 0,
            messages_accepted: 0,
            duplicates_rejected: 0,
            evictions: 0,
        };
    }
}

/// Compute BLAKE3 hash of message content.
fn content_hash(content: &str) -> [u8; 32] {
    hkask_types::blake3_hash(content.as_bytes())
}

/// Extract a deduplicated context window from chat history.
///
/// Takes a slice of messages (most recent last) and returns a deduplicated
/// subset that fits within the token budget, prioritizing recent messages.
///
/// **Side-effect:** This function mutates the `dedup` state by calling `accept()`
/// on each message. Messages consumed by this function will be marked as "seen"
/// and rejected as duplicates if the same `SessionDedup` is used for additional
/// filtering afterwards.
///
/// If you need to extract context without mutating the dedup state, use
/// `extract_context_window_pure()` instead.
///
/// This is the primary entry point for building context windows from
/// session history before sending to Okapi for inference.
pub fn extract_context_window(
    messages: &[String],
    max_tokens: usize,
    dedup: &mut SessionDedup,
) -> Vec<String> {
    // Process in reverse order (most recent first) to prioritize recent messages
    let mut accepted = Vec::new();
    let mut tokens_used = 0;

    for message in messages.iter().rev() {
        if !dedup.accept(message) {
            continue; // Skip duplicate
        }

        let msg_tokens = estimate_tokens(message);
        if tokens_used + msg_tokens > max_tokens {
            break; // Budget exceeded
        }

        tokens_used += msg_tokens;
        accepted.push(message.clone());
    }

    // Reverse to restore chronological order
    accepted.reverse();
    accepted
}

/// Extract a context window from chat history without mutating dedup state.
///
/// This is a pure function that uses a local HashSet for deduplication,
/// leaving the caller's `SessionDedup` state unchanged.
///
/// Use this when you need to extract context multiple times or when the
/// dedup state should be managed separately.
pub fn extract_context_window_pure(messages: &[String], max_tokens: usize) -> Vec<String> {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    let mut accepted = Vec::new();
    let mut tokens_used = 0;

    // Process in reverse order (most recent first) to prioritize recent messages
    for message in messages.iter().rev() {
        let hash = content_hash(message);
        if !seen.insert(hash) {
            continue; // Skip duplicate
        }

        let msg_tokens = estimate_tokens(message);
        if tokens_used + msg_tokens > max_tokens {
            break; // Budget exceeded
        }

        tokens_used += msg_tokens;
        accepted.push(message.clone());
    }

    // Reverse to restore chronological order
    accepted.reverse();
    accepted
}

/// Estimate token count for a string (~4 chars per token).
fn estimate_tokens(text: &str) -> usize {
    hkask_types::estimate_tokens(text)
}
