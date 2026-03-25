use crate::embeddings;
use crate::model::{ChunkEmbeddingRecord, Snapshot};
use anyhow::{Context, Result};
use rusqlite::{params, Connection, Transaction};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn snapshot_dir(project_root: &Path) -> PathBuf {
    project_root.join(".project-memory")
}

pub fn snapshot_path(project_root: &Path) -> PathBuf {
    snapshot_dir(project_root).join("snapshot.json")
}

pub fn database_path(project_root: &Path) -> PathBuf {
    snapshot_dir(project_root).join("project-memory.db")
}

pub fn load_snapshot(project_root: &Path) -> Result<Snapshot> {
    let path = snapshot_path(project_root);
    let content = fs::read_to_string(&path)
        .with_context(|| format!("snapshot not found at {}; run ingest first", path.display()))?;
    serde_json::from_str(&content)
        .with_context(|| format!("parsing snapshot {}", path.display()))
}

pub fn save_snapshot(project_root: &Path, snapshot: &Snapshot) -> Result<PathBuf> {
    let path = snapshot_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating snapshot directory {}", parent.display()))?;
    }

    let content = serde_json::to_string_pretty(snapshot)?;
    fs::write(&path, content).with_context(|| format!("writing snapshot {}", path.display()))?;
    save_snapshot_to_database(project_root, snapshot)?;
    Ok(path)
}

fn save_snapshot_to_database(project_root: &Path, snapshot: &Snapshot) -> Result<()> {
    let database = database_path(project_root);
    let mut connection = Connection::open(&database)
        .with_context(|| format!("opening SQLite store {}", database.display()))?;
    initialize_schema(&connection)?;

    let transaction = connection
        .transaction()
        .context("starting SQLite snapshot transaction")?;
    replace_snapshot(&transaction, snapshot)?;
    transaction.commit().context("committing SQLite snapshot transaction")?;
    Ok(())
}

