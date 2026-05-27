use super::types::*;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

pub struct ScholarStore {
    pub conn: Arc<std::sync::Mutex<rusqlite::Connection>>,
}

impl ScholarStore {
    pub fn new(path: &str) -> Result<Arc<Self>, anyhow::Error> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS papers (
                paper_id     TEXT PRIMARY KEY,
                title        TEXT,
                abstract_text TEXT,
                year         INTEGER,
                citation_count INTEGER,
                reference_count INTEGER,
                url          TEXT,
                venue        TEXT,
                publication_date TEXT,
                external_ids TEXT,
                stored_at    TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS authors (
                author_id   TEXT PRIMARY KEY,
                name        TEXT,
                paper_count INTEGER,
                citation_count INTEGER,
                h_index     INTEGER,
                url         TEXT,
                stored_at   TEXT DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS paper_authors (
                paper_id    TEXT NOT NULL REFERENCES papers(paper_id) ON DELETE CASCADE,
                author_id   TEXT NOT NULL REFERENCES authors(author_id) ON DELETE CASCADE,
                position    INTEGER DEFAULT 0,
                PRIMARY KEY (paper_id, author_id)
            );

            CREATE TABLE IF NOT EXISTS citations (
                citing_paper_id TEXT NOT NULL,
                cited_paper_id TEXT NOT NULL,
                PRIMARY KEY (citing_paper_id, cited_paper_id)
            );

            CREATE INDEX IF NOT EXISTS idx_citations_cited ON citations(cited_paper_id);
            CREATE INDEX IF NOT EXISTS idx_papers_year ON papers(year);
            CREATE INDEX IF NOT EXISTS idx_paper_authors_author ON paper_authors(author_id);",
        )?;
        Ok(Arc::new(Self {
            conn: Arc::new(std::sync::Mutex::new(conn)),
        }))
    }

    pub fn store_paper(&self, paper: &Paper) -> Result<(), anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO papers (paper_id, title, abstract_text, year, citation_count, reference_count, url, venue, publication_date, external_ids)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                paper.paper_id,
                paper.title,
                paper.abstract_text,
                paper.year,
                paper.citation_count,
                paper.reference_count,
                paper.url,
                paper.venue,
                paper.publication_date,
                paper.external_ids.as_ref().map(|v| v.to_string()),
            ],
        )?;
        if let Some(ref authors) = paper.authors {
            for (pos, author) in authors.iter().enumerate() {
                if let Some(ref aid) = author.author_id {
                    if let Some(ref name) = author.name {
                        conn.execute(
                            "INSERT OR IGNORE INTO authors (author_id, name) VALUES (?1, ?2)",
                            rusqlite::params![aid, name],
                        )?;
                    }
                    conn.execute(
                        "INSERT OR REPLACE INTO paper_authors (paper_id, author_id, position) VALUES (?1, ?2, ?3)",
                        rusqlite::params![paper.paper_id, aid, pos as i32],
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn store_citation(&self, citing_id: &str, cited_id: &str) -> Result<(), anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO citations (citing_paper_id, cited_paper_id) VALUES (?1, ?2)",
            rusqlite::params![citing_id, cited_id],
        )?;
        Ok(())
    }

    pub fn get_paper(&self, paper_id: &str) -> Result<Option<Paper>, anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        let result = conn.query_row(
            "SELECT paper_id, title, abstract_text, year, citation_count, reference_count, url, venue, publication_date, external_ids FROM papers WHERE paper_id = ?1",
            [paper_id],
            |row| {
                let paper_id: String = row.get(0)?;
                let title: Option<String> = row.get(1)?;
                let abstract_text: Option<String> = row.get(2)?;
                let year: Option<i32> = row.get(3)?;
                let citation_count: Option<i64> = row.get(4)?;
                let reference_count: Option<i64> = row.get(5)?;
                let url: Option<String> = row.get(6)?;
                let venue: Option<String> = row.get(7)?;
                let publication_date: Option<String> = row.get(8)?;
                let external_ids_str: Option<String> = row.get(9)?;
                let external_ids: Option<serde_json::Value> = external_ids_str
                    .and_then(|s| serde_json::from_str(&s).ok());
                Ok(Paper { paper_id, title, abstract_text, year, citation_count, reference_count, url, authors: None, venue, publication_date, external_ids })
            },
        );
        match result {
            Ok(paper) => Ok(Some(paper)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn get_stats(&self) -> Result<serde_json::Value, anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        let papers: i64 = conn.query_row("SELECT COUNT(*) FROM papers", [], |row| row.get(0))?;
        let authors: i64 = conn.query_row("SELECT COUNT(*) FROM authors", [], |row| row.get(0))?;
        let citations: i64 =
            conn.query_row("SELECT COUNT(*) FROM citations", [], |row| row.get(0))?;
        Ok(serde_json::json!({ "papers": papers, "authors": authors, "citations": citations }))
    }

    pub fn traverse_graph(&self, seeds: &[String]) -> Result<serde_json::Value, anyhow::Error> {
        let conn = self.conn.lock().unwrap();
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<String> = VecDeque::new();

        for seed in seeds {
            if !visited.contains(seed) {
                visited.insert(seed.clone());
                queue.push_back(seed.clone());
            }
        }

        while let Some(pid) = queue.pop_front() {
            if let Ok(paper) = conn.query_row(
                "SELECT paper_id, title, year, citation_count FROM papers WHERE paper_id = ?1",
                [&pid],
                |row| {
                    Ok(GraphNode {
                        paper_id: row.get(0)?,
                        title: row.get(1)?,
                        year: row.get(2)?,
                        citation_count: row.get(3)?,
                    })
                },
            ) {
                nodes.push(paper);
            }

            let mut citing_stmt =
                conn.prepare("SELECT citing_paper_id FROM citations WHERE cited_paper_id = ?1")?;
            let citing: Vec<String> = citing_stmt
                .query_map([&pid], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            for cid in &citing {
                edges.push(GraphEdge {
                    source: cid.clone(),
                    target: pid.clone(),
                    edge_type: "cites".to_string(),
                });
                if !visited.contains(cid) {
                    visited.insert(cid.clone());
                    queue.push_back(cid.clone());
                }
            }

            let mut cited_stmt =
                conn.prepare("SELECT cited_paper_id FROM citations WHERE citing_paper_id = ?1")?;
            let cited: Vec<String> = cited_stmt
                .query_map([&pid], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            for cid in &cited {
                edges.push(GraphEdge {
                    source: pid.clone(),
                    target: cid.clone(),
                    edge_type: "cites".to_string(),
                });
                if !visited.contains(cid) {
                    visited.insert(cid.clone());
                    queue.push_back(cid.clone());
                }
            }
        }

        Ok(serde_json::json!({ "nodes": nodes, "edges": edges, "seed_count": seeds.len() }))
    }
}
