//! SQLite registry adapter — persistent template registry backed by SQLite.
//!
//! Connection stored as `Arc<Mutex<Connection>>` for thread-safe shared access,
//! consistent with `hkask_storage::Database::conn_arc()`. Use `new_with_conn()`
//! when opening through `hkask_storage::Database` (SQLCipher-encrypted).

use crate::ports::{RegistryEntry, RegistryIndex, Result, TemplateError};
use hkask_types::ports::{BundleRegistryIndex, SkillRegistryIndex, SkillZone};
use hkask_types::{
    BundleManifest, HLexicon, InfrastructureError, Skill, SkillPolarity, TemplateType, Visibility,
};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};

type SkillRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
    Option<String>,
);
type TemplateRow = (String, TemplateType, String, String, String, u32, u32);

fn parse_template_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TemplateRow> {
    let tt_str: String = row.get(1)?;
    let tt = TemplateType::parse_str(&tt_str).ok_or_else(|| {
        rusqlite::Error::ToSqlConversionFailure(format!("Unknown template type: {}", tt_str).into())
    })?;
    Ok((
        row.get(0)?,
        tt,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
    ))
}

fn query_column(conn: &Connection, sql: &str, id: &str) -> Result<Vec<String>> {
    let db_err = |ctx: &str, e| {
        TemplateError::Database(InfrastructureError::Database(format!("{ctx}: {e}")))
    };
    let results: Vec<String> = conn
        .prepare(sql)
        .map_err(|e| db_err("Prepare", e))?
        .query_map(params![id], |row| row.get::<_, String>(0))
        .map_err(|e| db_err("Query", e))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(results)
}

// ── SqliteRegistry ─────────────────────────────────────────────────────────

pub struct SqliteRegistry {
    conn: Arc<Mutex<Connection>>,
    hlexicon: Option<HLexicon>,
}

impl SqliteRegistry {
    pub fn new(path: Option<&str>) -> Result<Self> {
        let conn = match path {
            Some(p) => Connection::open(p)
                .map_err(|e| TemplateError::Manifest(format!("Failed to open SQLite: {}", e)))?,
            None => {
                tracing::warn!(target: "hkask.templates",
                    "No database path — template registry is in-memory and will be lost on restart.");
                Connection::open_in_memory()
                    .map_err(|e| TemplateError::Manifest(format!("In-memory SQLite: {}", e)))?
            }
        };
        let mut registry = Self {
            conn: Arc::new(Mutex::new(conn)),
            hlexicon: None,
        };
        registry.init_schema()?;
        Ok(registry)
    }

    pub fn new_with_conn(conn: Arc<Mutex<Connection>>) -> Result<Self> {
        let mut registry = Self {
            conn,
            hlexicon: None,
        };
        registry.init_schema()?;
        Ok(registry)
    }

