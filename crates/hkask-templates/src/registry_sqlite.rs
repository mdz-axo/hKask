//! SQLite registry adapter
//!
//! Persistent template registry backed by SQLite.
//! Supports fast lookups, full-text search, and audit trail.

use crate::ports::{RegistryEntry, RegistryIndex, Result, TemplateError};
use crate::provenance::{ProvenanceManager, TemplateProvenance};
use hkask_types::ports::SkillRegistryIndex;
use hkask_types::{Skill, TemplateType};
use rusqlite::{Connection, params};

/// Raw skill row tuple: (id, domain, word_act, flow_def, know_act)
type SkillRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Parsed row tuple from the templates table:
/// (id, template_type, name, description, source_path, cascade_level, matroshka_limit)
type TemplateRow = (String, TemplateType, String, String, String, u32, u32);

fn parse_template_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TemplateRow> {
    let id: String = row.get(0)?;
    let template_type_str: String = row.get(1)?;
    let name: String = row.get(2)?;
    let description: String = row.get(3)?;
    let source_path: String = row.get(4)?;
    let cascade_level: u32 = row.get(5)?;
    let matroshka_limit: u32 = row.get(6)?;
    let template_type = TemplateType::parse_str(&template_type_str).ok_or_else(|| {
        rusqlite::Error::ToSqlConversionFailure(
            format!("Unknown template type: {}", template_type_str).into(),
        )
    })?;
    Ok((
        id,
        template_type,
        name,
        description,
        source_path,
        cascade_level,
        matroshka_limit,
    ))
}

/// SQLite-based registry index
pub struct SqliteRegistry {
    conn: Connection,
    provenance: ProvenanceManager,
}

impl SqliteRegistry {
    /// Create new SQLite registry (in-memory or file-backed)
    pub fn new(path: Option<&str>) -> Result<Self> {
        let conn = match path {
            Some(p) => Connection::open(p)
                .map_err(|e| TemplateError::Manifest(format!("Failed to open SQLite: {}", e)))?,
            None => {
                tracing::warn!(
                    target: "hkask.templates",
                    "No database path provided — template registry is in-memory and will be lost on restart. \
                     Pass a file path for sovereign persistence."
                );
                Connection::open_in_memory().map_err(|e| {
                    TemplateError::Manifest(format!("Failed to create in-memory SQLite: {}", e))
                })?
            }
        };

        let mut registry = Self {
            conn,
            provenance: ProvenanceManager::new(),
        };

        // Initialize schema
        registry.init_schema()?;

        Ok(registry)
    }

    /// Initialize database schema
    fn init_schema(&mut self) -> Result<()> {
        self.conn
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS templates (
                id TEXT PRIMARY KEY,
                template_type TEXT NOT NULL,
                name TEXT NOT NULL DEFAULT '',
                description TEXT,
                source_path TEXT NOT NULL,
                cascade_level INTEGER NOT NULL DEFAULT 0,
                matroshka_limit INTEGER NOT NULL DEFAULT 7,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS lexicon_terms (
                template_id TEXT NOT NULL,
                term TEXT NOT NULL,
                PRIMARY KEY (template_id, term),
                FOREIGN KEY (template_id) REFERENCES templates(id)
            );

            CREATE TABLE IF NOT EXISTS template_capabilities (
                template_id TEXT NOT NULL,
                capability TEXT NOT NULL,
                PRIMARY KEY (template_id, capability),
                FOREIGN KEY (template_id) REFERENCES templates(id)
            );

            CREATE TABLE IF NOT EXISTS provenance (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                template_id TEXT NOT NULL,
                git_sha TEXT NOT NULL,
                modified_by TEXT NOT NULL,
                modified_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                branch TEXT,
                commit_message TEXT,
                FOREIGN KEY (template_id) REFERENCES templates(id)
            );

            CREATE INDEX IF NOT EXISTS idx_templates_type ON templates(template_type);
            CREATE INDEX IF NOT EXISTS idx_lexicon_terms ON lexicon_terms(term);
            CREATE INDEX IF NOT EXISTS idx_provenance_template ON provenance(template_id);
            CREATE INDEX IF NOT EXISTS idx_template_capabilities ON template_capabilities(capability);

            CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                domain TEXT NOT NULL,
                word_act TEXT,
                flow_def TEXT,
                know_act TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS skill_cascade_order (
                skill_id TEXT NOT NULL,
                position INTEGER NOT NULL,
                template_id TEXT NOT NULL,
                PRIMARY KEY (skill_id, position),
                FOREIGN KEY (skill_id) REFERENCES skills(id)
            );

            CREATE INDEX IF NOT EXISTS idx_skills_domain ON skills(domain);
            CREATE INDEX IF NOT EXISTS idx_skill_cascade ON skill_cascade_order(skill_id);
            ",
            )
            .map_err(|e| TemplateError::Manifest(format!("Failed to init schema: {}", e)))?;

        Ok(())
    }

