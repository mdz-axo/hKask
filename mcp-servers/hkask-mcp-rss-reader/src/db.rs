use rusqlite::Connection;

use crate::types::EditTagRequest;
// P4.3: Use the canonical timestamp helper from `hkask-types` rather than
// inlining `chrono::Utc::now().to_rfc3339()` at every call site.
use hkask_types::now_rfc3339;

// RSS schema DDL — executed as extensions via Database::open_with_extensions()

/// RSS-specific schema DDL for use with `Database::open_with_extensions()`.
///
/// Creates 4 domain tables (feeds, subscriptions, entries, entry_states),
/// 1 FTS5 virtual table (entries_fts), 4 indexes, and 2 triggers
/// for FTS synchronization.
pub const RSS_SCHEMA_DDL: &str = "
    PRAGMA journal_mode=WAL;
    PRAGMA foreign_keys=ON;

    CREATE TABLE IF NOT EXISTS feeds (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        url         TEXT NOT NULL UNIQUE,
        title       TEXT,
        description TEXT,
        site_url    TEXT,
        etag        TEXT,
        last_modified TEXT,
        last_fetched_at TEXT,
        created_at  TEXT DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS subscriptions (
        id        INTEGER PRIMARY KEY AUTOINCREMENT,
        feed_id   INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
        stream_id TEXT NOT NULL UNIQUE,
        title     TEXT,
        label     TEXT,
        folder    TEXT,
        added_at  TEXT DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS entries (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        feed_id      INTEGER NOT NULL REFERENCES feeds(id) ON DELETE CASCADE,
        entry_id     TEXT NOT NULL,
        title        TEXT,
        url          TEXT,
        author       TEXT,
        content      TEXT,
        summary      TEXT,
        published_at TEXT,
        updated_at   TEXT,
        fetched_at   TEXT DEFAULT (datetime('now')),
        UNIQUE(feed_id, entry_id)
    );

    CREATE TABLE IF NOT EXISTS entry_states (
        entry_id   INTEGER PRIMARY KEY REFERENCES entries(id) ON DELETE CASCADE,
        is_read    INTEGER NOT NULL DEFAULT 0,
        is_starred INTEGER NOT NULL DEFAULT 0,
        read_at    TEXT,
        starred_at TEXT
    );

    CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(
        title, content, summary, content=''
    );

    CREATE INDEX IF NOT EXISTS idx_entries_feed_id ON entries(feed_id);
    CREATE INDEX IF NOT EXISTS idx_entries_published ON entries(published_at);
    CREATE INDEX IF NOT EXISTS idx_subscriptions_label ON subscriptions(label);
    CREATE INDEX IF NOT EXISTS idx_subscriptions_folder ON subscriptions(folder);

    CREATE TRIGGER IF NOT EXISTS entries_ai AFTER INSERT ON entries BEGIN
        INSERT INTO entries_fts(rowid, title, content, summary)
            VALUES (new.id, new.title, new.content, new.summary);
    END;

    CREATE TRIGGER IF NOT EXISTS entries_ad AFTER DELETE ON entries BEGIN
        INSERT INTO entries_fts(entries_fts, rowid, title, content, summary)
            VALUES ('delete', old.id, old.title, old.content, old.summary);
    END;
";

// DB write functions

pub fn upsert_feed(
    conn: &Connection,
    url: &str,
    feed: &feed_rs::model::Feed,
) -> Result<i64, anyhow::Error> {
    let title = feed
        .title
        .as_ref()
        .map(|t| t.content.as_str())
        .unwrap_or("");
    let description = feed
        .description
        .as_ref()
        .map(|t| t.content.as_str())
        .unwrap_or("");
    let site_url = feed.links.first().map(|l| l.href.as_str()).unwrap_or("");

    let existing_id: Option<i64> = conn
        .query_row("SELECT id FROM feeds WHERE url = ?1", [url], |row| {
            row.get(0)
        })
        .ok();

    if let Some(id) = existing_id {
        conn.execute(
            "UPDATE feeds SET title = ?1, description = ?2, site_url = ?3, last_fetched_at = datetime('now') WHERE id = ?4",
            rusqlite::params![title, description, site_url, id],
        )?;
        Ok(id)
    } else {
        conn.execute(
            "INSERT INTO feeds (url, title, description, site_url, last_fetched_at) VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            rusqlite::params![url, title, description, site_url],
        )?;
        Ok(conn.last_insert_rowid())
    }
}

pub fn insert_entries(
    conn: &Connection,
    feed_id: i64,
    entries: &[feed_rs::model::Entry],
) -> Result<usize, anyhow::Error> {
    let mut new_count = 0;
    for entry in entries {
        let entry_id = entry.id.clone();
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM entries WHERE feed_id = ?1 AND entry_id = ?2",
                rusqlite::params![feed_id, entry_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)?;

        if exists {
            continue;
        }

        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.clone())
            .unwrap_or_default();
        let url = entry
            .links
            .first()
            .map(|l| l.href.clone())
            .unwrap_or_default();
        let author = entry
            .authors
            .first()
            .map(|a| a.name.clone())
            .unwrap_or_default();
        let content = entry
            .content
            .as_ref()
            .and_then(|c| c.body.clone())
            .unwrap_or_default();
        let summary = entry
            .summary
            .as_ref()
            .map(|t| t.content.clone())
            .unwrap_or_default();
        let published_at = entry
            .published
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();
        let updated_at = entry.updated.map(|dt| dt.to_rfc3339()).unwrap_or_default();

        conn.execute(
            "INSERT INTO entries (feed_id, entry_id, title, url, author, content, summary, published_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![feed_id, entry_id, title, url, author, content, summary, published_at, updated_at],
        )?;
        new_count += 1;
    }
    Ok(new_count)
}

pub fn update_feed_cache_headers(
    conn: &Connection,
    feed_id: i64,
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> Result<(), anyhow::Error> {
    conn.execute(
        "UPDATE feeds SET etag = ?1, last_modified = ?2 WHERE id = ?3",
        rusqlite::params![etag, last_modified, feed_id],
    )?;
    Ok(())
}

// Stream resolution (Google Reader data model)

pub fn resolve_feed_url(conn: &Connection, stream_id: &str) -> Option<String> {
    if let Some(rest) = stream_id.strip_prefix("feed/") {
        Some(rest.to_string())
    } else {
        conn.query_row(
            "SELECT f.url FROM subscriptions s JOIN feeds f ON s.feed_id = f.id WHERE s.stream_id = ?1",
            [stream_id],
            |row| row.get(0),
        ).ok()
    }
}

pub fn build_stream_where(stream_id: &str) -> (String, Vec<Box<dyn rusqlite::types::ToSql>>) {
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut join_clause = String::new();
    let mut where_parts: Vec<String> = Vec::new();

    if stream_id == "user/-/state/com.google/reading-list" {
        join_clause = "JOIN subscriptions sub ON e.feed_id = sub.feed_id".to_string();
    } else if stream_id == "user/-/state/com.google/starred" {
        where_parts.push("s.is_starred = 1".to_string());
    } else if stream_id == "user/-/state/com.google/read" {
        where_parts.push("s.is_read = 1".to_string());
    } else if let Some(label) = stream_id.strip_prefix("user/-/label/") {
        join_clause = "JOIN subscriptions sub ON e.feed_id = sub.feed_id".to_string();
        params.push(Box::new(label.to_string()));
        where_parts.push("sub.label = ?".to_string());
    } else if let Some(feed_url) = stream_id.strip_prefix("feed/") {
        join_clause = "JOIN feeds f ON e.feed_id = f.id".to_string();
        params.push(Box::new(feed_url.to_string()));
        where_parts.push("f.url = ?".to_string());
    } else {
        where_parts.push("1 = 0".to_string());
    }

    let where_clause = if where_parts.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_parts.join(" AND "))
    };

    (format!("{join_clause} {where_clause}"), params)
}