    fn init_schema(&mut self) -> Result<()> {
        self.conn.lock().unwrap().execute_batch(concat!(
            "CREATE TABLE IF NOT EXISTS templates(id TEXT PRIMARY KEY, template_type TEXT NOT NULL, name TEXT NOT NULL DEFAULT '', description TEXT, source_path TEXT NOT NULL, cascade_level INTEGER NOT NULL DEFAULT 0, matroshka_limit INTEGER NOT NULL DEFAULT 7, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME DEFAULT CURRENT_TIMESTAMP);",
            "CREATE TABLE IF NOT EXISTS lexicon_terms(template_id TEXT NOT NULL, term TEXT NOT NULL, PRIMARY KEY(template_id, term), FOREIGN KEY(template_id) REFERENCES templates(id));",
            "CREATE TABLE IF NOT EXISTS template_capabilities(template_id TEXT NOT NULL, capability TEXT NOT NULL, PRIMARY KEY(template_id, capability), FOREIGN KEY(template_id) REFERENCES templates(id));",
            "CREATE TABLE IF NOT EXISTS provenance(id INTEGER PRIMARY KEY AUTOINCREMENT, template_id TEXT NOT NULL, git_sha TEXT NOT NULL, modified_by TEXT NOT NULL, modified_at DATETIME DEFAULT CURRENT_TIMESTAMP, branch TEXT, commit_message TEXT, FOREIGN KEY(template_id) REFERENCES templates(id));",
            "CREATE INDEX IF NOT EXISTS idx_templates_type ON templates(template_type);",
            "CREATE INDEX IF NOT EXISTS idx_lexicon_terms ON lexicon_terms(term);",
            "CREATE INDEX IF NOT EXISTS idx_provenance_template ON provenance(template_id);",
            "CREATE INDEX IF NOT EXISTS idx_template_capabilities ON template_capabilities(capability);",
            "CREATE TABLE IF NOT EXISTS skills(id TEXT PRIMARY KEY, domain TEXT NOT NULL, word_act TEXT, flow_def TEXT, know_act TEXT, polarity TEXT, content_hash TEXT, visibility TEXT NOT NULL DEFAULT 'private', zone TEXT NOT NULL DEFAULT 'private', namespace TEXT, created_at DATETIME DEFAULT CURRENT_TIMESTAMP);",
            "CREATE INDEX IF NOT EXISTS idx_skills_domain ON skills(domain);",
            "CREATE INDEX IF NOT EXISTS idx_skills_visibility ON skills(visibility);",
            "CREATE TABLE IF NOT EXISTS bundles(id TEXT PRIMARY KEY, name TEXT NOT NULL, description TEXT NOT NULL, version TEXT NOT NULL, editor TEXT NOT NULL DEFAULT 'curator-or-human-admin', visibility TEXT NOT NULL DEFAULT 'Private', manifest_json TEXT NOT NULL, created_at DATETIME DEFAULT CURRENT_TIMESTAMP, updated_at DATETIME DEFAULT CURRENT_TIMESTAMP);",
            "CREATE TABLE IF NOT EXISTS bundle_skills(bundle_id TEXT NOT NULL, skill_id TEXT NOT NULL, polarity TEXT, manifest_ref TEXT, content_hash TEXT, position INTEGER NOT NULL, PRIMARY KEY(bundle_id, skill_id), FOREIGN KEY(bundle_id) REFERENCES bundles(id));",
            "CREATE INDEX IF NOT EXISTS idx_bundles_visibility ON bundles(visibility);",
            "CREATE INDEX IF NOT EXISTS idx_bundle_skills_bundle ON bundle_skills(bundle_id);",
            "CREATE INDEX IF NOT EXISTS idx_bundle_skills_skill ON bundle_skills(skill_id);",
        )).map_err(|e| TemplateError::Manifest(format!("Schema init: {}", e)))?;
        Ok(())
    }

    pub fn set_lexicon(&mut self, lexicon: HLexicon) {
        self.hlexicon = Some(lexicon);
    }

    pub fn register(&mut self, entry: RegistryEntry) -> Result<()> {
        for warning in &entry.validate() {
            tracing::warn!(target: "hkask.templates", "{}", warning);
        }
        if let Some(ref lexicon) = self.hlexicon {
            let unknown = lexicon.validate(&entry.lexicon_terms);
            if !unknown.is_empty() {
                tracing::warn!(
                    target: "hkask.templates",
                    template_id = %entry.id,
                    unknown_terms = ?unknown,
                    "Lexicon terms not in canonical vocabulary"
                );
            }
        }
        let mut conn = self.conn.lock().unwrap();
        let tx = conn
            .transaction()
            .map_err(|e| TemplateError::Manifest(format!("Transaction: {}", e)))?;
        tx.execute(
            "INSERT OR REPLACE INTO templates (id, template_type, name, description, source_path, cascade_level, matroshka_limit, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP)",
            params![entry.id, entry.template_type.as_str(), entry.name, entry.description, entry.source_path, entry.cascade_level, entry.matroshka_limit],
        ).map_err(|e| TemplateError::Manifest(format!("Insert: {}", e)))?;
        for (table, col, items) in [
            ("lexicon_terms", "term", &entry.lexicon_terms),
            (
                "template_capabilities",
                "capability",
                &entry.required_capabilities,
            ),
        ] {
            tx.execute(
                &format!("DELETE FROM {} WHERE template_id = ?1", table),
                params![entry.id],
            )
            .map_err(|e| TemplateError::Manifest(format!("Delete {col}: {}", e)))?;
            for item in items {
                tx.execute(
                    &format!(
                        "INSERT INTO {} (template_id, {}) VALUES (?1, ?2)",
                        table, col
                    ),
                    params![entry.id, item],
                )
                .map_err(|e| TemplateError::Manifest(format!("Insert {col}: {}", e)))?;
            }
        }
        tx.commit()
            .map_err(|e| TemplateError::Manifest(format!("Commit: {}", e)))?;
        Ok(())
    }

