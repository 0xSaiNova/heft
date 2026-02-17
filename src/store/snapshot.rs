use rusqlite::{Connection, params};
use std::path::PathBuf;
use crate::scan::ScanResult;
use crate::scan::detector::{BloatEntry, BloatCategory, Location};

/// Snapshot metadata stored in database
#[derive(Debug)]
pub struct Snapshot {
    pub id: i64,
    pub timestamp: i64,
    pub total_bytes: u64,
    pub reclaimable_bytes: u64,
    pub scan_duration_ms: u64,
    pub peak_memory_bytes: Option<usize>,
}

/// Get the database path (~/.local/share/heft/heft.db or platform equivalent)
fn get_db_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let data_dir = directories::ProjectDirs::from("", "", "heft")
        .ok_or("Could not determine data directory")?
        .data_dir()
        .to_path_buf();

    std::fs::create_dir_all(&data_dir)?;
    Ok(data_dir.join("heft.db"))
}

fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            total_bytes INTEGER NOT NULL,
            reclaimable_bytes INTEGER NOT NULL,
            scan_duration_ms INTEGER NOT NULL,
            peak_memory_bytes INTEGER
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            snapshot_id INTEGER NOT NULL,
            category TEXT NOT NULL,
            name TEXT NOT NULL,
            location TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            reclaimable_bytes INTEGER NOT NULL,
            last_modified INTEGER,
            cleanup_hint TEXT,
            FOREIGN KEY(snapshot_id) REFERENCES snapshots(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_entries_snapshot_id ON entries(snapshot_id)",
        [],
    )?;

    Ok(())
}

/// Database handle. Open once per command, reuse across all operations.
pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open() -> Result<Self, Box<dyn std::error::Error>> {
        let db_path = get_db_path()?;
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        init_schema(&conn)?;
        Ok(Store { conn })
    }

    /// Save a scan result as a snapshot
    pub fn save_snapshot(&mut self, result: &ScanResult) -> Result<i64, Box<dyn std::error::Error>> {
        let (total_bytes, reclaimable_bytes) = result.entries.iter()
            .fold((0u64, 0u64), |(total, reclaimable), entry| {
                (total.saturating_add(entry.size_bytes), reclaimable.saturating_add(entry.reclaimable_bytes))
            });

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        let tx = self.conn.transaction()?;

        tx.execute(
            "INSERT INTO snapshots (timestamp, total_bytes, reclaimable_bytes, scan_duration_ms, peak_memory_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                timestamp,
                i64::try_from(total_bytes).unwrap_or(i64::MAX),
                i64::try_from(reclaimable_bytes).unwrap_or(i64::MAX),
                i64::try_from(result.duration_ms.unwrap_or(0)).unwrap_or(i64::MAX),
                result.peak_memory_bytes.map(|m| i64::try_from(m).unwrap_or(i64::MAX))
            ],
        )?;

        let snapshot_id = tx.last_insert_rowid();

        let mut stmt = tx.prepare_cached(
            "INSERT INTO entries (snapshot_id, category, name, location, size_bytes, reclaimable_bytes, last_modified, cleanup_hint)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        )?;

        for entry in &result.entries {
            let location_str = match &entry.location {
                Location::FilesystemPath(p) => p.to_string_lossy().to_string(),
                Location::DockerObject(name) => format!("docker:{name}"),
                Location::Aggregate(name) => format!("aggregate:{name}"),
            };

            stmt.execute(params![
                snapshot_id,
                entry.category.as_str(),
                entry.name,
                location_str,
                i64::try_from(entry.size_bytes).unwrap_or(i64::MAX),
                i64::try_from(entry.reclaimable_bytes).unwrap_or(i64::MAX),
                entry.last_modified,
                entry.cleanup_hint.as_deref()
            ])?;
        }

        drop(stmt);
        tx.commit()?;

        Ok(snapshot_id)
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> Result<Vec<Snapshot>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, total_bytes, reclaimable_bytes, scan_duration_ms, peak_memory_bytes
             FROM snapshots
             ORDER BY timestamp DESC"
        )?;

        let snapshots = stmt.query_map([], snapshot_from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(snapshots)
    }

    /// Get a specific snapshot by ID
    pub fn get_snapshot(&self, id: i64) -> Result<Option<Snapshot>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, total_bytes, reclaimable_bytes, scan_duration_ms, peak_memory_bytes
             FROM snapshots
             WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(snapshot_from_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// Get the most recent snapshot
    pub fn get_latest_snapshot(&self) -> Result<Option<Snapshot>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, total_bytes, reclaimable_bytes, scan_duration_ms, peak_memory_bytes
             FROM snapshots
             ORDER BY timestamp DESC
             LIMIT 1"
        )?;

        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            Ok(Some(snapshot_from_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// Load entries for a specific snapshot
    pub fn load_snapshot_entries(&self, snapshot_id: i64) -> Result<Vec<BloatEntry>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT category, name, location, size_bytes, reclaimable_bytes, last_modified, cleanup_hint
             FROM entries
             WHERE snapshot_id = ?1"
        )?;

        let entries = stmt.query_map(params![snapshot_id], |row| {
            let category_str: String = row.get(0)?;
            let location_str: String = row.get(2)?;

            let location = if let Some(docker_name) = location_str.strip_prefix("docker:") {
                Location::DockerObject(docker_name.to_string())
            } else if let Some(agg_name) = location_str.strip_prefix("aggregate:") {
                Location::Aggregate(agg_name.to_string())
            } else {
                Location::FilesystemPath(PathBuf::from(location_str))
            };

            let category = match category_str.as_str() {
                "ProjectArtifacts" => BloatCategory::ProjectArtifacts,
                "ContainerData" => BloatCategory::ContainerData,
                "PackageCache" => BloatCategory::PackageCache,
                "IdeData" => BloatCategory::IdeData,
                "SystemCache" => BloatCategory::SystemCache,
                _ => BloatCategory::Other,
            };

            Ok(BloatEntry {
                category,
                name: row.get(1)?,
                location,
                size_bytes: row.get::<_, i64>(3)?.max(0) as u64,
                reclaimable_bytes: row.get::<_, i64>(4)?.max(0) as u64,
                last_modified: row.get(5)?,
                cleanup_hint: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }
}

fn snapshot_from_row(row: &rusqlite::Row) -> rusqlite::Result<Snapshot> {
    Ok(Snapshot {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        total_bytes: row.get::<_, i64>(2)?.max(0) as u64,
        reclaimable_bytes: row.get::<_, i64>(3)?.max(0) as u64,
        scan_duration_ms: row.get::<_, i64>(4)?.max(0) as u64,
        peak_memory_bytes: row.get::<_, Option<i64>>(5)?.map(|m| m.max(0) as usize),
    })
}