    /// Register a template in the registry
    ///
    /// Validates the entry (cascade_level, matroshka_limit) and logs warnings
    /// for any issues before persisting.
    pub fn register(
        &mut self,
        entry: RegistryEntry,
        provenance: Option<TemplateProvenance>,
    ) -> Result<()> {
        // Validate entry consistency and log warnings
        let warnings = entry.validate();
        for warning in &warnings {
            tracing::warn!(target: "hkask.templates", "Registration warning: {}", warning);
        }

        let tx = self
            .conn
            .transaction()
            .map_err(|e| TemplateError::Manifest(format!("Failed to start transaction: {}", e)))?;

        // Insert template
        tx.execute(
            "INSERT OR REPLACE INTO templates (id, template_type, name, description, source_path, cascade_level, matroshka_limit, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP)",
            params![
                entry.id,
                entry.template_type.as_str(),
                entry.name,
                entry.description,
                entry.source_path,
                entry.cascade_level,
                entry.matroshka_limit,
            ],
        ).map_err(|e| TemplateError::Manifest(format!("Failed to insert template: {}", e)))?;

        // Delete existing lexicon terms
        tx.execute(
            "DELETE FROM lexicon_terms WHERE template_id = ?1",
            params![entry.id],
        )
        .map_err(|e| TemplateError::Manifest(format!("Failed to delete lexicon: {}", e)))?;

        // Insert lexicon terms
        for term in &entry.lexicon_terms {
            tx.execute(
                "INSERT INTO lexicon_terms (template_id, term) VALUES (?1, ?2)",
                params![entry.id, term],
            )
            .map_err(|e| TemplateError::Manifest(format!("Failed to insert lexicon: {}", e)))?;
        }

        // Delete existing capabilities
        tx.execute(
            "DELETE FROM template_capabilities WHERE template_id = ?1",
            params![entry.id],
        )
        .map_err(|e| TemplateError::Manifest(format!("Failed to delete capabilities: {}", e)))?;

        // Insert capabilities
        for cap in &entry.required_capabilities {
            tx.execute(
                "INSERT INTO template_capabilities (template_id, capability) VALUES (?1, ?2)",
                params![entry.id, cap],
            )
            .map_err(|e| TemplateError::Manifest(format!("Failed to insert capability: {}", e)))?;
        }

        tx.commit()
            .map_err(|e| TemplateError::Manifest(format!("Failed to commit: {}", e)))?;

        // Record provenance
        if let Some(p) = provenance {
            self.provenance.record(p);
        }

        Ok(())
    }

