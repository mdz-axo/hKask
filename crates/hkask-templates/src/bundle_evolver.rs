//! Bundle evolution orchestration
//!
//! Handles the process of detecting skill changes, preparing evolution context,
//! and determining which bundles need re-composition when skills evolve.

use hkask_types::{BundleDependencyIndex, BundleManifest, BundleSkillChange, Skill, SkillPolarity};

/// Orchestrates bundle evolution when skills change.
///
/// The evolver:
/// 1. Loads the existing bundle manifest
/// 2. Compares content hashes of referenced skills with their current hashes
/// 3. Identifies which skills have changed
/// 4. Prepares the evolution context (evolved_skills, unchanged_skills)
/// 5. Determines the version bump type based on the nature of changes
#[derive(Debug, Clone)]
pub struct BundleEvolver {
    /// Dependency index for tracking which bundles depend on which skills
    dependency_index: BundleDependencyIndex,
}

impl BundleEvolver {
    /// Create a new evolver with an empty dependency index.
    pub fn new() -> Self {
        Self {
            dependency_index: BundleDependencyIndex::new(),
        }
    }

    /// Register a bundle in the dependency index.
    pub fn register_bundle(&mut self, bundle: &BundleManifest) {
        self.dependency_index.register_bundle(bundle);
    }

    /// Remove a bundle from the dependency index.
    pub fn remove_bundle(&mut self, bundle: &BundleManifest) {
        self.dependency_index.remove_bundle(bundle);
    }

    /// Find which bundles need evolution given a set of skill changes.
    pub fn bundles_needing_evolution(&self, changes: &[BundleSkillChange]) -> Vec<String> {
        self.dependency_index.bundles_needing_evolution(changes)
    }

    /// Compare skill content hashes against a bundle manifest to find changes.
    ///
    /// Returns a list of `BundleSkillChange` for skills whose current hash
    /// differs from what the manifest records.
    pub fn detect_skill_changes(
        &self,
        bundle: &BundleManifest,
        current_skills: &[Skill],
    ) -> Vec<BundleSkillChange> {
        let mut changes = Vec::new();

        for bundle_skill in &bundle.skills {
            // Find the current version of this skill
            if let Some(current) = current_skills.iter().find(|s| s.id == bundle_skill.id) {
                let current_hash = current.content_hash.clone().unwrap_or_default();
                let stored_hash = bundle_skill.content_hash.clone();

                if current_hash != stored_hash && !current_hash.is_empty() {
                    // Check if polarity changed
                    let polarity_changed = current
                        .polarity
                        .map(|p| p != bundle_skill.polarity)
                        .unwrap_or(false);

                    changes.push(BundleSkillChange {
                        skill_id: bundle_skill.id.clone(),
                        previous_hash: stored_hash,
                        current_hash,
                        polarity_changed,
                    });
                }
            }
            // If skill not found in current_skills, it was removed —
            // that's a structural change handled separately
        }

        changes
    }

    /// Determine the version bump type based on the nature of changes.
    ///
    /// - **Major**: Phase reordering, step add/remove, polarity changes
    /// - **Minor**: Skill add/remove, conflict/complementarity changes
    /// - **Patch**: Instruction-only changes (hash differs but structure unchanged)
    pub fn determine_version_bump(
        &self,
        _bundle: &BundleManifest,
        changes: &[BundleSkillChange],
    ) -> hkask_types::VersionBump {
        let has_polarity_change = changes.iter().any(|c| c.polarity_changed);

        if has_polarity_change {
            // Polarity change may require phase reordering — major bump
            hkask_types::VersionBump::Major
        } else if !changes.is_empty() {
            // Content hash changed but structure intact — minor bump
            hkask_types::VersionBump::Minor
        } else {
            // No changes detected — patch bump (shouldn't normally happen)
            hkask_types::VersionBump::Patch
        }
    }

