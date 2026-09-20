use crate::{
    AssetId, EntryId, Error, ErrorKind, HistoryEntry, Layer, Mutation, PreviewJob, Raster,
    Snapshot, SnapshotId, SourceImage, Transform, open_source, render, sample,
};
use rusqlite::{
    Connection, ErrorCode, OptionalExtension, Transaction, TransactionBehavior, params,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    fs::Metadata,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const CATALOG_FORMAT: i64 = 2;
const MAX_HISTORY_PAGE: usize = 100;
const MAX_VERSION_NAME: usize = 64;
const ASSET_COLUMNS: &str =
    "id,source_root,locator,fingerprint,file_identity,byte_len,width,height";

fn catalog_error(error: rusqlite::Error) -> Error {
    let kind = match &error {
        rusqlite::Error::SqliteFailure(problem, _)
            if matches!(
                problem.code,
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
            ) =>
        {
            ErrorKind::Conflict
        }
        _ => ErrorKind::Catalog,
    };
    Error::new(kind, error.to_string())
}

fn json_error(context: &str, error: impl std::fmt::Display) -> Error {
    Error::new(ErrorKind::Incompatible, format!("{context}: {error}"))
}

fn encode<T: Serialize>(value: &T) -> Result<String, Error> {
    serde_json::to_string(value).map_err(|e| json_error("cannot encode catalog value", e))
}

fn decode<T: DeserializeOwned>(context: &str, value: String) -> Result<T, Error> {
    serde_json::from_str(&value).map_err(|e| json_error(context, e))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRecord {
    pub id: AssetId,
    pub source_root: PathBuf,
    pub locator: PathBuf,
    pub fingerprint: String,
    pub file_identity: String,
    pub byte_len: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorState {
    pub asset: AssetRecord,
    pub revision: u64,
    pub current_entry: HistoryEntry,
    pub redo: Vec<EntryId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MutationOutcome {
    Applied,
    Navigated,
    NoOp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationResult {
    pub outcome: MutationOutcome,
    pub revision: u64,
    pub current_entry_id: EntryId,
    pub created_entry_id: Option<EntryId>,
    pub deduplicated: bool,
}

/// One evaluated output pixel of a saved history entry, with the identities that produced it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PixelSample {
    pub entry_id: EntryId,
    pub snapshot_id: SnapshotId,
    pub source_fingerprint: String,
    pub width: u32,
    pub height: u32,
    pub x: u32,
    pub y: u32,
    pub rgba: [u8; 4],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub next_before_sequence: Option<u64>,
}

/// One step on the undo-parent chain, without the entry's full stack.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LineageStep {
    pub entry_id: EntryId,
    pub sequence: u64,
    pub action_id: String,
    pub undo_parent: Option<EntryId>,
}

/// The chain of undo parents from one entry back towards Original, newest first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lineage {
    pub steps: Vec<LineageStep>,
    /// The next parent to continue from when the page limit stopped the walk.
    pub next_entry_id: Option<EntryId>,
}

/// A named reference to one retained history entry: the Lightroom-style saved state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    pub asset_id: AssetId,
    pub name: String,
    pub entry_id: EntryId,
    pub entry_sequence: u64,
    pub actor: String,
    pub created_ms: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionResult {
    pub outcome: MutationOutcome,
    pub version: Option<Version>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceSignature {
    byte_len: u64,
    modified: Option<SystemTime>,
    file_identity: String,
    change_marker: Option<(i128, i128)>,
}

#[derive(Clone, Debug)]
struct CachedSource {
    asset_id: AssetId,
    signature: SourceSignature,
    source: SourceImage,
}

#[derive(Debug)]
pub struct EditorService {
    connection: Connection,
    source_cache: RefCell<Option<CachedSource>>,
}

impl EditorService {
    pub fn open(path: &Path) -> Result<Self, Error> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::new(
                    ErrorKind::Catalog,
                    format!("cannot create catalog directory: {}", e.kind()),
                )
            })?;
        }
        let connection = Connection::open(path).map_err(catalog_error)?;
        connection
            .busy_timeout(Duration::from_millis(100))
            .map_err(catalog_error)?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys=ON;
                 PRAGMA synchronous=FULL;
                 PRAGMA locking_mode=EXCLUSIVE;
                 BEGIN IMMEDIATE;
                 COMMIT;",
            )
            .map_err(catalog_error)?;
        let version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(catalog_error)?;
        match version {
            0 => Self::create_schema(&connection)?,
            1 => Self::convert_format_1(&connection)?,
            CATALOG_FORMAT => {}
            other => {
                return Err(Error::new(
                    ErrorKind::Incompatible,
                    format!("catalog format {other} is not supported; expected {CATALOG_FORMAT}"),
                ));
            }
        }
        Ok(Self {
            connection,
            source_cache: RefCell::new(None),
        })
    }

    fn create_schema(connection: &Connection) -> Result<(), Error> {
        connection
            .execute_batch(
                "BEGIN IMMEDIATE;
                 CREATE TABLE assets (
                    id TEXT PRIMARY KEY,
                    source_root TEXT NOT NULL,
                    locator TEXT NOT NULL,
                    canonical_locator TEXT NOT NULL UNIQUE,
                    file_identity TEXT NOT NULL UNIQUE,
                    fingerprint TEXT NOT NULL,
                    byte_len INTEGER NOT NULL,
                    width INTEGER NOT NULL,
                    height INTEGER NOT NULL
                 );
                 CREATE TABLE entries (
                    id TEXT PRIMARY KEY,
                    asset_id TEXT NOT NULL REFERENCES assets(id),
                    sequence INTEGER NOT NULL,
                    action_id TEXT NOT NULL,
                    undo_parent_id TEXT,
                    entry_json TEXT NOT NULL,
                    UNIQUE(asset_id, sequence)
                 );
                 CREATE TABLE asset_state (
                    asset_id TEXT PRIMARY KEY REFERENCES assets(id),
                    current_entry_id TEXT NOT NULL REFERENCES entries(id),
                    revision INTEGER NOT NULL,
                    redo_json TEXT NOT NULL
                 );
                 CREATE TABLE requests (
                    asset_id TEXT NOT NULL REFERENCES assets(id),
                    request_id TEXT NOT NULL,
                    input_hash TEXT NOT NULL,
                    result_json TEXT NOT NULL,
                    PRIMARY KEY(asset_id, request_id)
                 );
                 CREATE TABLE versions (
                    asset_id TEXT NOT NULL REFERENCES assets(id),
                    name TEXT NOT NULL COLLATE NOCASE,
                    entry_id TEXT NOT NULL REFERENCES entries(id),
                    actor TEXT NOT NULL,
                    created_ms INTEGER NOT NULL,
                    PRIMARY KEY(asset_id, name)
                 );
                 CREATE TRIGGER entries_are_immutable BEFORE UPDATE ON entries BEGIN
                    SELECT RAISE(ABORT, 'history entries are immutable');
                 END;
                 PRAGMA user_version=2;
                 COMMIT;",
            )
            .map_err(catalog_error)
    }

    /// Format 1 kept two unread copies of every stack beside the authoritative entry JSON.
    /// Format 2 drops them, adds the undo-parent column and the versions table.
    fn convert_format_1(connection: &Connection) -> Result<(), Error> {
        let orphaned: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM entries e WHERE NOT EXISTS (
                    SELECT 1 FROM snapshots s WHERE s.id = json_extract(e.entry_json, '$.snapshot.id'))",
                [],
                |row| row.get(0),
            )
            .map_err(catalog_error)?;
        if orphaned != 0 {
            return Err(Error::new(
                ErrorKind::Incompatible,
                format!(
                    "catalog format 1 has {orphaned} entries without snapshots; not converting"
                ),
            ));
        }
        connection
            .execute_batch(
                "BEGIN IMMEDIATE;
                 DROP TRIGGER entries_are_immutable;
                 ALTER TABLE entries ADD COLUMN undo_parent_id TEXT;
                 UPDATE entries SET undo_parent_id = json_extract(entry_json, '$.undo_parent');
                 CREATE TRIGGER entries_are_immutable BEFORE UPDATE ON entries BEGIN
                    SELECT RAISE(ABORT, 'history entries are immutable');
                 END;
                 DROP TRIGGER snapshots_are_immutable;
                 DROP TABLE snapshot_layers;
                 DROP TABLE snapshots;
                 CREATE TABLE versions (
                    asset_id TEXT NOT NULL REFERENCES assets(id),
                    name TEXT NOT NULL COLLATE NOCASE,
                    entry_id TEXT NOT NULL REFERENCES entries(id),
                    actor TEXT NOT NULL,
                    created_ms INTEGER NOT NULL,
                    PRIMARY KEY(asset_id, name)
                 );
                 PRAGMA user_version=2;
                 COMMIT;",
            )
            .map_err(catalog_error)
    }

    pub fn import(&mut self, path: &Path) -> Result<EditorState, Error> {
        let canonical = path.canonicalize().map_err(|e| {
            Error::new(
                ErrorKind::FileAccess,
                format!("cannot resolve source: {}", e.kind()),
            )
        })?;
        let before = canonical
            .metadata()
            .map_err(|e| Error::new(ErrorKind::FileAccess, e.to_string()))?;
        let identity = file_identity(&before, &canonical);
        let canonical_text = canonical.to_string_lossy().into_owned();
        if let Some(existing) = self
            .connection
            .query_row(
                "SELECT id FROM assets WHERE canonical_locator=?1 OR file_identity=?2 LIMIT 1",
                params![canonical_text, identity],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(catalog_error)?
        {
            return self.state(&AssetId::parse(existing)?);
        }
        let source = open_source(&canonical)?;
        let after = canonical
            .metadata()
            .map_err(|e| Error::new(ErrorKind::FileAccess, e.to_string()))?;
        if source_signature(&canonical, &before) != source_signature(&canonical, &after) {
            return Err(Error::new(
                ErrorKind::Conflict,
                "source changed while it was being imported",
            ));
        }
        let asset = AssetRecord {
            id: AssetId::new(),
            source_root: canonical.parent().unwrap_or(Path::new("")).to_path_buf(),
            locator: canonical.clone(),
            fingerprint: source.fingerprint.clone(),
            file_identity: identity,
            byte_len: after.len(),
            width: source.width,
            height: source.height,
        };
        let snapshot = Snapshot::original(asset.id.clone());
        let entry = HistoryEntry {
            id: EntryId::new(),
            asset_id: asset.id.clone(),
            sequence: 0,
            action_id: "original".into(),
            parameters: json!({}),
            actor: "system".into(),
            timestamp_ms: now_ms(),
            request_id: None,
            base_revision: 0,
            result_revision: 0,
            snapshot,
            undo_parent: None,
            restore_target: None,
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(catalog_error)?;
        tx.execute(
            "INSERT INTO assets VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                asset.id.as_str(),
                asset.source_root.to_string_lossy(),
                asset.locator.to_string_lossy(),
                canonical_text,
                asset.file_identity,
                asset.fingerprint,
                i64::try_from(asset.byte_len).map_err(|_| Error::new(
                    ErrorKind::ResourceLimit,
                    "source length exceeds catalog range"
                ))?,
                i64::from(asset.width),
                i64::from(asset.height),
            ],
        )
        .map_err(catalog_error)?;
        insert_entry(&tx, &entry)?;
        tx.execute(
            "INSERT INTO asset_state VALUES (?1,?2,0,'[]')",
            params![asset.id.as_str(), entry.id.as_str()],
        )
        .map_err(catalog_error)?;
        tx.commit().map_err(catalog_error)?;
        self.source_cache.replace(Some(CachedSource {
            asset_id: asset.id.clone(),
            signature: source_signature(&canonical, &after),
            source,
        }));
        Ok(EditorState {
            asset,
            revision: 0,
            current_entry: entry,
            redo: Vec::new(),
        })
    }

    pub fn state(&self, asset_id: &AssetId) -> Result<EditorState, Error> {
        state_from(&self.connection, asset_id)
    }

    /// Every referenced asset in import order.
    pub fn assets(&self) -> Result<Vec<AssetRecord>, Error> {
        let mut statement = self
            .connection
            .prepare(&format!(
                "SELECT {ASSET_COLUMNS} FROM assets ORDER BY rowid"
            ))
            .map_err(catalog_error)?;
        let rows = statement
            .query_map([], asset_record)
            .map_err(catalog_error)?;
        rows.map(|row| row.map_err(catalog_error)).collect()
    }

    pub fn history(
        &self,
        asset_id: &AssetId,
        before_sequence: Option<u64>,
        limit: usize,
    ) -> Result<HistoryPage, Error> {
        if limit == 0 || limit > MAX_HISTORY_PAGE {
            return Err(Error::new(
                ErrorKind::Validation,
                format!("history page limit must be 1..={MAX_HISTORY_PAGE}"),
            ));
        }
        let before = before_sequence
            .unwrap_or(i64::MAX as u64)
            .min(i64::MAX as u64) as i64;
        let mut statement = self
            .connection
            .prepare("SELECT entry_json FROM entries WHERE asset_id=?1 AND sequence<?2 ORDER BY sequence DESC LIMIT ?3")
            .map_err(catalog_error)?;
        let rows = statement
            .query_map(params![asset_id.as_str(), before, limit as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(catalog_error)?;
        let mut entries: Vec<HistoryEntry> = Vec::new();
        for row in rows {
            entries.push(decode(
                "invalid history entry",
                row.map_err(catalog_error)?,
            )?);
        }
        let next_before_sequence =
            (entries.len() == limit).then(|| entries.last().unwrap().sequence);
        Ok(HistoryPage {
            entries,
            next_before_sequence,
        })
    }

    pub fn entry(&self, asset_id: &AssetId, entry_id: &EntryId) -> Result<HistoryEntry, Error> {
        entry_from(&self.connection, asset_id, entry_id)
    }

    pub fn render_current(&self, asset_id: &AssetId) -> Result<Raster, Error> {
        let state = self.state(asset_id)?;
        self.render_entry(asset_id, &state.current_entry.id)
    }

    pub fn preview_job(
        &self,
        asset_id: &AssetId,
        entry_id: Option<&EntryId>,
    ) -> Result<PreviewJob, Error> {
        let state = self.state(asset_id)?;
        let entry = match entry_id {
            Some(entry_id) => self.entry(asset_id, entry_id)?,
            None => state.current_entry,
        };
        Ok(PreviewJob {
            source: self.verified_source(&state.asset)?,
            entry,
        })
    }

    pub fn render_entry(&self, asset_id: &AssetId, entry_id: &EntryId) -> Result<Raster, Error> {
        let state = self.state(asset_id)?;
        let entry = self.entry(asset_id, entry_id)?;
        let source = self.verified_source(&state.asset)?;
        render(&source, entry.snapshot.id.clone(), &entry.snapshot.recipe)
    }

    /// Evaluate one output pixel of a saved entry without rasterizing the image.
    pub fn sample_entry(
        &self,
        asset_id: &AssetId,
        entry_id: &EntryId,
        x: u32,
        y: u32,
    ) -> Result<PixelSample, Error> {
        let state = self.state(asset_id)?;
        let entry = self.entry(asset_id, entry_id)?;
        let source = self.verified_source(&state.asset)?;
        let sampled = sample(&source, &entry.snapshot.recipe, x, y)?;
        let rgba = sampled.rgba.ok_or_else(|| {
            Error::new(
                ErrorKind::Validation,
                format!(
                    "sample ({x}, {y}) is outside the {}x{} rendered image",
                    sampled.width, sampled.height
                ),
            )
        })?;
        Ok(PixelSample {
            entry_id: entry.id,
            snapshot_id: entry.snapshot.id,
            source_fingerprint: source.fingerprint,
            width: sampled.width,
            height: sampled.height,
            x,
            y,
            rgba,
        })
    }

    pub fn apply_pixel(
        &mut self,
        asset_id: &AssetId,
        mutation: Mutation,
        x: u32,
        y: u32,
        rgb: [u8; 3],
    ) -> Result<MutationResult, Error> {
        mutation.validate()?;
        let input = json!({"action":"set-pixel","mutation":mutation,"x":x,"y":y,"rgb":rgb});
        if let Some(result) = self.request_result(asset_id, &mutation.request_id, &input)? {
            return Ok(result);
        }
        let state = self.state(asset_id)?;
        ensure_revision(&state, mutation.expected_revision)?;
        let source = self.verified_source(&state.asset)?;
        let current = sample(&source, &state.current_entry.snapshot.recipe, x, y)?;
        let rgba = current.rgba.ok_or_else(|| {
            Error::new(
                ErrorKind::Validation,
                format!(
                    "pixel ({x}, {y}) is outside {}x{} input stage",
                    current.width, current.height
                ),
            )
        })?;
        if rgba[..3] == rgb {
            return self.persist_noop(asset_id, &mutation, &input, &state);
        }
        self.commit_layer(
            asset_id,
            mutation,
            input,
            Layer::pixel(x, y, rgb),
            "set-pixel",
            json!({"x":x,"y":y,"rgb":rgb}),
        )
    }

    fn verified_source(&self, asset: &AssetRecord) -> Result<SourceImage, Error> {
        let before = asset.locator.metadata().map_err(|_| {
            Error::new(
                ErrorKind::SourceUnavailable,
                "original source is unavailable",
            )
        })?;
        let signature = source_signature(&asset.locator, &before);
        if signature.byte_len != asset.byte_len
            || signature.byte_len > 128 * 1024 * 1024
            || signature.file_identity != asset.file_identity
        {
            return Err(Error::new(
                ErrorKind::SourceUnavailable,
                "original source fingerprint changed",
            ));
        }
        if let Some(cached) = self.source_cache.borrow().as_ref()
            && cached.asset_id == asset.id
            && cached.signature == signature
        {
            return Ok(cached.source.clone());
        }

        let source = open_source(&asset.locator).map_err(|_| {
            Error::new(
                ErrorKind::SourceUnavailable,
                "original source is unavailable or changed",
            )
        })?;
        let after = asset.locator.metadata().map_err(|_| {
            Error::new(
                ErrorKind::SourceUnavailable,
                "original source is unavailable",
            )
        })?;
        let after_signature = source_signature(&asset.locator, &after);
        if after_signature != signature || source.fingerprint != asset.fingerprint {
            return Err(Error::new(
                ErrorKind::SourceUnavailable,
                "original source fingerprint changed",
            ));
        }
        self.source_cache.replace(Some(CachedSource {
            asset_id: asset.id.clone(),
            signature,
            source: source.clone(),
        }));
        Ok(source)
    }

    pub fn apply_transform(
        &mut self,
        asset_id: &AssetId,
        mutation: Mutation,
        transform: Transform,
    ) -> Result<MutationResult, Error> {
        mutation.validate()?;
        let input = json!({"action":transform.action_id(),"mutation":mutation});
        if let Some(result) = self.request_result(asset_id, &mutation.request_id, &input)? {
            return Ok(result);
        }
        self.commit_layer(
            asset_id,
            mutation,
            input,
            Layer::transform(transform),
            transform.action_id(),
            json!({"transform":transform}),
        )
    }

    fn commit_layer(
        &mut self,
        asset_id: &AssetId,
        mutation: Mutation,
        input: Value,
        layer: Layer,
        action_id: &str,
        parameters: Value,
    ) -> Result<MutationResult, Error> {
        let mut state = self.state(asset_id)?;
        ensure_revision(&state, mutation.expected_revision)?;
        let snapshot = state.current_entry.snapshot.append(layer)?;
        let entry = HistoryEntry {
            id: EntryId::new(),
            asset_id: asset_id.clone(),
            sequence: next_sequence(&self.connection, asset_id)?,
            action_id: action_id.into(),
            parameters,
            actor: mutation.actor.clone(),
            timestamp_ms: now_ms(),
            request_id: Some(mutation.request_id.clone()),
            base_revision: state.revision,
            result_revision: state.revision + 1,
            snapshot,
            undo_parent: Some(state.current_entry.id.clone()),
            restore_target: None,
        };
        let result = MutationResult {
            outcome: MutationOutcome::Applied,
            revision: entry.result_revision,
            current_entry_id: entry.id.clone(),
            created_entry_id: Some(entry.id.clone()),
            deduplicated: false,
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(catalog_error)?;
        ensure_request_absent(&tx, asset_id, &mutation.request_id)?;
        insert_entry(&tx, &entry)?;
        tx.execute("UPDATE asset_state SET current_entry_id=?2,revision=?3,redo_json='[]' WHERE asset_id=?1", params![asset_id.as_str(), entry.id.as_str(), entry.result_revision as i64]).map_err(catalog_error)?;
        insert_request(&tx, asset_id, &mutation.request_id, &input, &result)?;
        tx.commit().map_err(catalog_error)?;
        state.current_entry = entry;
        Ok(result)
    }

    pub fn undo(
        &mut self,
        asset_id: &AssetId,
        mutation: Mutation,
    ) -> Result<MutationResult, Error> {
        self.navigate(asset_id, mutation, Navigation::Undo)
    }

    pub fn redo(
        &mut self,
        asset_id: &AssetId,
        mutation: Mutation,
    ) -> Result<MutationResult, Error> {
        self.navigate(asset_id, mutation, Navigation::Redo)
    }

    fn navigate(
        &mut self,
        asset_id: &AssetId,
        mutation: Mutation,
        navigation: Navigation,
    ) -> Result<MutationResult, Error> {
        mutation.validate()?;
        let action = match navigation {
            Navigation::Undo => "undo",
            Navigation::Redo => "redo",
        };
        let input = json!({"action":action,"mutation":mutation});
        if let Some(result) = self.request_result(asset_id, &mutation.request_id, &input)? {
            return Ok(result);
        }
        let state = self.state(asset_id)?;
        ensure_revision(&state, mutation.expected_revision)?;
        let mut redo = state.redo.clone();
        let target = match navigation {
            Navigation::Undo => {
                let Some(parent) = state.current_entry.undo_parent.clone() else {
                    return self.persist_noop(asset_id, &mutation, &input, &state);
                };
                redo.push(state.current_entry.id.clone());
                parent
            }
            Navigation::Redo => {
                let Some(entry) = redo.pop() else {
                    return self.persist_noop(asset_id, &mutation, &input, &state);
                };
                entry
            }
        };
        let _ = self.entry(asset_id, &target)?;
        let result = MutationResult {
            outcome: MutationOutcome::Navigated,
            revision: state.revision + 1,
            current_entry_id: target.clone(),
            created_entry_id: None,
            deduplicated: false,
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(catalog_error)?;
        tx.execute(
            "UPDATE asset_state SET current_entry_id=?2,revision=?3,redo_json=?4 WHERE asset_id=?1",
            params![
                asset_id.as_str(),
                target.as_str(),
                result.revision as i64,
                encode(&redo)?
            ],
        )
        .map_err(catalog_error)?;
        insert_request(&tx, asset_id, &mutation.request_id, &input, &result)?;
        tx.commit().map_err(catalog_error)?;
        Ok(result)
    }

    pub fn restore(
        &mut self,
        asset_id: &AssetId,
        mutation: Mutation,
        target_id: &EntryId,
    ) -> Result<MutationResult, Error> {
        mutation.validate()?;
        let input = json!({"action":"restore","mutation":mutation,"target_entry_id":target_id});
        if let Some(result) = self.request_result(asset_id, &mutation.request_id, &input)? {
            return Ok(result);
        }
        let state = self.state(asset_id)?;
        ensure_revision(&state, mutation.expected_revision)?;
        let target = self.entry(asset_id, target_id)?;
        if target.snapshot.recipe == state.current_entry.snapshot.recipe {
            return self.persist_noop(asset_id, &mutation, &input, &state);
        }
        let snapshot = Snapshot {
            id: SnapshotId::new(),
            asset_id: asset_id.clone(),
            recipe: target.snapshot.recipe.clone(),
        };
        let entry = HistoryEntry {
            id: EntryId::new(),
            asset_id: asset_id.clone(),
            sequence: next_sequence(&self.connection, asset_id)?,
            action_id: "restore".into(),
            parameters: json!({"target_entry_id":target_id}),
            actor: mutation.actor.clone(),
            timestamp_ms: now_ms(),
            request_id: Some(mutation.request_id.clone()),
            base_revision: state.revision,
            result_revision: state.revision + 1,
            snapshot,
            undo_parent: Some(state.current_entry.id.clone()),
            restore_target: Some(target_id.clone()),
        };
        let result = MutationResult {
            outcome: MutationOutcome::Applied,
            revision: entry.result_revision,
            current_entry_id: entry.id.clone(),
            created_entry_id: Some(entry.id.clone()),
            deduplicated: false,
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(catalog_error)?;
        insert_entry(&tx, &entry)?;
        tx.execute("UPDATE asset_state SET current_entry_id=?2,revision=?3,redo_json='[]' WHERE asset_id=?1", params![asset_id.as_str(), entry.id.as_str(), entry.result_revision as i64]).map_err(catalog_error)?;
        insert_request(&tx, asset_id, &mutation.request_id, &input, &result)?;
        tx.commit().map_err(catalog_error)?;
        Ok(result)
    }

    /// Walk undo parents from `from` (default: current) towards Original, newest first.
    pub fn lineage(
        &self,
        asset_id: &AssetId,
        from: Option<&EntryId>,
        limit: usize,
    ) -> Result<Lineage, Error> {
        if limit == 0 || limit > MAX_HISTORY_PAGE {
            return Err(Error::new(
                ErrorKind::Validation,
                format!("lineage limit must be 1..={MAX_HISTORY_PAGE}"),
            ));
        }
        let mut next = Some(match from {
            Some(entry_id) => entry_id.clone(),
            None => self.state(asset_id)?.current_entry.id,
        });
        let mut steps = Vec::new();
        while let Some(entry_id) = next.take() {
            if steps.len() == limit {
                next = Some(entry_id);
                break;
            }
            let (sequence, action_id, parent): (i64, String, Option<String>) = self
                .connection
                .query_row(
                    "SELECT sequence,action_id,undo_parent_id FROM entries WHERE id=?1 AND asset_id=?2",
                    params![entry_id.as_str(), asset_id.as_str()],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(catalog_error)?
                .ok_or_else(|| {
                    Error::new(
                        ErrorKind::Validation,
                        "history entry does not belong to this asset",
                    )
                })?;
            let undo_parent = parent.map(EntryId::parse).transpose()?;
            next = undo_parent.clone();
            steps.push(LineageStep {
                entry_id,
                sequence: u64::try_from(sequence)
                    .map_err(|_| Error::new(ErrorKind::Catalog, "invalid history sequence"))?,
                action_id,
                undo_parent,
            });
        }
        Ok(Lineage {
            steps,
            next_entry_id: next,
        })
    }

    /// Saved versions of one asset in creation order.
    pub fn versions(&self, asset_id: &AssetId) -> Result<Vec<Version>, Error> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT v.name,v.entry_id,e.sequence,v.actor,v.created_ms FROM versions v
                 JOIN entries e ON e.id = v.entry_id
                 WHERE v.asset_id=?1 ORDER BY v.created_ms, v.name",
            )
            .map_err(catalog_error)?;
        let rows = statement
            .query_map([asset_id.as_str()], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(catalog_error)?;
        let mut versions = Vec::new();
        for row in rows {
            let (name, entry_id, sequence, actor, created_ms) = row.map_err(catalog_error)?;
            versions.push(Version {
                asset_id: asset_id.clone(),
                name,
                entry_id: EntryId::parse(entry_id)?,
                entry_sequence: u64::try_from(sequence)
                    .map_err(|_| Error::new(ErrorKind::Catalog, "invalid history sequence"))?,
                actor,
                created_ms,
            });
        }
        Ok(versions)
    }

    /// Name a retained entry (default: current). Re-creating the same name on the same entry is a no-op;
    /// on a different entry it is a conflict. Names are unique per asset ignoring case.
    pub fn create_version(
        &mut self,
        asset_id: &AssetId,
        name: &str,
        entry_id: Option<&EntryId>,
        actor: &str,
    ) -> Result<VersionResult, Error> {
        let name = valid_version_name(name)?;
        if actor.is_empty() || actor.len() > 128 {
            return Err(Error::new(
                ErrorKind::Validation,
                "actor must contain 1..128 characters",
            ));
        }
        let entry = match entry_id {
            Some(entry_id) => self.entry(asset_id, entry_id)?,
            None => self.state(asset_id)?.current_entry,
        };
        if let Some(existing) = self
            .versions(asset_id)?
            .into_iter()
            .find(|version| version.name.eq_ignore_ascii_case(&name))
        {
            if existing.entry_id == entry.id {
                return Ok(VersionResult {
                    outcome: MutationOutcome::NoOp,
                    version: Some(existing),
                });
            }
            return Err(Error::new(
                ErrorKind::Conflict,
                format!("version {:?} already names another entry", existing.name),
            ));
        }
        let version = Version {
            asset_id: asset_id.clone(),
            name,
            entry_id: entry.id,
            entry_sequence: entry.sequence,
            actor: actor.into(),
            created_ms: now_ms(),
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(catalog_error)?;
        tx.execute(
            "INSERT INTO versions VALUES (?1,?2,?3,?4,?5)",
            params![
                asset_id.as_str(),
                version.name,
                version.entry_id.as_str(),
                version.actor,
                version.created_ms
            ],
        )
        .map_err(catalog_error)?;
        tx.commit().map_err(catalog_error)?;
        Ok(VersionResult {
            outcome: MutationOutcome::Applied,
            version: Some(version),
        })
    }

    /// Remove a version name. The entry it named is retained history and stays reachable.
    pub fn delete_version(
        &mut self,
        asset_id: &AssetId,
        name: &str,
    ) -> Result<VersionResult, Error> {
        let name = valid_version_name(name)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(catalog_error)?;
        let removed = tx
            .execute(
                "DELETE FROM versions WHERE asset_id=?1 AND name=?2",
                params![asset_id.as_str(), name],
            )
            .map_err(catalog_error)?;
        tx.commit().map_err(catalog_error)?;
        Ok(VersionResult {
            outcome: if removed == 0 {
                MutationOutcome::NoOp
            } else {
                MutationOutcome::Applied
            },
            version: None,
        })
    }

    fn persist_noop(
        &mut self,
        asset_id: &AssetId,
        mutation: &Mutation,
        input: &Value,
        state: &EditorState,
    ) -> Result<MutationResult, Error> {
        let result = MutationResult {
            outcome: MutationOutcome::NoOp,
            revision: state.revision,
            current_entry_id: state.current_entry.id.clone(),
            created_entry_id: None,
            deduplicated: false,
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(catalog_error)?;
        insert_request(&tx, asset_id, &mutation.request_id, input, &result)?;
        tx.commit().map_err(catalog_error)?;
        Ok(result)
    }

    fn request_result(
        &self,
        asset_id: &AssetId,
        request_id: &str,
        input: &Value,
    ) -> Result<Option<MutationResult>, Error> {
        let found = self
            .connection
            .query_row(
                "SELECT input_hash,result_json FROM requests WHERE asset_id=?1 AND request_id=?2",
                params![asset_id.as_str(), request_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(catalog_error)?;
        let Some((stored_hash, result_json)) = found else {
            return Ok(None);
        };
        if stored_hash != input_hash(input)? {
            return Err(Error::new(
                ErrorKind::Conflict,
                "request_id was already used with different input",
            ));
        }
        let mut result: MutationResult = decode("invalid saved request result", result_json)?;
        result.deduplicated = true;
        Ok(Some(result))
    }
}

enum Navigation {
    Undo,
    Redo,
}

fn ensure_revision(state: &EditorState, expected: u64) -> Result<(), Error> {
    if state.revision == expected {
        Ok(())
    } else {
        Err(Error::new(
            ErrorKind::Conflict,
            format!(
                "stale revision {expected}; current revision is {}",
                state.revision
            ),
        ))
    }
}

fn input_hash(input: &Value) -> Result<String, Error> {
    Ok(format!(
        "{:x}",
        Sha256::digest(
            serde_json::to_vec(input).map_err(|e| json_error("cannot encode request", e))?
        )
    ))
}

fn insert_request(
    tx: &Transaction<'_>,
    asset_id: &AssetId,
    request_id: &str,
    input: &Value,
    result: &MutationResult,
) -> Result<(), Error> {
    tx.execute(
        "INSERT INTO requests VALUES (?1,?2,?3,?4)",
        params![
            asset_id.as_str(),
            request_id,
            input_hash(input)?,
            encode(result)?
        ],
    )
    .map_err(catalog_error)?;
    Ok(())
}

fn ensure_request_absent(
    tx: &Transaction<'_>,
    asset_id: &AssetId,
    request_id: &str,
) -> Result<(), Error> {
    let found: bool = tx
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM requests WHERE asset_id=?1 AND request_id=?2)",
            params![asset_id.as_str(), request_id],
            |row| row.get(0),
        )
        .map_err(catalog_error)?;
    if found {
        Err(Error::new(
            ErrorKind::Conflict,
            "request was committed concurrently",
        ))
    } else {
        Ok(())
    }
}

fn valid_version_name(name: &str) -> Result<String, Error> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > MAX_VERSION_NAME
        || name.chars().any(char::is_control)
    {
        return Err(Error::new(
            ErrorKind::Validation,
            format!("version name must contain 1..={MAX_VERSION_NAME} printable characters"),
        ));
    }
    Ok(name.to_string())
}

fn insert_entry(tx: &Transaction<'_>, entry: &HistoryEntry) -> Result<(), Error> {
    entry.snapshot.recipe.validate()?;
    tx.execute(
        "INSERT INTO entries VALUES (?1,?2,?3,?4,?5,?6)",
        params![
            entry.id.as_str(),
            entry.asset_id.as_str(),
            entry.sequence as i64,
            entry.action_id,
            entry.undo_parent.as_ref().map(EntryId::as_str),
            encode(entry)?
        ],
    )
    .map_err(catalog_error)?;
    Ok(())
}

fn next_sequence(connection: &Connection, asset_id: &AssetId) -> Result<u64, Error> {
    let sequence: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(sequence),-1)+1 FROM entries WHERE asset_id=?1",
            [asset_id.as_str()],
            |row| row.get(0),
        )
        .map_err(catalog_error)?;
    u64::try_from(sequence).map_err(|_| Error::new(ErrorKind::Catalog, "invalid history sequence"))
}

fn asset_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetRecord> {
    let id = AssetId::parse(row.get::<_, String>(0)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })?;
    Ok(AssetRecord {
        id,
        source_root: PathBuf::from(row.get::<_, String>(1)?),
        locator: PathBuf::from(row.get::<_, String>(2)?),
        fingerprint: row.get(3)?,
        file_identity: row.get(4)?,
        byte_len: row.get::<_, i64>(5)? as u64,
        width: row.get::<_, i64>(6)? as u32,
        height: row.get::<_, i64>(7)? as u32,
    })
}

fn state_from(connection: &Connection, asset_id: &AssetId) -> Result<EditorState, Error> {
    let asset = connection
        .query_row(
            &format!("SELECT {ASSET_COLUMNS} FROM assets WHERE id=?1"),
            [asset_id.as_str()],
            asset_record,
        )
        .optional()
        .map_err(catalog_error)?
        .ok_or_else(|| Error::new(ErrorKind::Validation, "unknown asset"))?;
    let (current, revision, redo): (String, i64, String) = connection
        .query_row(
            "SELECT current_entry_id,revision,redo_json FROM asset_state WHERE asset_id=?1",
            [asset_id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(catalog_error)?;
    let current_id = EntryId::parse(current)?;
    Ok(EditorState {
        asset,
        revision: revision as u64,
        current_entry: entry_from(connection, asset_id, &current_id)?,
        redo: decode("invalid redo state", redo)?,
    })
}

fn entry_from(
    connection: &Connection,
    asset_id: &AssetId,
    entry_id: &EntryId,
) -> Result<HistoryEntry, Error> {
    let json: String = connection
        .query_row(
            "SELECT entry_json FROM entries WHERE id=?1 AND asset_id=?2",
            params![entry_id.as_str(), asset_id.as_str()],
            |row| row.get(0),
        )
        .optional()
        .map_err(catalog_error)?
        .ok_or_else(|| {
            Error::new(
                ErrorKind::Validation,
                "history entry does not belong to this asset",
            )
        })?;
    decode("invalid history entry", json)
}

fn source_signature(path: &Path, metadata: &Metadata) -> SourceSignature {
    SourceSignature {
        byte_len: metadata.len(),
        modified: metadata.modified().ok(),
        file_identity: file_identity(metadata, path),
        change_marker: metadata_change_marker(metadata),
    }
}

#[cfg(unix)]
fn metadata_change_marker(metadata: &Metadata) -> Option<(i128, i128)> {
    use std::os::unix::fs::MetadataExt;
    Some((
        i128::from(metadata.ctime()),
        i128::from(metadata.ctime_nsec()),
    ))
}

#[cfg(windows)]
fn metadata_change_marker(metadata: &Metadata) -> Option<(i128, i128)> {
    use std::os::windows::fs::MetadataExt;
    Some((i128::from(metadata.last_write_time()), 0))
}

#[cfg(not(any(unix, windows)))]
fn metadata_change_marker(_: &Metadata) -> Option<(i128, i128)> {
    None
}

#[cfg(unix)]
fn file_identity(metadata: &Metadata, _: &Path) -> String {
    use std::os::unix::fs::MetadataExt;
    format!("unix:{}:{}", metadata.dev(), metadata.ino())
}

#[cfg(windows)]
fn file_identity(metadata: &Metadata, canonical: &Path) -> String {
    use std::os::windows::fs::MetadataExt;
    match (metadata.volume_serial_number(), metadata.file_index()) {
        (Some(volume), Some(index)) => format!("windows:{volume}:{index}"),
        _ => format!("path:{}", canonical.to_string_lossy().to_lowercase()),
    }
}

#[cfg(not(any(unix, windows)))]
fn file_identity(_: &Metadata, canonical: &Path) -> String {
    format!("path:{}", canonical.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(1);
    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "lightwell-editor-{}-{}-{name}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }
    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/s0/orientation-1.jpg")
    }
    fn mutation(revision: u64, request: &str) -> Mutation {
        Mutation {
            expected_revision: revision,
            request_id: request.into(),
            actor: "test".into(),
        }
    }

    #[test]
    fn persistent_history_journey_retains_snapshots_navigation_and_source() {
        let catalog = temp("journey.sqlite");
        let source_path = fixture();
        let source_bytes = std::fs::read(&source_path).unwrap();
        let (asset, original, a, b);
        {
            let mut service = EditorService::open(&catalog).unwrap();
            let state = service.import(&source_path).unwrap();
            asset = state.asset.id.clone();
            original = state.current_entry.id.clone();
            let result = service
                .apply_pixel(&asset, mutation(0, "a"), 0, 0, [1, 2, 3])
                .unwrap();
            a = result.current_entry_id;
            let result = service
                .apply_pixel(&asset, mutation(1, "b"), 0, 0, [4, 5, 6])
                .unwrap();
            b = result.current_entry_id;
            assert_eq!(
                service.render_current(&asset).unwrap().pixel(0, 0),
                Some([4, 5, 6, 255])
            );
            service.undo(&asset, mutation(2, "undo")).unwrap();
            assert_eq!(
                service.render_current(&asset).unwrap().pixel(0, 0),
                Some([1, 2, 3, 255])
            );
            service.redo(&asset, mutation(3, "redo")).unwrap();
            service.restore(&asset, mutation(4, "restore"), &a).unwrap();
            service
                .apply_pixel(&asset, mutation(5, "c"), 1, 0, [7, 8, 9])
                .unwrap();
            assert_eq!(service.history(&asset, None, 20).unwrap().entries.len(), 5);
        }
        let service = EditorService::open(&catalog).unwrap();
        let state = service.state(&asset).unwrap();
        assert_eq!(state.revision, 6);
        assert!(
            service
                .entry(&asset, &original)
                .unwrap()
                .snapshot
                .recipe
                .layers
                .is_empty()
        );
        assert_eq!(
            service.render_entry(&asset, &b).unwrap().pixel(0, 0),
            Some([4, 5, 6, 255])
        );
        assert_eq!(std::fs::read(source_path).unwrap(), source_bytes);
        drop(service);
        std::fs::remove_file(catalog).unwrap();
    }

    #[test]
    fn retries_noops_conflicts_pagination_and_wrong_assets_are_safe() {
        let catalog = temp("requests.sqlite");
        let mut service = EditorService::open(&catalog).unwrap();
        let state = service.import(&fixture()).unwrap();
        let asset = state.asset.id;
        let original_pixel = service.render_current(&asset).unwrap().pixel(0, 0).unwrap();
        let no_op = service
            .apply_pixel(
                &asset,
                mutation(0, "noop"),
                0,
                0,
                [original_pixel[0], original_pixel[1], original_pixel[2]],
            )
            .unwrap();
        assert_eq!(no_op.outcome, MutationOutcome::NoOp);
        let first = service
            .apply_pixel(&asset, mutation(0, "same"), 0, 0, [1, 2, 3])
            .unwrap();
        let retry = service
            .apply_pixel(&asset, mutation(0, "same"), 0, 0, [1, 2, 3])
            .unwrap();
        assert_eq!(first.current_entry_id, retry.current_entry_id);
        assert!(retry.deduplicated);
        assert!(
            service
                .apply_pixel(&asset, mutation(0, "same"), 0, 0, [9, 9, 9])
                .is_err()
        );
        assert_eq!(
            service
                .apply_pixel(&asset, mutation(0, "stale"), 1, 0, [9, 9, 9])
                .unwrap_err()
                .kind,
            ErrorKind::Conflict
        );
        assert_eq!(service.history(&asset, None, 1).unwrap().entries.len(), 1);
        assert!(service.history(&asset, None, 101).is_err());
        assert!(
            service
                .entry(&AssetId::new(), &first.current_entry_id)
                .is_err()
        );
        drop(service);
        std::fs::remove_file(catalog).unwrap();
    }

    #[test]
    fn aliases_reuse_asset_but_copies_do_not_and_changed_sources_fail() {
        let dir = temp("aliases");
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("source.jpg");
        let hard = dir.join("hard.jpg");
        let copy = dir.join("copy.jpg");
        std::fs::copy(fixture(), &source).unwrap();
        std::fs::hard_link(&source, &hard).unwrap();
        std::fs::copy(&source, &copy).unwrap();
        let mut service = EditorService::open(&dir.join("catalog.sqlite")).unwrap();
        let a = service.import(&source).unwrap().asset.id;
        assert_eq!(service.import(&hard).unwrap().asset.id, a);
        assert_ne!(service.import(&copy).unwrap().asset.id, a);
        let listed: Vec<AssetId> = service
            .assets()
            .unwrap()
            .into_iter()
            .map(|asset| asset.id)
            .collect();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0], a);
        std::fs::write(&source, b"changed").unwrap();
        assert_eq!(
            service.render_current(&a).unwrap_err().kind,
            ErrorKind::SourceUnavailable
        );
        drop(service);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unchanged_sources_reuse_one_decoded_pixel_allocation() {
        let catalog = temp("source-cache.sqlite");
        let mut service = EditorService::open(&catalog).unwrap();
        let state = service.import(&fixture()).unwrap();
        let first = service.preview_job(&state.asset.id, None).unwrap();
        let second = service.preview_job(&state.asset.id, None).unwrap();
        assert!(std::sync::Arc::ptr_eq(
            &first.source.rgba,
            &second.source.rgba
        ));
        let raster = render(
            &first.source,
            first.entry.snapshot.id.clone(),
            &first.entry.snapshot.recipe,
        )
        .unwrap();
        assert!(std::sync::Arc::ptr_eq(&first.source.rgba, &raster.rgba));
        drop(service);
        std::fs::remove_file(catalog).unwrap();
    }

    #[test]
    fn same_length_source_replacement_invalidates_the_decode_cache() {
        let dir = temp("source-cache-invalidation");
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("source.jpg");
        std::fs::copy(fixture(), &source).unwrap();
        let mut service = EditorService::open(&dir.join("catalog.sqlite")).unwrap();
        let asset = service.import(&source).unwrap().asset.id;
        service.preview_job(&asset, None).unwrap();
        let replacement =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/s0/orientation-2.jpg");
        assert_eq!(
            std::fs::metadata(&source).unwrap().len(),
            std::fs::metadata(&replacement).unwrap().len()
        );
        std::fs::copy(replacement, &source).unwrap();
        assert_eq!(
            service.preview_job(&asset, None).unwrap_err().kind,
            ErrorKind::SourceUnavailable
        );
        drop(service);
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn format_1_catalog(path: &Path, drop_snapshot_row: bool) -> (AssetId, EntryId) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE assets (id TEXT PRIMARY KEY, source_root TEXT NOT NULL, locator TEXT NOT NULL,
                    canonical_locator TEXT NOT NULL UNIQUE, file_identity TEXT NOT NULL UNIQUE,
                    fingerprint TEXT NOT NULL, byte_len INTEGER NOT NULL, width INTEGER NOT NULL, height INTEGER NOT NULL);
                 CREATE TABLE snapshots (id TEXT PRIMARY KEY, asset_id TEXT NOT NULL REFERENCES assets(id), recipe_json TEXT NOT NULL);
                 CREATE TABLE snapshot_layers (snapshot_id TEXT NOT NULL REFERENCES snapshots(id), position INTEGER NOT NULL,
                    layer_id TEXT NOT NULL, effect_id TEXT NOT NULL, effect_format INTEGER NOT NULL, payload_json TEXT NOT NULL,
                    PRIMARY KEY(snapshot_id, position), UNIQUE(snapshot_id, layer_id));
                 CREATE TABLE entries (id TEXT PRIMARY KEY, asset_id TEXT NOT NULL REFERENCES assets(id), sequence INTEGER NOT NULL,
                    action_id TEXT NOT NULL, entry_json TEXT NOT NULL, UNIQUE(asset_id, sequence));
                 CREATE TABLE asset_state (asset_id TEXT PRIMARY KEY REFERENCES assets(id), current_entry_id TEXT NOT NULL REFERENCES entries(id),
                    revision INTEGER NOT NULL, redo_json TEXT NOT NULL);
                 CREATE TABLE requests (asset_id TEXT NOT NULL REFERENCES assets(id), request_id TEXT NOT NULL, input_hash TEXT NOT NULL,
                    result_json TEXT NOT NULL, PRIMARY KEY(asset_id, request_id));
                 CREATE TRIGGER snapshots_are_immutable BEFORE UPDATE ON snapshots BEGIN SELECT RAISE(ABORT, 'snapshots are immutable'); END;
                 CREATE TRIGGER entries_are_immutable BEFORE UPDATE ON entries BEGIN SELECT RAISE(ABORT, 'history entries are immutable'); END;
                 PRAGMA user_version=1;",
            )
            .unwrap();
        let asset = AssetId::new();
        let original = HistoryEntry {
            id: EntryId::new(),
            asset_id: asset.clone(),
            sequence: 0,
            action_id: "original".into(),
            parameters: json!({}),
            actor: "system".into(),
            timestamp_ms: 0,
            request_id: None,
            base_revision: 0,
            result_revision: 0,
            snapshot: Snapshot::original(asset.clone()),
            undo_parent: None,
            restore_target: None,
        };
        let edit = HistoryEntry {
            id: EntryId::new(),
            sequence: 1,
            action_id: "set-pixel".into(),
            result_revision: 1,
            snapshot: original
                .snapshot
                .append(Layer::pixel(0, 0, [1, 2, 3]))
                .unwrap(),
            undo_parent: Some(original.id.clone()),
            ..original.clone()
        };
        connection
            .execute(
                "INSERT INTO assets VALUES (?1,'/r','/r/a.jpg','/r/a.jpg','unix:1:1','abc',1,480,320)",
                [asset.as_str()],
            )
            .unwrap();
        for entry in [&original, &edit] {
            if !(drop_snapshot_row && entry.sequence == 1) {
                connection
                    .execute(
                        "INSERT INTO snapshots VALUES (?1,?2,?3)",
                        params![
                            entry.snapshot.id.as_str(),
                            asset.as_str(),
                            encode(&entry.snapshot.recipe).unwrap()
                        ],
                    )
                    .unwrap();
            }
            connection
                .execute(
                    "INSERT INTO entries VALUES (?1,?2,?3,?4,?5)",
                    params![
                        entry.id.as_str(),
                        asset.as_str(),
                        entry.sequence as i64,
                        entry.action_id,
                        encode(entry).unwrap()
                    ],
                )
                .unwrap();
        }
        connection
            .execute(
                "INSERT INTO asset_state VALUES (?1,?2,1,'[]')",
                params![asset.as_str(), edit.id.as_str()],
            )
            .unwrap();
        (asset, edit.id)
    }

    fn user_version(path: &Path) -> i64 {
        Connection::open(path)
            .unwrap()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn format_1_catalogs_convert_once_and_inconsistent_ones_are_left_alone() {
        let catalog = temp("format1.sqlite");
        let (asset, current) = format_1_catalog(&catalog, false);
        let service = EditorService::open(&catalog).unwrap();
        let state = service.state(&asset).unwrap();
        assert_eq!(state.current_entry.id, current);
        assert_eq!(state.current_entry.snapshot.recipe.layers.len(), 1);
        let lineage = service.lineage(&asset, None, 10).unwrap();
        assert_eq!(lineage.steps.len(), 2);
        assert_eq!(lineage.steps[0].entry_id, current);
        assert!(service.versions(&asset).unwrap().is_empty());
        drop(service);
        assert_eq!(user_version(&catalog), 2);
        let tables: Vec<String> = {
            let connection = Connection::open(&catalog).unwrap();
            let mut statement = connection
                .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
                .unwrap();
            statement
                .query_map([], |row| row.get(0))
                .unwrap()
                .map(Result::unwrap)
                .collect()
        };
        assert_eq!(
            tables,
            ["asset_state", "assets", "entries", "requests", "versions"]
        );
        EditorService::open(&catalog).unwrap();
        std::fs::remove_file(catalog).unwrap();

        let catalog = temp("format1-broken.sqlite");
        format_1_catalog(&catalog, true);
        assert_eq!(
            EditorService::open(&catalog).unwrap_err().kind,
            ErrorKind::Incompatible
        );
        assert_eq!(user_version(&catalog), 1);
        std::fs::remove_file(catalog).unwrap();
    }

    #[test]
    fn versions_name_retained_entries_and_survive_reopen() {
        let catalog = temp("versions.sqlite");
        let (asset, a, original);
        {
            let mut service = EditorService::open(&catalog).unwrap();
            let state = service.import(&fixture()).unwrap();
            asset = state.asset.id.clone();
            original = state.current_entry.id.clone();
            a = service
                .apply_pixel(&asset, mutation(0, "a"), 0, 0, [1, 2, 3])
                .unwrap()
                .current_entry_id;
            let created = service
                .create_version(&asset, " Keeper ", None, "test")
                .unwrap();
            assert_eq!(created.outcome, MutationOutcome::Applied);
            assert_eq!(created.version.as_ref().unwrap().name, "Keeper");
            assert_eq!(created.version.as_ref().unwrap().entry_id, a);
            let again = service
                .create_version(&asset, "keeper", Some(&a), "test")
                .unwrap();
            assert_eq!(again.outcome, MutationOutcome::NoOp);
            assert_eq!(
                service
                    .create_version(&asset, "KEEPER", Some(&original), "test")
                    .unwrap_err()
                    .kind,
                ErrorKind::Conflict
            );
            assert!(service.create_version(&asset, "", None, "test").is_err());
            assert!(
                service
                    .create_version(&asset, "bad\u{7}", None, "test")
                    .is_err()
            );
            service.undo(&asset, mutation(1, "undo")).unwrap();
            service
                .create_version(&asset, "Start", None, "test")
                .unwrap();
            let names: Vec<(String, u64)> = service
                .versions(&asset)
                .unwrap()
                .into_iter()
                .map(|version| (version.name, version.entry_sequence))
                .collect();
            assert_eq!(names, [("Keeper".to_string(), 1), ("Start".to_string(), 0)]);
        }
        let mut service = EditorService::open(&catalog).unwrap();
        let versions = service.versions(&asset).unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[1].entry_id, original);
        service
            .restore(&asset, mutation(2, "restore-keeper"), &versions[0].entry_id)
            .unwrap();
        assert_eq!(
            service.render_current(&asset).unwrap().pixel(0, 0),
            Some([1, 2, 3, 255])
        );
        assert_eq!(
            service.delete_version(&asset, "keeper").unwrap().outcome,
            MutationOutcome::Applied
        );
        assert_eq!(
            service.delete_version(&asset, "keeper").unwrap().outcome,
            MutationOutcome::NoOp
        );
        assert_eq!(service.versions(&asset).unwrap().len(), 1);
        assert!(
            service.entry(&asset, &a).is_ok(),
            "deleting a version keeps its entry"
        );
        drop(service);
        std::fs::remove_file(catalog).unwrap();
    }

    #[test]
    fn lineage_follows_undo_parents_and_skips_abandoned_branches() {
        let catalog = temp("lineage.sqlite");
        let mut service = EditorService::open(&catalog).unwrap();
        let state = service.import(&fixture()).unwrap();
        let asset = state.asset.id;
        let original = state.current_entry.id;
        let a = service
            .apply_pixel(&asset, mutation(0, "a"), 0, 0, [1, 2, 3])
            .unwrap()
            .current_entry_id;
        let b = service
            .apply_pixel(&asset, mutation(1, "b"), 1, 0, [4, 5, 6])
            .unwrap()
            .current_entry_id;
        service.undo(&asset, mutation(2, "undo")).unwrap();
        let c = service
            .apply_pixel(&asset, mutation(3, "c"), 2, 0, [7, 8, 9])
            .unwrap()
            .current_entry_id;
        let ids = |lineage: Lineage| -> Vec<EntryId> {
            lineage
                .steps
                .into_iter()
                .map(|step| step.entry_id)
                .collect()
        };
        assert_eq!(
            ids(service.lineage(&asset, None, 10).unwrap()),
            [c.clone(), a.clone(), original.clone()]
        );
        assert_eq!(service.history(&asset, None, 10).unwrap().entries.len(), 4);
        assert_eq!(
            ids(service.lineage(&asset, Some(&b), 10).unwrap()),
            [b.clone(), a.clone(), original.clone()]
        );
        let page = service.lineage(&asset, None, 2).unwrap();
        assert_eq!(page.steps.len(), 2);
        assert_eq!(page.next_entry_id, Some(original.clone()));
        assert_eq!(
            ids(service
                .lineage(&asset, page.next_entry_id.as_ref(), 2)
                .unwrap()),
            [original]
        );
        assert!(service.lineage(&asset, None, 0).is_err());
        assert!(service.lineage(&AssetId::new(), Some(&c), 5).is_err());
        drop(service);
        std::fs::remove_file(catalog).unwrap();
    }

    #[test]
    fn transform_composition_is_persistent_and_exact() {
        let catalog = temp("transform.sqlite");
        let mut service = EditorService::open(&catalog).unwrap();
        let asset = service.import(&fixture()).unwrap().asset.id;
        service
            .apply_pixel(&asset, mutation(0, "pixel"), 0, 0, [1, 2, 3])
            .unwrap();
        service
            .apply_transform(&asset, mutation(1, "right"), Transform::RotateRight)
            .unwrap();
        let state = service.state(&asset).unwrap();
        assert_eq!(
            (
                service.render_current(&asset).unwrap().width,
                service.render_current(&asset).unwrap().height
            ),
            (320, 480)
        );
        assert_eq!(state.current_entry.snapshot.recipe.layers.len(), 2);
        service.undo(&asset, mutation(2, "undo-transform")).unwrap();
        assert_eq!(
            (
                service.render_current(&asset).unwrap().width,
                service.render_current(&asset).unwrap().height
            ),
            (480, 320)
        );
        drop(service);
        let service = EditorService::open(&catalog).unwrap();
        assert_eq!(service.state(&asset).unwrap().revision, 3);
        drop(service);
        std::fs::remove_file(catalog).unwrap();
    }

    #[test]
    fn incompatible_and_competing_catalogs_fail_without_rewriting_data() {
        let catalog = temp("owner.sqlite");
        let owner = EditorService::open(&catalog).unwrap();
        assert_eq!(
            EditorService::open(&catalog).unwrap_err().kind,
            ErrorKind::Conflict
        );
        drop(owner);
        let connection = Connection::open(&catalog).unwrap();
        connection.pragma_update(None, "user_version", 99).unwrap();
        drop(connection);
        assert_eq!(
            EditorService::open(&catalog).unwrap_err().kind,
            ErrorKind::Incompatible
        );
        std::fs::remove_file(catalog).unwrap();
    }

    #[test]
    fn failed_entry_write_rolls_back_snapshot_state_and_request() {
        let catalog = temp("rollback.sqlite");
        let mut service = EditorService::open(&catalog).unwrap();
        let asset = service.import(&fixture()).unwrap().asset.id;
        let before = service.state(&asset).unwrap();
        service
            .connection
            .execute_batch(
                "CREATE TRIGGER injected_entry_failure BEFORE INSERT ON entries
                 WHEN NEW.sequence > 0 BEGIN SELECT RAISE(ABORT, 'injected'); END;",
            )
            .unwrap();
        assert!(
            service
                .apply_pixel(&asset, mutation(0, "will-fail"), 0, 0, [1, 2, 3])
                .is_err()
        );
        assert_eq!(service.state(&asset).unwrap(), before);
        assert_eq!(service.history(&asset, None, 10).unwrap().entries.len(), 1);
        service
            .connection
            .execute_batch("DROP TRIGGER injected_entry_failure")
            .unwrap();
        assert!(
            service
                .apply_pixel(&asset, mutation(0, "will-fail"), 0, 0, [1, 2, 3])
                .is_ok(),
            "a failed transaction must not retain a false request result"
        );
        drop(service);
        std::fs::remove_file(catalog).unwrap();
    }

    #[test]
    fn undoing_restore_returns_to_the_preceding_current_entry() {
        let catalog = temp("restore-undo.sqlite");
        let mut service = EditorService::open(&catalog).unwrap();
        let state = service.import(&fixture()).unwrap();
        let asset = state.asset.id;
        let a = service
            .apply_pixel(&asset, mutation(0, "a"), 0, 0, [1, 2, 3])
            .unwrap()
            .current_entry_id;
        let b = service
            .apply_pixel(&asset, mutation(1, "b"), 0, 0, [4, 5, 6])
            .unwrap()
            .current_entry_id;
        service
            .restore(&asset, mutation(2, "restore-a"), &a)
            .unwrap();
        service.undo(&asset, mutation(3, "undo-restore")).unwrap();
        assert_eq!(service.state(&asset).unwrap().current_entry.id, b);
        assert_eq!(
            service.render_current(&asset).unwrap().pixel(0, 0),
            Some([4, 5, 6, 255])
        );
        drop(service);
        std::fs::remove_file(catalog).unwrap();
    }
}