    /// Read a single template row into a RegistryEntry
    #[allow(clippy::too_many_arguments)]
    fn row_to_entry(
        &self,
        id: &str,
        template_type: TemplateType,
        name: String,
        description: String,
        source_path: String,
        cascade_level: u32,
        matroshka_limit: u32,
    ) -> Result<RegistryEntry> {
        let lexicon_terms: Vec<String> = self
            .conn
            .prepare("SELECT term FROM lexicon_terms WHERE template_id = ?1")
            .map_err(|e| {
                TemplateError::Database(format!("Failed to prepare lexicon query: {}", e))
            })?
            .query_map(params![id], |row| row.get(0))
            .map_err(|e| TemplateError::Database(format!("Failed to query lexicon: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        let required_capabilities: Vec<String> = self
            .conn
            .prepare("SELECT capability FROM template_capabilities WHERE template_id = ?1")
            .map_err(|e| {
                TemplateError::Database(format!("Failed to prepare capabilities query: {}", e))
            })?
            .query_map(params![id], |row| row.get(0))
            .map_err(|e| TemplateError::Database(format!("Failed to query capabilities: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(RegistryEntry {
            id: id.to_string(),
            template_type,
            name,
            lexicon_terms,
            description,
            source_path,
            required_capabilities,
            cascade_level,
            matroshka_limit,
        })
    }

    /// Get a single template by ID directly from the database
    pub fn get_entry(&self, id: &str) -> Result<RegistryEntry> {
        let row = self
            .conn
            .prepare("SELECT id, template_type, name, description, source_path, cascade_level, matroshka_limit FROM templates WHERE id = ?1")
            .map_err(|e| TemplateError::Database(format!("Failed to prepare query: {}", e)))?
            .query_row(params![id], parse_template_row)
            .map_err(|e| TemplateError::NotFound(format!("Template '{}' not found: {}", id, e)))?;

        self.row_to_entry(&row.0, row.1, row.2, row.3, row.4, row.5, row.6)
    }

    /// Search templates by lexicon term
    pub fn search_by_lexicon(&self, term: &str) -> Result<Vec<RegistryEntry>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT t.id, t.template_type, t.name, t.description, t.source_path, t.cascade_level, t.matroshka_limit
             FROM templates t
             JOIN lexicon_terms l ON t.id = l.template_id
             WHERE l.term = ?1",
            )
            .map_err(|e| TemplateError::Database(format!("Failed to prepare statement: {}", e)))?;

        let rows: Vec<TemplateRow> = stmt
            .query_map(params![term], parse_template_row)
            .map_err(|e| TemplateError::Database(format!("Failed to query templates: {}", e)))?
            .filter_map(|r| r.ok())
            .collect();

        let mut results = Vec::new();
        for (id, template_type, name, description, source_path, cascade_level, matroshka_limit) in
            rows
        {
            let entry = self.row_to_entry(
                &id,
                template_type,
                name,
                description,
                source_path,
                cascade_level,
                matroshka_limit,
            )?;
            results.push(entry);
        }

        Ok(results)
    }

    /// Get provenance for a template
    pub fn get_provenance(&self, template_id: &str) -> Option<&TemplateProvenance> {
        self.provenance.get_latest(template_id)
    }

    /// Get template count
    pub fn count(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM templates", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap_or(0) as usize
    }
}

impl RegistryIndex for SqliteRegistry {
    fn list(&self, domain_hint: Option<TemplateType>) -> Vec<RegistryEntry> {
        let sql = match domain_hint {
            Some(_) => {
                "SELECT id, template_type, name, description, source_path, cascade_level, matroshka_limit FROM templates WHERE template_type = ?1"
            }
            None => {
                "SELECT id, template_type, name, description, source_path, cascade_level, matroshka_limit FROM templates"
            }
        };

        let mut stmt = match self.conn.prepare(sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows: Vec<TemplateRow> = match domain_hint {
            Some(tt) => stmt
                .query_map(params![tt.as_str()], parse_template_row)
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
                .unwrap_or_default(),
            None => stmt
                .query_map([], parse_template_row)
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
                .unwrap_or_default(),
        };

        rows.into_iter()
            .filter_map(
                |(
                    id,
                    template_type,
                    name,
                    description,
                    source_path,
                    cascade_level,
                    matroshka_limit,
                )| {
                    self.row_to_entry(
                        &id,
                        template_type,
                        name,
                        description,
                        source_path,
                        cascade_level,
                        matroshka_limit,
                    )
                    .ok()
                },
            )
            .collect()
    }

    fn get(
        &self,
        id: &str,
    ) -> std::result::Result<RegistryEntry, hkask_types::ports::RegistryError> {
        self.get_entry(id).map_err(|e| {
            hkask_types::ports::RegistryError::NotFound(format!(
                "Template '{}' not found: {}",
                id, e
            ))
        })
    }
}

impl SkillRegistryIndex for SqliteRegistry {
    fn register_skill(&mut self, skill: Skill) {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO skills (id, domain, word_act, flow_def, know_act)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    skill.id,
                    skill.domain.as_str(),
                    skill.word_act,
                    skill.flow_def,
                    skill.know_act,
                ],
            )
            .map_err(|e| TemplateError::Manifest(format!("Failed to insert skill: {}", e)))
            .ok();

        // Delete existing cascade order entries
        self.conn
            .execute(
                "DELETE FROM skill_cascade_order WHERE skill_id = ?1",
                params![skill.id],
            )
            .ok();

        // Insert cascade order
        for (position, template_id) in skill.cascade_order.iter().enumerate() {
            self.conn
                .execute(
                    "INSERT INTO skill_cascade_order (skill_id, position, template_id) VALUES (?1, ?2, ?3)",
                    params![skill.id, position as i64, template_id],
                )
                .ok();
        }
    }

    fn get_skill(&self, id: &str) -> Option<Skill> {
        self.get_skill_owned(id)
    }

    fn list_skills(&self) -> Vec<Skill> {
        self.list_skills_owned()
    }

    fn skills_by_domain(&self, domain: TemplateType) -> Vec<Skill> {
        self.skills_by_domain_owned(domain)
    }

    fn skills_referencing_template(&self, template_id: &str) -> Vec<Skill> {
        self.skills_referencing_template_owned(template_id)
    }

    fn remove_skill(&mut self, id: &str) -> Option<Skill> {
        // Retrieve skill before deletion to return it
        let skill = self.get_skill_owned(id);
        self.conn
            .execute(
                "DELETE FROM skill_cascade_order WHERE skill_id = ?1",
                params![id],
            )
            .ok();
        self.conn
            .execute("DELETE FROM skills WHERE id = ?1", params![id])
            .ok();
        skill
    }
}

/// Owned-skill retrieval methods specific to SQLite (no lifetime borrowing)
impl SqliteRegistry {
    fn row_to_skill(
        &self,
        id: String,
        domain_str: String,
        word_act: Option<String>,
        flow_def: Option<String>,
        know_act: Option<String>,
    ) -> Option<Skill> {
        let domain = TemplateType::parse_str(&domain_str).unwrap_or(TemplateType::FlowDef);
        let cascade_order = self.cascade_order_for_skill(&id).ok()?;
        Some(Skill {
            id,
            domain,
            word_act,
            flow_def,
            know_act,
            cascade_order,
        })
    }