    /// Prepare the evolution context for the evolve-bundle template.
    ///
    /// Returns a map of template variables needed by `evolve-bundle.j2`:
    /// - `bundle_manifest`: the existing manifest
    /// - `evolved_skills`: skills that have changed
    /// - `unchanged_skills`: skills that haven't changed
    pub fn prepare_evolution_context(
        &self,
        bundle: &BundleManifest,
        current_skills: &[Skill],
    ) -> EvolutionContext {
        let changes = self.detect_skill_changes(bundle, current_skills);
        let changed_ids: std::collections::HashSet<&str> =
            changes.iter().map(|c| c.skill_id.as_str()).collect();

        let mut evolved_skills = Vec::new();
        let mut unchanged_skills = Vec::new();

        for bundle_skill in &bundle.skills {
            if changed_ids.contains(bundle_skill.id.as_str()) {
                // Find the current skill data
                if let Some(current) = current_skills.iter().find(|s| s.id == bundle_skill.id) {
                    evolved_skills.push(EvolvedSkillInfo {
                        id: current.id.clone(),
                        name: current.id.clone(), // Use ID as name if no separate name field
                        polarity: current.polarity.unwrap_or(bundle_skill.polarity),
                        lexicon_terms: current.cascade_order.clone(), // Simplified
                        previous_hash: bundle_skill.content_hash.clone(),
                        current_hash: current.content_hash.clone().unwrap_or_default(),
                        change_summary: if current
                            .polarity
                            .map(|p| p != bundle_skill.polarity)
                            .unwrap_or(false)
                        {
                            "Polarity changed — full re-analysis required".to_string()
                        } else {
                            "Content hash differs — re-analysis recommended".to_string()
                        },
                    });
                }
            } else {
                unchanged_skills.push(UnchangedSkillInfo {
                    id: bundle_skill.id.clone(),
                    name: bundle_skill.id.clone(),
                    polarity: bundle_skill.polarity,
                    content_hash: bundle_skill.content_hash.clone(),
                });
            }
        }

        let version_bump = self.determine_version_bump(bundle, &changes);

        EvolutionContext {
            bundle_id: bundle.id.clone(),
            evolved_skills,
            unchanged_skills,
            changes,
            version_bump,
        }
    }
}

impl Default for BundleEvolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Skill info for the evolution template context (evolved skills).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvolvedSkillInfo {
    pub id: String,
    pub name: String,
    pub polarity: SkillPolarity,
    pub lexicon_terms: Vec<String>,
    pub previous_hash: String,
    pub current_hash: String,
    pub change_summary: String,
}

/// Skill info for the evolution template context (unchanged skills).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UnchangedSkillInfo {
    pub id: String,
    pub name: String,
    pub polarity: SkillPolarity,
    pub content_hash: String,
}

/// The full context needed by the evolve-bundle template.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvolutionContext {
    /// The bundle ID being evolved
    pub bundle_id: String,
    /// Skills that have changed
    pub evolved_skills: Vec<EvolvedSkillInfo>,
    /// Skills that haven't changed
    pub unchanged_skills: Vec<UnchangedSkillInfo>,
    /// The raw change records
    pub changes: Vec<BundleSkillChange>,
    /// The recommended version bump
    pub version_bump: hkask_types::VersionBump,
}

#[cfg(test)]
mod tests {
    use super::*;
    use hkask_types::{BundleSkill, Visibility};

    fn make_skill(id: &str, hash: &str) -> Skill {
        Skill::new(id, hkask_types::TemplateType::FlowDef).with_content_hash(hash.to_string())
    }

    fn make_minimal_bundle(skills: Vec<BundleSkill>) -> BundleManifest {
        hkask_types::BundleManifest {
            id: "test-bundle".to_string(),
            name: "Test Bundle".to_string(),
            description: "A test bundle".to_string(),
            version: "1.0.0".to_string(),
            editor: "curator-or-human-admin".to_string(),
            visibility: Visibility::Private,
            skills,
            conflicts: vec![],
            complementarities: vec![],
            steps: vec![],
            convergence: Default::default(),
            gas: Default::default(),
            error_handling: Default::default(),
            ocap: Default::default(),
            cns: Default::default(),
            audit: Default::default(),
        }
    }