fn initialize_schema(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS snapshot_metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS snapshots (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                schema_version TEXT NOT NULL,
                project_root TEXT NOT NULL,
                generated_at TEXT NOT NULL,
                files_indexed INTEGER NOT NULL,
                requirements_detected INTEGER NOT NULL,
                trace_edges INTEGER NOT NULL,
                skipped_files INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS files (
                path TEXT PRIMARY KEY,
                file_type TEXT NOT NULL,
                bytes INTEGER NOT NULL,
                lines INTEGER NOT NULL,
                hash TEXT NOT NULL,
                title TEXT,
                content TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS chunks (
                chunk_id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                kind TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                title TEXT,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                content TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                FOREIGN KEY (file_path) REFERENCES files(path) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS chunk_embeddings (
                chunk_id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                dimensions INTEGER NOT NULL,
                content_hash TEXT NOT NULL,
                vector_json TEXT NOT NULL,
                FOREIGN KEY (chunk_id) REFERENCES chunks(chunk_id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS file_requirements (
                file_path TEXT NOT NULL,
                requirement_id TEXT NOT NULL,
                PRIMARY KEY (file_path, requirement_id),
                FOREIGN KEY (file_path) REFERENCES files(path) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS requirement_references (
                file_path TEXT NOT NULL,
                requirement_id TEXT NOT NULL,
                referenced_path TEXT NOT NULL,
                PRIMARY KEY (file_path, requirement_id, referenced_path),
                FOREIGN KEY (file_path) REFERENCES files(path) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS file_references (
                file_path TEXT NOT NULL,
                referenced_path TEXT NOT NULL,
                PRIMARY KEY (file_path, referenced_path),
                FOREIGN KEY (file_path) REFERENCES files(path) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS symbols (
                file_path TEXT NOT NULL,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                line INTEGER NOT NULL,
                PRIMARY KEY (file_path, name, line),
                FOREIGN KEY (file_path) REFERENCES files(path) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS imports (
                file_path TEXT NOT NULL,
                import_path TEXT NOT NULL,
                PRIMARY KEY (file_path, import_path),
                FOREIGN KEY (file_path) REFERENCES files(path) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS trace_edges (
                source_kind TEXT NOT NULL,
                source_id TEXT NOT NULL,
                target_kind TEXT NOT NULL,
                target_id TEXT NOT NULL,
                edge_type TEXT NOT NULL,
                status TEXT NOT NULL,
                evidence_source_path TEXT NOT NULL,
                evidence_detail TEXT NOT NULL,
                PRIMARY KEY (
                    source_kind,
                    source_id,
                    target_kind,
                    target_id,
                    edge_type,
                    evidence_source_path
                )
            );
            ",
        )
        .context("initializing SQLite schema")?;

    Ok(())
}

fn replace_snapshot(transaction: &Transaction<'_>, snapshot: &Snapshot) -> Result<()> {
    transaction
        .execute("DELETE FROM chunk_embeddings", [])
        .context("clearing chunk_embeddings")?;
    transaction
        .execute("DELETE FROM chunks", [])
        .context("clearing chunks")?;
    transaction
        .execute("DELETE FROM trace_edges", [])
        .context("clearing trace_edges")?;
    transaction
        .execute("DELETE FROM imports", [])
        .context("clearing imports")?;
    transaction
        .execute("DELETE FROM symbols", [])
        .context("clearing symbols")?;
    transaction
        .execute("DELETE FROM file_references", [])
        .context("clearing file_references")?;
    transaction
        .execute("DELETE FROM requirement_references", [])
        .context("clearing requirement_references")?;
    transaction
        .execute("DELETE FROM file_requirements", [])
        .context("clearing file_requirements")?;
    transaction
        .execute("DELETE FROM files", [])
        .context("clearing files")?;
    transaction
        .execute("DELETE FROM snapshots", [])
        .context("clearing snapshots")?;

    transaction
        .execute(
            "INSERT INTO snapshots (
                id,
                schema_version,
                project_root,
                generated_at,
                files_indexed,
                requirements_detected,
                trace_edges,
                skipped_files
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                1,
                snapshot.schema_version,
                snapshot.project_root,
                snapshot.generated_at,
                snapshot.stats.files_indexed as i64,
                snapshot.stats.requirements_detected as i64,
                snapshot.stats.trace_edges as i64,
                snapshot.stats.skipped_files as i64,
            ],
        )
        .context("writing snapshots row")?;

    transaction
        .execute(
            "INSERT OR REPLACE INTO snapshot_metadata (key, value) VALUES ('active_backend', 'sqlite+json')",
            [],
        )
        .context("writing active_backend metadata")?;
    transaction
        .execute(
            "INSERT OR REPLACE INTO snapshot_metadata (key, value) VALUES ('embedding_provider', ?1)",
            [embeddings::EMBEDDING_PROVIDER],
        )
        .context("writing embedding_provider metadata")?;

    let chunk_embeddings = embeddings::build_chunk_embeddings(snapshot);

    for file in &snapshot.files {
        transaction
            .execute(
                "INSERT INTO files (path, file_type, bytes, lines, hash, title, content)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    file.path,
                    file.file_type.to_string(),
                    file.bytes as i64,
                    file.lines as i64,
                    file.hash,
                    file.title,
                    file.content,
                ],
            )
            .with_context(|| format!("writing file row for {}", file.path))?;

        for chunk in &file.chunks {
            transaction
                .execute(
                    "INSERT INTO chunks (
                        chunk_id,
                        file_path,
                        kind,
                        ordinal,
                        title,
                        start_line,
                        end_line,
                        content,
                        content_hash
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        chunk.chunk_id,
                        file.path,
                        chunk.kind.to_string(),
                        chunk.ordinal as i64,
                        chunk.title,
                        chunk.start_line as i64,
                        chunk.end_line as i64,
                        chunk.content,
                        chunk.content_hash,
                    ],
                )
                .with_context(|| format!("writing chunk {} for {}", chunk.chunk_id, file.path))?;
        }

        for requirement_id in &file.requirement_ids {
            transaction
                .execute(
                    "INSERT INTO file_requirements (file_path, requirement_id) VALUES (?1, ?2)",
                    params![file.path, requirement_id],
                )
                .with_context(|| format!("writing requirement {requirement_id} for {}", file.path))?;
        }

        for (requirement_id, referenced_paths) in &file.requirement_references {
            for referenced_path in referenced_paths {
                transaction
                    .execute(
                        "INSERT INTO requirement_references (file_path, requirement_id, referenced_path)
                         VALUES (?1, ?2, ?3)",
                        params![file.path, requirement_id, referenced_path],
                    )
                    .with_context(|| {
                        format!(
                            "writing requirement reference {requirement_id} -> {referenced_path} for {}",
                            file.path
                        )
                    })?;
            }
        }

        for referenced_path in &file.referenced_paths {
            transaction
                .execute(
                    "INSERT INTO file_references (file_path, referenced_path) VALUES (?1, ?2)",
                    params![file.path, referenced_path],
                )
                .with_context(|| format!("writing file reference {referenced_path} for {}", file.path))?;
        }

        for symbol in &file.symbols {
            transaction
                .execute(
                    "INSERT INTO symbols (file_path, name, kind, line) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        file.path,
                        symbol.name,
                        format!("{:?}", symbol.kind).to_ascii_lowercase(),
                        symbol.line as i64,
                    ],
                )
                .with_context(|| format!("writing symbol {} for {}", symbol.name, file.path))?;
        }

        for import_path in &file.imports {
            transaction
                .execute(
                    "INSERT INTO imports (file_path, import_path) VALUES (?1, ?2)",
                    params![file.path, import_path],
                )
                .with_context(|| format!("writing import {import_path} for {}", file.path))?;
        }
    }

    for embedding in &chunk_embeddings {
        let vector_json = serde_json::to_string(&embedding.vector)
            .with_context(|| format!("serializing embedding vector for {}", embedding.chunk_id))?;
        transaction
            .execute(
                "INSERT INTO chunk_embeddings (
                    chunk_id,
                    provider,
                    dimensions,
                    content_hash,
                    vector_json
                ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    embedding.chunk_id,
                    embedding.provider,
                    embedding.dimensions as i64,
                    embedding.content_hash,
                    vector_json,
                ],
            )
            .with_context(|| format!("writing chunk embedding for {}", embedding.chunk_id))?;
    }

    for edge in &snapshot.trace_edges {
        transaction
            .execute(
                "INSERT INTO trace_edges (
                    source_kind,
                    source_id,
                    target_kind,
                    target_id,
                    edge_type,
                    status,
                    evidence_source_path,
                    evidence_detail
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    format!("{:?}", edge.source.kind).to_ascii_lowercase(),
                    edge.source.id,
                    format!("{:?}", edge.target.kind).to_ascii_lowercase(),
                    edge.target.id,
                    format!("{:?}", edge.edge_type).to_ascii_lowercase(),
                    format!("{:?}", edge.status).to_ascii_lowercase(),
                    edge.evidence.source_path,
                    edge.evidence.detail,
                ],
            )
            .with_context(|| format!("writing trace edge {} -> {}", edge.source.id, edge.target.id))?;
    }

    Ok(())
}

pub fn load_chunk_embeddings(project_root: &Path) -> Result<BTreeMap<String, ChunkEmbeddingRecord>> {
    let database = database_path(project_root);
    let connection = Connection::open(&database)
        .with_context(|| format!("opening SQLite store {}", database.display()))?;

    let mut statement = connection
        .prepare(
            "SELECT chunk_id, provider, dimensions, content_hash, vector_json
             FROM chunk_embeddings",
        )
        .context("preparing chunk_embeddings query")?;

    let rows = statement
        .query_map([], |row| {
            let vector_json: String = row.get(4)?;
            let vector: Vec<f32> = serde_json::from_str(&vector_json).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;

            Ok(ChunkEmbeddingRecord {
                chunk_id: row.get(0)?,
                provider: row.get(1)?,
                dimensions: row.get::<_, i64>(2)? as usize,
                content_hash: row.get(3)?,
                vector,
            })
        })
        .context("querying chunk_embeddings")?;

    let mut records = BTreeMap::new();
    for row in rows {
        let record = row.context("reading chunk_embeddings row")?;
        records.insert(record.chunk_id.clone(), record);
    }

    Ok(records)
}