    /// Retrieve a skill by ID (owned)
    pub fn get_skill_owned(&self, id: &str) -> Option<Skill> {
        self.conn
            .query_row(
                "SELECT id, domain, word_act, flow_def, know_act FROM skills WHERE id = ?1",
                params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .ok()
            .and_then(|(id, domain_str, word_act, flow_def, know_act)| {
                self.row_to_skill(id, domain_str, word_act, flow_def, know_act)
            })
    }

    /// List all skills (owned)
    pub fn list_skills_owned(&self) -> Vec<Skill> {
        let mut stmt = match self
            .conn
            .prepare("SELECT id, domain, word_act, flow_def, know_act FROM skills")
        {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows: Vec<SkillRow> = match stmt.query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        }) {
            Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
            Err(_) => return Vec::new(),
        };

        let mut skills = Vec::new();
        for (id, domain_str, word_act, flow_def, know_act) in rows {
            if let Some(skill) = self.row_to_skill(id, domain_str, word_act, flow_def, know_act) {
                skills.push(skill);
            }
        }
        skills
    }

    /// List skills by domain (owned)
    pub fn skills_by_domain_owned(&self, domain: TemplateType) -> Vec<Skill> {
        let mut stmt = match self.conn.prepare(
            "SELECT id, domain, word_act, flow_def, know_act FROM skills WHERE domain = ?1",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows: Vec<SkillRow> = match stmt.query_map(params![domain.as_str()], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        }) {
            Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
            Err(_) => return Vec::new(),
        };

        let mut skills = Vec::new();
        for (id, domain_str, word_act, flow_def, know_act) in rows {
            if let Some(skill) = self.row_to_skill(id, domain_str, word_act, flow_def, know_act) {
                skills.push(skill);
            }
        }
        skills
    }

    /// Find skills referencing a template (owned)
    pub fn skills_referencing_template_owned(&self, template_id: &str) -> Vec<Skill> {
        let mut stmt = match self.conn.prepare(
            "SELECT id, domain, word_act, flow_def, know_act FROM skills WHERE word_act = ?1 OR flow_def = ?1 OR know_act = ?1"
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows: Vec<SkillRow> = match stmt.query_map(params![template_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        }) {
            Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
            Err(_) => return Vec::new(),
        };

        let mut skills = Vec::new();
        for (id, domain_str, word_act, flow_def, know_act) in rows {
            if let Some(skill) = self.row_to_skill(id, domain_str, word_act, flow_def, know_act) {
                skills.push(skill);
            }
        }
        skills
    }

    fn cascade_order_for_skill(&self, skill_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT template_id FROM skill_cascade_order WHERE skill_id = ?1 ORDER BY position",
            )
            .map_err(|e| {
                TemplateError::Database(format!("Failed to prepare cascade query: {}", e))
            })?;

        let rows = stmt
            .query_map(params![skill_id], |row| row.get(0))
            .map_err(|e| TemplateError::Database(format!("Failed to query cascade: {}", e)))?;

        let mut result = Vec::new();
        for row in rows {
            match row {
                Ok(template_id) => result.push(template_id),
                Err(e) => {
                    return Err(TemplateError::Database(format!("Cascade row error: {}", e)));
                }
            }
        }
        Ok(result)
    }
}