    fn row_to_entry(
        conn: &Connection,
        id: &str,
        tt: TemplateType,
        name: String,
        desc: String,
        sp: String,
        cl: u32,
        ml: u32,
    ) -> Result<RegistryEntry> {
        Ok(RegistryEntry {
            id: id.to_string(),
            template_type: tt,
            name,
            description: desc,
            source_path: sp,
            lexicon_terms: query_column(
                conn,
                "SELECT term FROM lexicon_terms WHERE template_id = ?1",
                id,
            )?,
            required_capabilities: query_column(
                conn,
                "SELECT capability FROM template_capabilities WHERE template_id = ?1",
                id,
            )?,
            cascade_level: cl,
            matroshka_limit: ml,
        })
    }

    pub fn get_entry(&self, id: &str) -> Result<RegistryEntry> {
        let conn = self.conn.lock().unwrap();
        let row = conn
            .prepare(Self::_T_SELECT)
            .map_err(|e| {
                TemplateError::Database(InfrastructureError::Database(format!("Prepare: {}", e)))
            })?
            .query_row(params![id], parse_template_row)
            .map_err(|e| TemplateError::NotFound(format!("Template '{}': {}", id, e)))?;
        Self::row_to_entry(&conn, &row.0, row.1, row.2, row.3, row.4, row.5, row.6)
    }

    pub fn search_by_lexicon(&self, term: &str) -> Result<Vec<RegistryEntry>> {
        let conn = self.conn.lock().unwrap();
        let rows: Vec<TemplateRow> = conn
    			.prepare("SELECT t.id, t.template_type, t.name, t.description, t.source_path, t.cascade_level, t.matroshka_limit FROM templates t JOIN lexicon_terms l ON t.id = l.template_id WHERE l.term = ?1")
    			.map_err(|e| TemplateError::Database(InfrastructureError::Database(format!("Prepare: {}", e))))?
    			.query_map(params![term], parse_template_row)
    			.map_err(|e| TemplateError::Database(InfrastructureError::Database(format!("Query: {}", e))))?
    			.filter_map(|r| r.ok()).collect();
        let mut results = Vec::new();
        for (id, tt, name, desc, sp, cl, ml) in rows {
            results.push(Self::row_to_entry(&conn, &id, tt, name, desc, sp, cl, ml)?);
        }
        Ok(results)
    }

    pub fn count(&self) -> usize {
        self.conn
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM templates", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap_or(0) as usize
    }

    const _T_SELECT: &str = "SELECT id, template_type, name, description, source_path, cascade_level, matroshka_limit FROM templates WHERE id = ?1";
}

// ── RegistryIndex ──────────────────────────────────────────────────────────