// Query functions

pub fn query_entries(
    conn: &Connection,
    stream_id: &str,
    unread_only: bool,
    starred_only: bool,
    offset: usize,
    limit: usize,
) -> Result<Vec<serde_json::Value>, anyhow::Error> {
    let (join_where, mut params) = build_stream_where(stream_id);

    let mut extra_where = Vec::new();
    if unread_only {
        extra_where.push("(s.is_read = 0 OR s.is_read IS NULL)");
    }
    if starred_only {
        extra_where.push("s.is_starred = 1");
    }

    let extra = if extra_where.is_empty() {
        String::new()
    } else if join_where.contains("WHERE") {
        format!(" AND {}", extra_where.join(" AND "))
    } else {
        format!("WHERE {}", extra_where.join(" AND "))
    };

    let sql = format!(
        "SELECT e.id, e.entry_id, e.title, e.url, e.author, e.summary, e.published_at, e.updated_at,
                COALESCE(s.is_read, 0) as is_read, COALESCE(s.is_starred, 0) as is_starred
         FROM entries e
         LEFT JOIN entry_states s ON e.id = s.entry_id
         {join_where}{extra}
         ORDER BY e.published_at DESC
         LIMIT ? OFFSET ?"
    );

    params.push(Box::new(limit as i64));
    params.push(Box::new(offset as i64));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        let is_read: i64 = row.get("is_read")?;
        let is_starred: i64 = row.get("is_starred")?;
        Ok(serde_json::json!({
            "id": row.get::<_, i64>("id")?,
            "entry_id": row.get::<_, String>("entry_id")?,
            "title": row.get::<_, Option<String>>("title")?.unwrap_or_default(),
            "url": row.get::<_, Option<String>>("url")?.unwrap_or_default(),
            "author": row.get::<_, Option<String>>("author")?.unwrap_or_default(),
            "summary": row.get::<_, Option<String>>("summary")?.unwrap_or_default(),
            "published_at": row.get::<_, Option<String>>("published_at")?.unwrap_or_default(),
            "updated_at": row.get::<_, Option<String>>("updated_at")?.unwrap_or_default(),
            "is_read": is_read == 1,
            "is_starred": is_starred == 1,
        }))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

pub fn count_entries(
    conn: &Connection,
    stream_id: &str,
    unread_only: bool,
) -> Result<usize, anyhow::Error> {
    let (join_where, params) = build_stream_where(stream_id);

    let extra = if unread_only {
        if join_where.contains("WHERE") {
            " AND (s.is_read = 0 OR s.is_read IS NULL)"
        } else {
            "WHERE (s.is_read = 0 OR s.is_read IS NULL)"
        }
    } else {
        ""
    };

    let sql = format!(
        "SELECT COUNT(*) FROM entries e LEFT JOIN entry_states s ON e.id = s.entry_id {join_where}{extra}"
    );
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let count: i64 = conn.query_row(&sql, param_refs.as_slice(), |row| row.get(0))?;
    Ok(count as usize)
}

// Mutation functions

pub fn mark_stream_read(conn: &Connection, stream_id: &str) -> Result<usize, anyhow::Error> {
    let (join_where, params) = build_stream_where(stream_id);

    let extra = if join_where.contains("WHERE") {
        " AND (s.is_read = 0 OR s.is_read IS NULL)"
    } else {
        "WHERE (s.is_read = 0 OR s.is_read IS NULL)"
    };

    let find_sql = format!(
        "SELECT e.id FROM entries e LEFT JOIN entry_states s ON e.id = s.entry_id {join_where}{extra}"
    );
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut stmt = conn.prepare(&find_sql)?;
    let entry_ids: Vec<i64> = stmt
        .query_map(param_refs.as_slice(), |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    let now = now_rfc3339();
    for id in &entry_ids {
        conn.execute(
            "INSERT INTO entry_states (entry_id, is_read, read_at) VALUES (?1, 1, ?2)
             ON CONFLICT(entry_id) DO UPDATE SET is_read = 1, read_at = ?2",
            rusqlite::params![id, now],
        )?;
    }
    Ok(entry_ids.len())
}

pub fn edit_tags(
    conn: &Connection,
    req: &EditTagRequest,
) -> Result<serde_json::Value, anyhow::Error> {
    let now = now_rfc3339();
    let mut updated = 0u64;

    for id in &req.entry_ids {
        let exists: bool = conn
            .query_row("SELECT COUNT(*) FROM entries WHERE id = ?1", [id], |row| {
                row.get::<_, i64>(0)
            })
            .map(|c| c > 0)?;
        if !exists {
            continue;
        }

        if req.add_read == Some(true) {
            conn.execute(
                "INSERT INTO entry_states (entry_id, is_read, read_at) VALUES (?1, 1, ?2)
                 ON CONFLICT(entry_id) DO UPDATE SET is_read = 1, read_at = ?2",
                rusqlite::params![id, now],
            )?;
            updated += 1;
        }
        if req.remove_read == Some(true) {
            conn.execute(
                "INSERT INTO entry_states (entry_id, is_read) VALUES (?1, 0)
                 ON CONFLICT(entry_id) DO UPDATE SET is_read = 0, read_at = NULL",
                rusqlite::params![id],
            )?;
            updated += 1;
        }
        if req.add_starred == Some(true) {
            conn.execute(
                "INSERT INTO entry_states (entry_id, is_starred, starred_at) VALUES (?1, 1, ?2)
                 ON CONFLICT(entry_id) DO UPDATE SET is_starred = 1, starred_at = ?2",
                rusqlite::params![id, now],
            )?;
            updated += 1;
        }
        if req.remove_starred == Some(true) {
            conn.execute(
                "INSERT INTO entry_states (entry_id, is_starred) VALUES (?1, 0)
                 ON CONFLICT(entry_id) DO UPDATE SET is_starred = 0, starred_at = NULL",
                rusqlite::params![id],
            )?;
            updated += 1;
        }
        if let Some(ref label) = req.add_label {
            conn.execute(
                "UPDATE subscriptions SET label = ?1 WHERE feed_id = (SELECT feed_id FROM entries WHERE id = ?2)",
                rusqlite::params![label, id],
            )?;
            updated += 1;
        }
        if let Some(ref label) = req.remove_label {
            conn.execute(
                "UPDATE subscriptions SET label = NULL WHERE label = ?1 AND feed_id = (SELECT feed_id FROM entries WHERE id = ?2)",
                rusqlite::params![label, id],
            )?;
            updated += 1;
        }
    }

    Ok(serde_json::json!({
        "updated": updated,
        "entry_count": req.entry_ids.len(),
    }))
}

// Search and listing

pub fn search_entries(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<serde_json::Value>, anyhow::Error> {
    let sql = "SELECT e.id, e.entry_id, e.title, e.url, e.author, e.summary, e.published_at,
                COALESCE(s.is_read, 0) as is_read, COALESCE(s.is_starred, 0) as is_starred
         FROM entries e
         JOIN entries_fts fts ON e.id = fts.rowid
         LEFT JOIN entry_states s ON e.id = s.entry_id
         WHERE entries_fts MATCH ?1
         ORDER BY rank
         LIMIT ?2";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(rusqlite::params![query, limit as i64], |row| {
        let is_read: i64 = row.get("is_read")?;
        let is_starred: i64 = row.get("is_starred")?;
        Ok(serde_json::json!({
            "id": row.get::<_, i64>("id")?,
            "entry_id": row.get::<_, String>("entry_id")?,
            "title": row.get::<_, Option<String>>("title")?.unwrap_or_default(),
            "url": row.get::<_, Option<String>>("url")?.unwrap_or_default(),
            "author": row.get::<_, Option<String>>("author")?.unwrap_or_default(),
            "summary": row.get::<_, Option<String>>("summary")?.unwrap_or_default(),
            "published_at": row.get::<_, Option<String>>("published_at")?.unwrap_or_default(),
            "is_read": is_read == 1,
            "is_starred": is_starred == 1,
        }))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

pub fn list_subscriptions(
    conn: &Connection,
    folder: Option<&str>,
) -> Result<Vec<serde_json::Value>, anyhow::Error> {
    let sql = if folder.is_some() {
        "SELECT s.stream_id, s.title, s.label, s.folder, s.added_at, f.url, f.title as feed_title
         FROM subscriptions s JOIN feeds f ON s.feed_id = f.id
         WHERE s.folder = ?1
         ORDER BY s.added_at"
    } else {
        "SELECT s.stream_id, s.title, s.label, s.folder, s.added_at, f.url, f.title as feed_title
         FROM subscriptions s JOIN feeds f ON s.feed_id = f.id
         ORDER BY s.added_at"
    };

    let map_row = |row: &rusqlite::Row| -> Result<serde_json::Value, rusqlite::Error> {
        let feed_title: Option<String> = row.get("feed_title")?;
        let sub_title: Option<String> = row.get("title")?;
        let display_title = sub_title.or(feed_title).unwrap_or_default();
        Ok(serde_json::json!({
            "stream_id": row.get::<_, String>("stream_id")?,
            "title": display_title,
            "url": row.get::<_, String>("url")?,
            "label": row.get::<_, Option<String>>("label")?,
            "folder": row.get::<_, Option<String>>("folder")?,
            "added_at": row.get::<_, Option<String>>("added_at")?.unwrap_or_default(),
        }))
    };

    let mut stmt = conn.prepare(sql)?;
    let results = if let Some(f) = folder {
        stmt.query_map([f], map_row)?
            .filter_map(|r| r.ok())
            .collect()
    } else {
        stmt.query_map([], map_row)?
            .filter_map(|r| r.ok())
            .collect()
    };
    Ok(results)
}

// OPML export

pub fn export_opml(conn: &Connection) -> Result<String, anyhow::Error> {
    let subs = list_subscriptions(conn, None)?;

    let mut folders: std::collections::BTreeMap<String, Vec<&serde_json::Value>> =
        std::collections::BTreeMap::new();
    let mut unfiled: Vec<&serde_json::Value> = Vec::new();

    for sub in &subs {
        if let Some(folder) = sub.get("folder").and_then(|f| f.as_str())
            && !folder.is_empty()
        {
            folders.entry(folder.to_string()).or_default().push(sub);
            continue;
        }
        unfiled.push(sub);
    }

    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<opml version=\"2.0\">\n  <head>\n    <title>hKask RSS Subscriptions</title>\n  </head>\n  <body>\n",
    );

    for (folder, subs) in &folders {
        xml.push_str(&format!(
            "    <outline text=\"{}\" title=\"{}\">\n",
            xml_escape(folder),
            xml_escape(folder)
        ));
        for sub in subs {
            let url = sub["url"].as_str().unwrap_or("");
            let title = sub["title"].as_str().unwrap_or("");
            xml.push_str(&format!(
                "      <outline type=\"rss\" text=\"{}\" title=\"{}\" xmlUrl=\"{}\" />\n",
                xml_escape(title),
                xml_escape(title),
                xml_escape(url)
            ));
        }
        xml.push_str("    </outline>\n");
    }

    for sub in &unfiled {
        let url = sub["url"].as_str().unwrap_or("");
        let title = sub["title"].as_str().unwrap_or("");
        xml.push_str(&format!(
            "    <outline type=\"rss\" text=\"{}\" title=\"{}\" xmlUrl=\"{}\" />\n",
            xml_escape(title),
            xml_escape(title),
            xml_escape(url)
        ));
    }

    xml.push_str("  </body>\n</opml>");
    Ok(xml)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// OPML import

pub fn import_opml(
    conn: &Connection,
    opml_content: &str,
) -> Result<serde_json::Value, anyhow::Error> {
    let re = regex::Regex::new(r#"<outline[^>]*xmlUrl\s*=\s*"([^"]+)"[^>]*/?\s*>"#)?;

    let mut imported = 0u32;
    let mut skipped = 0u32;
    let mut errors = 0u32;

    let feeds: Vec<String> = re
        .captures_iter(opml_content)
        .filter_map(|cap| Some(cap.get(1)?.as_str().to_string()))
        .collect();

    for url in feeds {
        let stream_id = format!("feed/{url}");
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM subscriptions WHERE stream_id = ?1",
                [&stream_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if exists {
            skipped += 1;
            continue;
        }

        conn.execute(
            "INSERT OR IGNORE INTO feeds (url, last_fetched_at) VALUES (?1, datetime('now'))",
            [&url],
        )?;

        let feed_id: i64 = conn
            .query_row("SELECT id FROM feeds WHERE url = ?1", [&url], |row| {
                row.get(0)
            })
            .unwrap_or(0);

        if feed_id == 0 {
            errors += 1;
            continue;
        }

        match conn.execute(
            "INSERT INTO subscriptions (feed_id, stream_id) VALUES (?1, ?2)",
            rusqlite::params![feed_id, stream_id],
        ) {
            Ok(_) => imported += 1,
            Err(_) => errors += 1,
        }
    }

    Ok(serde_json::json!({
        "imported": imported,
        "skipped": skipped,
        "errors": errors,
    }))
}
