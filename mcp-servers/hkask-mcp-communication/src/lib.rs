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
//!   Conduit (Docker sidecar Matrix homeserver) — hosted per hKask install
//!   Agents connect via Matrix protocol directly
//!   7R7 listener — polls Matrix rooms, emits CNS observation spans
//!   Agent layer (Curator + skills + templates) — decides what content means

pub mod agent_registration;
pub mod listener;
pub mod matrix;
