//! hKask Skill Service — skill discovery, publishing, and hashing.
//!
//! Extracted from `hkask-services`.
mod skill_impl;
pub use skill_impl::{
    SkillInfo, SkillPublishResult, compute_file_hash, discover_skills, find_public_skill,
    publish_skill, read_skill_namespace, read_skill_visibility, resolve_replicant_name,
};