    #[test]
    fn evolver_detects_hash_changes() {
        let mut evolver = BundleEvolver::new();

        let bundle_skill_a = BundleSkill {
            id: "skill-a".to_string(),
            polarity: SkillPolarity::Generative,
            lexicon_terms: vec!["create".to_string()],
            manifest_ref: "manifest-a".to_string(),
            content_hash: "abc123".to_string(),
        };
        let bundle = make_minimal_bundle(vec![bundle_skill_a]);
        evolver.register_bundle(&bundle);

        let current_skill_a = make_skill("skill-a", "def456");
        let changes = evolver.detect_skill_changes(&bundle, &[current_skill_a]);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].previous_hash, "abc123");
        assert_eq!(changes[0].current_hash, "def456");
    }

    #[test]
    fn evolver_no_changes_when_hashes_match() {
        let mut evolver = BundleEvolver::new();
        let bundle_skill = BundleSkill {
            id: "skill-a".to_string(),
            polarity: SkillPolarity::Generative,
            lexicon_terms: vec!["create".to_string()],
            manifest_ref: "manifest-a".to_string(),
            content_hash: "abc123".to_string(),
        };
        let bundle = make_minimal_bundle(vec![bundle_skill]);
        evolver.register_bundle(&bundle);

        let current_skill = make_skill("skill-a", "abc123");
        let changes = evolver.detect_skill_changes(&bundle, &[current_skill]);
        assert!(changes.is_empty());
    }

    #[test]
    fn version_bump_major_for_polarity_change() {
        let evolver = BundleEvolver::new();
        let bundle_skill = BundleSkill {
            id: "skill-a".to_string(),
            polarity: SkillPolarity::Generative,
            lexicon_terms: vec![],
            manifest_ref: "manifest-a".to_string(),
            content_hash: "abc".to_string(),
        };
        let bundle = make_minimal_bundle(vec![bundle_skill]);

        let changes = vec![BundleSkillChange {
            skill_id: "skill-a".to_string(),
            previous_hash: "abc".to_string(),
            current_hash: "def".to_string(),
            polarity_changed: true,
        }];

        let bump = evolver.determine_version_bump(&bundle, &changes);
        assert_eq!(bump, hkask_types::VersionBump::Major);
    }

    #[test]
    fn version_bump_minor_for_content_change() {
        let evolver = BundleEvolver::new();
        let bundle_skill = BundleSkill {
            id: "skill-a".to_string(),
            polarity: SkillPolarity::Generative,
            lexicon_terms: vec![],
            manifest_ref: "manifest-a".to_string(),
            content_hash: "abc".to_string(),
        };
        let bundle = make_minimal_bundle(vec![bundle_skill]);

        let changes = vec![BundleSkillChange {
            skill_id: "skill-a".to_string(),
            previous_hash: "abc".to_string(),
            current_hash: "def".to_string(),
            polarity_changed: false,
        }];

        let bump = evolver.determine_version_bump(&bundle, &changes);
        assert_eq!(bump, hkask_types::VersionBump::Minor);
    }

    #[test]
    fn dependency_index_tracks_skills_to_bundles() {
        let mut evolver = BundleEvolver::new();
        let skill = BundleSkill {
            id: "coding-guidelines".to_string(),
            polarity: SkillPolarity::Regulative,
            lexicon_terms: vec![],
            manifest_ref: "coding-guidelines".to_string(),
            content_hash: "hash1".to_string(),
        };
        let bundle = make_minimal_bundle(vec![skill]);
        evolver.register_bundle(&bundle);

        let deps = evolver.bundles_needing_evolution(&[BundleSkillChange {
            skill_id: "coding-guidelines".to_string(),
            previous_hash: "hash1".to_string(),
            current_hash: "hash2".to_string(),
            polarity_changed: false,
        }]);
        assert!(deps.contains(&"test-bundle".to_string()));
    }
}