impl RegistryIndex for SqliteRegistry {
    fn list(&self, domain_hint: Option<TemplateType>) -> Vec<RegistryEntry> {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let sql = "SELECT id, template_type, name, description, source_path, cascade_level, matroshka_limit FROM templates";
        let (query_sql, query_params): (&str, &[rusqlite::types::Value]) = match &domain_hint {
            Some(tt) => (
                &format!("{sql} WHERE template_type = ?1"),
                &[rusqlite::types::Value::Text(tt.as_str().to_string())][..],
            ),
            None => (sql, &[]),
        };
        let mut stmt = match conn.prepare(query_sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows: Vec<TemplateRow> = stmt
            .query_map(
                rusqlite::params_from_iter(
                    query_params
                        .iter()
                        .map(|v| v as &dyn rusqlite::types::ToSql),
                ),
                parse_template_row,
            )
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();
        rows.into_iter()
            .filter_map(|(id, tt, name, desc, sp, cl, ml)| {
                Self::row_to_entry(&conn, &id, tt, name, desc, sp, cl, ml).ok()
            })
            .collect()
    }

    fn get(
        &self,
        id: &str,
    ) -> std::result::Result<RegistryEntry, hkask_types::ports::RegistryError> {
        self.get_entry(id).map_err(|e| {
            hkask_types::ports::RegistryError::NotFound(format!("Template '{}': {}", id, e))
        })
    }
}

// ── SkillRegistryIndex ─────────────────────────────────────────────────────

impl SkillRegistryIndex for SqliteRegistry {
    fn register_skill(&mut self, skill: Skill) {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO skills (id, domain, word_act, flow_def, know_act, polarity, content_hash, visibility, zone, namespace) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![skill.id, skill.domain.as_str(), skill.word_act, skill.flow_def, skill.know_act,
                skill.polarity.as_ref().map(|p| p.as_str()), skill.content_hash,
                skill.visibility.as_str(), skill.zone.as_str(), skill.namespace],
        ).map_err(|e| TemplateError::Manifest(format!("Insert skill: {}", e))).ok();
    }

    fn get_skill(&self, id: &str) -> Option<Skill> {
        self.get_skill_owned(id)
    }
    fn list_skills(&self) -> Vec<Skill> {
        self.list_skills_owned()
    }
    fn list_skills_by_visibility(&self, v: Visibility) -> Vec<Skill> {
        self.list_skills_owned()
            .into_iter()
            .filter(|s| s.visibility == v)
            .collect()
    }
    fn skills_by_domain(&self, domain: TemplateType) -> Vec<Skill> {
        self.skills_by_domain_owned(domain)
    }
    fn skills_referencing_template(&self, tid: &str) -> Vec<Skill> {
        self.skills_referencing_template_owned(tid)
    }

    fn remove_skill(&mut self, id: &str) -> Option<Skill> {
        let skill = self.get_skill_owned(id);
        self.conn
            .lock()
            .unwrap()
            .execute("DELETE FROM skills WHERE id = ?1", params![id])
            .ok();
        skill
    }
}

// ── BundleRegistryIndex ────────────────────────────────────────────────────

