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
    pub scan_duration_ms: u128,
    pub peak_memory_bytes: Option<usize>,
}

/// Get the database path (~/.local/share/heft/heft.db or platform equivalent)
pub fn get_db_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let data_dir = directories::ProjectDirs::from("", "", "heft")
        .ok_or("Could not determine data directory")?
        .data_dir()
        .to_path_buf();

    std::fs::create_dir_all(&data_dir)?;
    Ok(data_dir.join("heft.db"))
}

/// Initialize database schema
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
            FOREIGN KEY(snapshot_id) REFERENCES snapshots(id)
        )",
        [],
    )?;

    // Index for faster queries
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_entries_snapshot_id ON entries(snapshot_id)",
        [],
    )?;

    Ok(())
}

/// Open database connection and ensure schema exists
pub fn open_db() -> Result<Connection, Box<dyn std::error::Error>> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;
    init_schema(&conn)?;
    Ok(conn)
}

/// Save a scan result as a snapshot
pub fn save_snapshot(result: &ScanResult) -> Result<i64, Box<dyn std::error::Error>> {
    let conn = open_db()?;

    // Calculate totals
    let total_bytes: u64 = result.entries.iter().map(|e| e.size_bytes).sum();
    let reclaimable_bytes: u64 = result.entries.iter().map(|e| e.reclaimable_bytes).sum();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    // Insert snapshot
    conn.execute(
        "INSERT INTO snapshots (timestamp, total_bytes, reclaimable_bytes, scan_duration_ms, peak_memory_bytes)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            timestamp,
            total_bytes as i64,
            reclaimable_bytes as i64,
            result.duration_ms.unwrap_or(0) as i64,
            result.peak_memory_bytes.map(|m| m as i64)
        ],
    )?;

    let snapshot_id = conn.last_insert_rowid();

    // Insert entries
    for entry in &result.entries {
        let location_str = match &entry.location {
            Location::FilesystemPath(p) => p.to_string_lossy().to_string(),
            Location::DockerObject(name) => format!("docker:{name}"),
            Location::Aggregate(name) => format!("aggregate:{name}"),
        };

        conn.execute(
            "INSERT INTO entries (snapshot_id, category, name, location, size_bytes, reclaimable_bytes, last_modified, cleanup_hint)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                snapshot_id,
                entry.category.as_str(),
                entry.name,
                location_str,
                entry.size_bytes as i64,
                entry.reclaimable_bytes as i64,
                entry.last_modified,
                entry.cleanup_hint.as_deref()
            ],
        )?;
    }

    Ok(snapshot_id)
}

/// List all snapshots
pub fn list_snapshots() -> Result<Vec<Snapshot>, Box<dyn std::error::Error>> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, total_bytes, reclaimable_bytes, scan_duration_ms, peak_memory_bytes
         FROM snapshots
         ORDER BY timestamp DESC"
    )?;

    let snapshots = stmt.query_map([], |row| {
        Ok(Snapshot {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            total_bytes: row.get::<_, i64>(2)? as u64,
            reclaimable_bytes: row.get::<_, i64>(3)? as u64,
            scan_duration_ms: row.get::<_, i64>(4)? as u128,
            peak_memory_bytes: row.get::<_, Option<i64>>(5)?.map(|m| m as usize),
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(snapshots)
}

/// Get a specific snapshot by ID
pub fn get_snapshot(id: i64) -> Result<Option<Snapshot>, Box<dyn std::error::Error>> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, total_bytes, reclaimable_bytes, scan_duration_ms, peak_memory_bytes
         FROM snapshots
         WHERE id = ?1"
    )?;

    let mut rows = stmt.query(params![id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(Snapshot {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            total_bytes: row.get::<_, i64>(2)? as u64,
            reclaimable_bytes: row.get::<_, i64>(3)? as u64,
            scan_duration_ms: row.get::<_, i64>(4)? as u128,
            peak_memory_bytes: row.get::<_, Option<i64>>(5)?.map(|m| m as usize),
        }))
    } else {
        Ok(None)
    }
}

/// Get the most recent snapshot
pub fn get_latest_snapshot() -> Result<Option<Snapshot>, Box<dyn std::error::Error>> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT id, timestamp, total_bytes, reclaimable_bytes, scan_duration_ms, peak_memory_bytes
         FROM snapshots
         ORDER BY timestamp DESC
         LIMIT 1"
    )?;

    let mut rows = stmt.query([])?;

    if let Some(row) = rows.next()? {
        Ok(Some(Snapshot {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            total_bytes: row.get::<_, i64>(2)? as u64,
            reclaimable_bytes: row.get::<_, i64>(3)? as u64,
            scan_duration_ms: row.get::<_, i64>(4)? as u128,
            peak_memory_bytes: row.get::<_, Option<i64>>(5)?.map(|m| m as usize),
        }))
    } else {
        Ok(None)
    }
}

/// Load entries for a specific snapshot
pub fn load_snapshot_entries(snapshot_id: i64) -> Result<Vec<BloatEntry>, Box<dyn std::error::Error>> {
    let conn = open_db()?;
    let mut stmt = conn.prepare(
        "SELECT category, name, location, size_bytes, reclaimable_bytes, last_modified, cleanup_hint
         FROM entries
         WHERE snapshot_id = ?1"
    )?;

    let entries = stmt.query_map(params![snapshot_id], |row| {
        let category_str: String = row.get(0)?;
        let location_str: String = row.get(2)?;

        // Parse location
        let location = if let Some(docker_name) = location_str.strip_prefix("docker:") {
            Location::DockerObject(docker_name.to_string())
        } else if let Some(agg_name) = location_str.strip_prefix("aggregate:") {
            Location::Aggregate(agg_name.to_string())
        } else {
            Location::FilesystemPath(PathBuf::from(location_str))
        };

        // Parse category
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
            size_bytes: row.get::<_, i64>(3)? as u64,
            reclaimable_bytes: row.get::<_, i64>(4)? as u64,
            last_modified: row.get(5)?,
            cleanup_hint: row.get(6)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(entries)
}
