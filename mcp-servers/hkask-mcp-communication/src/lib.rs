//! hKask MCP Communication — Agent-to-agent and human-to-agent communication server.
//!
//! Exposes a full communication surface:
//! - `tts_speak`, `tts_generate`, `tts_list_voices` — local TTS/STT
//! - `send_message` — Send a message to a Matrix room
//! - `create_thread` — Create a threaded conversation
//! - `invite_agent` — Invite another replicant to a room
//! - `list_threads` — List active communication threads
//! - `monitor_thread` — Assign a thread to an agent's watchlist
//! - `tag_agent` — Pull an agent into a discussion
//!
//! Architecture:
//!   Conduit (embedded Matrix homeserver) — hosted per hKask install
//!   Iamb (embedded TUI client) — human interface
//!   Agents connect via Matrix protocol directly
//!   7R7 bot — polls Matrix for moderation → escalation pipeline

pub mod agent_registration;
pub mod matrix;
pub mod moderation;