impl BundleRegistryIndex for SqliteRegistry {
    fn register_bundle(&mut self, bundle: BundleManifest) {
        let manifest_json = serde_json::to_string(&bundle).unwrap_or_else(|_| "{}".into());
        let conn = self.conn.lock().unwrap();
        conn.execute("INSERT OR REPLACE INTO bundles (id, name, description, version, editor, visibility, manifest_json, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, CURRENT_TIMESTAMP)", params![bundle.id, bundle.name, bundle.description, bundle.version, bundle.editor, bundle.visibility.as_str(), manifest_json]).map_err(|e| TemplateError::Manifest(format!("Insert bundle: {}", e))).ok();
        conn.execute(
            "DELETE FROM bundle_skills WHERE bundle_id = ?1",
            params![bundle.id],
        )
        .ok();
        for (position, skill) in bundle.skills.iter().enumerate() {
            conn.execute("INSERT INTO bundle_skills (bundle_id, skill_id, polarity, manifest_ref, content_hash, position) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![bundle.id, skill.id, Some(skill.polarity.as_str()), skill.manifest_ref, skill.content_hash, position as i64]).ok();
        }
    }

    fn get_bundle(&self, id: &str) -> Option<BundleManifest> {
        self.conn
            .lock()
            .unwrap()
            .query_row(
                "SELECT manifest_json FROM bundles WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .and_then(|json| serde_json::from_str(&json).ok())
    }

    fn list_bundles(&self) -> Vec<BundleManifest> {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut stmt = match conn.prepare("SELECT manifest_json FROM bundles") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        stmt.query_map([], |row| row.get::<_, String>(0))
            .ok()
            .map(|rows| {
                rows.filter_map(|r| r.ok())
                    .filter_map(|json| serde_json::from_str(&json).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn remove_bundle(&mut self, id: &str) -> Option<BundleManifest> {
        let bundle = self.get_bundle(id);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM bundle_skills WHERE bundle_id = ?1",
            params![id],
        )
        .ok();
        conn.execute("DELETE FROM bundles WHERE id = ?1", params![id])
            .ok();
        bundle
    }

    fn find_bundle_by_skills(&self, skill_ids: &[String]) -> Option<BundleManifest> {
        let target: std::collections::HashSet<&str> =
            skill_ids.iter().map(|s| s.as_str()).collect();
        self.list_bundles().into_iter().find(|b| {
            b.skills
                .iter()
                .map(|s| s.id.as_str())
                .collect::<std::collections::HashSet<_>>()
                == target
        })
    }
}

// ── Owned-skill retrieval ──────────────────────────────────────────────────

impl SqliteRegistry {
    fn row_to_skill(
        id: String,
        domain_str: String,
        word_act: Option<String>,
        flow_def: Option<String>,
        know_act: Option<String>,
        polarity_str: Option<String>,
        content_hash: Option<String>,
        visibility_str: String,
        zone_str: String,
        namespace: Option<String>,
    ) -> Option<Skill> {
        Some(Skill {
            id,
            domain: TemplateType::parse_str(&domain_str).unwrap_or(TemplateType::FlowDef),
            word_act,
            flow_def,
            know_act,
            polarity: polarity_str.and_then(|s| SkillPolarity::parse_str(&s)),
            content_hash,
            visibility: Visibility::parse_str(&visibility_str).unwrap_or(Visibility::Private),
            zone: SkillZone::parse_str(&zone_str).unwrap_or(SkillZone::Private),
            namespace,
        })
    }

    pub fn get_skill_owned(&self, id: &str) -> Option<Skill> {
        self.conn.lock().unwrap().query_row(
            "SELECT id, domain, word_act, flow_def, know_act, polarity, content_hash, visibility, zone, namespace FROM skills WHERE id = ?1", params![id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?)),
        ).ok().and_then(|(id, ds, wa, fd, ka, ps, ch, vs, zs, ns)| Self::row_to_skill(id, ds, wa, fd, ka, ps, ch, vs, zs, ns))
    }

    fn query_skills(&self, sql: &str, params: &[rusqlite::types::Value]) -> Vec<Skill> {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut stmt = match conn.prepare(sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows: Vec<SkillRow> = match stmt.query_map(
            rusqlite::params_from_iter(params.iter().map(|v| v as &dyn rusqlite::types::ToSql)),
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            },
        ) {
            Ok(m) => m.filter_map(|r| r.ok()).collect(),
            Err(_) => return Vec::new(),
        };
        let mut skills = Vec::with_capacity(rows.len());
        for (id, ds, wa, fd, ka, ps, ch, vs, zs, ns) in rows {
            if let Some(s) = Self::row_to_skill(id, ds, wa, fd, ka, ps, ch, vs, zs, ns) {
                skills.push(s);
            }
        }
        skills
    }

    const _SKILLS_SELECT: &str = "SELECT id, domain, word_act, flow_def, know_act, polarity, content_hash, visibility, zone, namespace FROM skills";

    pub fn list_skills_owned(&self) -> Vec<Skill> {
        self.query_skills(Self::_SKILLS_SELECT, &[])
    }

    pub fn skills_by_domain_owned(&self, domain: TemplateType) -> Vec<Skill> {
        self.query_skills(
            &format!("{} WHERE domain = ?1", Self::_SKILLS_SELECT),
            &[rusqlite::types::Value::Text(domain.as_str().to_string())],
        )
    }

    pub fn skills_referencing_template_owned(&self, tid: &str) -> Vec<Skill> {
        self.query_skills(
            &format!(
                "{} WHERE word_act = ?1 OR flow_def = ?1 OR know_act = ?1",
                Self::_SKILLS_SELECT
            ),
            &[rusqlite::types::Value::Text(tid.to_string())],
        )
    }
}
