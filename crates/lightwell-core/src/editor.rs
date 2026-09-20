use crate::{
    AssetId, EntryId, Error, ErrorKind, HistoryEntry, Layer, Mutation, PreviewJob, Raster,
    Snapshot, SnapshotId, SourceImage, Transform, open_source, render,
};
use rusqlite::{
    Connection, ErrorCode, OptionalExtension, Transaction, TransactionBehavior, params,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::Metadata,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const CATALOG_FORMAT: i64 = 1;
const MAX_HISTORY_PAGE: usize = 100;

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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub next_before_sequence: Option<u64>,
}

#[derive(Debug)]
pub struct EditorService {
    connection: Connection,
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
        if version == 0 {
            Self::create_schema(&connection)?;
        } else if version != CATALOG_FORMAT {
            return Err(Error::new(
                ErrorKind::Incompatible,
                format!("catalog format {version} is not supported; expected {CATALOG_FORMAT}"),
            ));
        }
        Ok(Self { connection })
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
                 CREATE TABLE snapshots (
                    id TEXT PRIMARY KEY,
                    asset_id TEXT NOT NULL REFERENCES assets(id),
                    recipe_json TEXT NOT NULL
                 );
                 CREATE TABLE snapshot_layers (
                    snapshot_id TEXT NOT NULL REFERENCES snapshots(id),
                    position INTEGER NOT NULL,
                    layer_id TEXT NOT NULL,
                    effect_id TEXT NOT NULL,
                    effect_format INTEGER NOT NULL,
                    payload_json TEXT NOT NULL,
                    PRIMARY KEY(snapshot_id, position),
                    UNIQUE(snapshot_id, layer_id)
                 );
                 CREATE TABLE entries (
                    id TEXT PRIMARY KEY,
                    asset_id TEXT NOT NULL REFERENCES assets(id),
                    sequence INTEGER NOT NULL,
                    action_id TEXT NOT NULL,
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
                 CREATE TRIGGER snapshots_are_immutable BEFORE UPDATE ON snapshots BEGIN
                    SELECT RAISE(ABORT, 'snapshots are immutable');
                 END;
                 CREATE TRIGGER entries_are_immutable BEFORE UPDATE ON entries BEGIN
                    SELECT RAISE(ABORT, 'history entries are immutable');
                 END;
                 PRAGMA user_version=1;
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
        if metadata_signature(&before) != metadata_signature(&after) {
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
        insert_snapshot(&tx, &entry.snapshot)?;
        insert_entry(&tx, &entry)?;
        tx.execute(
            "INSERT INTO asset_state VALUES (?1,?2,0,'[]')",
            params![asset.id.as_str(), entry.id.as_str()],
        )
        .map_err(catalog_error)?;
        tx.commit().map_err(catalog_error)?;
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
            source: verified_source(&state.asset)?,
            entry,
        })
    }

    pub fn render_entry(&self, asset_id: &AssetId, entry_id: &EntryId) -> Result<Raster, Error> {
        let state = self.state(asset_id)?;
        let entry = self.entry(asset_id, entry_id)?;
        let source = verified_source(&state.asset)?;
        render(&source, entry.snapshot.id.clone(), &entry.snapshot.recipe)
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
        let source = verified_source(&state.asset)?;
        let current = render(
            &source,
            state.current_entry.snapshot.id.clone(),
            &state.current_entry.snapshot.recipe,
        )?;
        let rgba = current.pixel(x, y).ok_or_else(|| {
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
        insert_snapshot(&tx, &entry.snapshot)?;
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
        insert_snapshot(&tx, &entry.snapshot)?;
        insert_entry(&tx, &entry)?;
        tx.execute("UPDATE asset_state SET current_entry_id=?2,revision=?3,redo_json='[]' WHERE asset_id=?1", params![asset_id.as_str(), entry.id.as_str(), entry.result_revision as i64]).map_err(catalog_error)?;
        insert_request(&tx, asset_id, &mutation.request_id, &input, &result)?;
        tx.commit().map_err(catalog_error)?;
        Ok(result)
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

fn insert_snapshot(tx: &Transaction<'_>, snapshot: &Snapshot) -> Result<(), Error> {
    snapshot.recipe.validate()?;
    tx.execute(
        "INSERT INTO snapshots VALUES (?1,?2,?3)",
        params![
            snapshot.id.as_str(),
            snapshot.asset_id.as_str(),
            encode(&snapshot.recipe)?
        ],
    )
    .map_err(catalog_error)?;
    for (position, layer) in snapshot.recipe.layers.iter().enumerate() {
        tx.execute(
            "INSERT INTO snapshot_layers VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                snapshot.id.as_str(),
                position as i64,
                layer.id.as_str(),
                layer.effect_id,
                i64::from(layer.effect_format),
                encode(&layer.payload)?
            ],
        )
        .map_err(catalog_error)?;
    }
    Ok(())
}

fn insert_entry(tx: &Transaction<'_>, entry: &HistoryEntry) -> Result<(), Error> {
    tx.execute(
        "INSERT INTO entries VALUES (?1,?2,?3,?4,?5)",
        params![
            entry.id.as_str(),
            entry.asset_id.as_str(),
            entry.sequence as i64,
            entry.action_id,
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

fn state_from(connection: &Connection, asset_id: &AssetId) -> Result<EditorState, Error> {
    let asset_json = connection.query_row("SELECT source_root,locator,fingerprint,file_identity,byte_len,width,height FROM assets WHERE id=?1", [asset_id.as_str()], |row| {
        Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,i64>(4)?,row.get::<_,i64>(5)?,row.get::<_,i64>(6)?))
    }).optional().map_err(catalog_error)?.ok_or_else(|| Error::new(ErrorKind::Validation, "unknown asset"))?;
    let asset = AssetRecord {
        id: asset_id.clone(),
        source_root: asset_json.0.into(),
        locator: asset_json.1.into(),
        fingerprint: asset_json.2,
        file_identity: asset_json.3,
        byte_len: asset_json.4 as u64,
        width: asset_json.5 as u32,
        height: asset_json.6 as u32,
    };
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

fn verified_source(asset: &AssetRecord) -> Result<SourceImage, Error> {
    let metadata = asset.locator.metadata().map_err(|_| {
        Error::new(
            ErrorKind::SourceUnavailable,
            "original source is unavailable",
        )
    })?;
    if metadata.len() != asset.byte_len || metadata.len() > 128 * 1024 * 1024 {
        return Err(Error::new(
            ErrorKind::SourceUnavailable,
            "original source fingerprint changed",
        ));
    }
    let bytes = std::fs::read(&asset.locator).map_err(|_| {
        Error::new(
            ErrorKind::SourceUnavailable,
            "original source is unavailable",
        )
    })?;
    let fingerprint = format!("{:x}", Sha256::digest(&bytes));
    if fingerprint != asset.fingerprint {
        return Err(Error::new(
            ErrorKind::SourceUnavailable,
            "original source fingerprint changed",
        ));
    }
    let source = open_source(&asset.locator).map_err(|error| {
        if error.kind == ErrorKind::FileAccess {
            Error::new(
                ErrorKind::SourceUnavailable,
                "original source is unavailable",
            )
        } else {
            error
        }
    })?;
    if source.fingerprint != asset.fingerprint {
        return Err(Error::new(
            ErrorKind::SourceUnavailable,
            "original source fingerprint changed",
        ));
    }
    Ok(source)
}

fn metadata_signature(metadata: &Metadata) -> (u64, Option<SystemTime>) {
    (metadata.len(), metadata.modified().ok())
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
        std::fs::write(&source, b"changed").unwrap();
        assert_eq!(
            service.render_current(&a).unwrap_err().kind,
            ErrorKind::SourceUnavailable
        );
        drop(service);
        std::fs::remove_dir_all(dir).unwrap();
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
