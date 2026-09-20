//! Continuity and parity proofs for the module registry: a journey recorded with the pre-M3
//! binary must reopen with the same identities, snapshots, navigation and exact pixels, and the
//! same action must behave identically through the wrappers, `apply_action` and the JSON API.
use crate::{
    ApiRequest, AssetId, ClientId, EditorService, EntryId, ErrorKind, HistoryEntry, Mutation,
    MutationOutcome, OwnerHandle, Transform,
};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const GOLDEN: &str = include_str!("../../../fixtures/history/m2-journey.json");
static NEXT: AtomicU64 = AtomicU64::new(1);

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "lightwell-continuity-{}-{}-{name}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/s0")
        .join(name)
}

fn golden() -> Value {
    serde_json::from_str(GOLDEN).unwrap()
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap()
}

fn mutation(revision: u64, request: &str, actor: &str) -> Mutation {
    Mutation {
        expected_revision: revision,
        request_id: request.into(),
        actor: actor.into(),
    }
}

/// Rebuild the recorded catalog around a local copy of the fixture: the asset row is rebound to
/// this machine's locator and file identity, everything else is inserted exactly as recorded.
fn recorded_catalog(dir: &Path, golden: &Value) -> (PathBuf, PathBuf, AssetId) {
    let source = dir.join("orientation-6.jpg");
    std::fs::copy(
        fixture(text(&golden["fixture"]).rsplit('/').next().unwrap()),
        &source,
    )
    .unwrap();
    let catalog = dir.join("catalog.sqlite");
    let imported = {
        let mut service = EditorService::open(&catalog).unwrap();
        service.import(&source).unwrap().asset
    };
    let recorded = &golden["state"]["asset"];
    assert_eq!(
        imported.fingerprint,
        text(&recorded["fingerprint"]),
        "the local fixture copy is the recorded source"
    );
    assert_eq!(
        (u64::from(imported.width), u64::from(imported.height)),
        (
            recorded["width"].as_u64().unwrap(),
            recorded["height"].as_u64().unwrap()
        )
    );
    let asset = AssetId::parse(text(&recorded["id"])).unwrap();

    let connection = Connection::open(&catalog).unwrap();
    let (root, locator, canonical, identity, byte_len): (String, String, String, String, i64) =
        connection
            .query_row(
                "SELECT source_root,locator,canonical_locator,file_identity,byte_len FROM assets WHERE id=?1",
                [imported.id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .unwrap();
    connection
        .execute_batch("DELETE FROM asset_state; DELETE FROM requests; DELETE FROM entries; DELETE FROM assets;")
        .unwrap();
    connection
        .execute(
            "INSERT INTO assets VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                asset.as_str(),
                root,
                locator,
                canonical,
                identity,
                text(&recorded["fingerprint"]),
                byte_len,
                recorded["width"].as_i64().unwrap(),
                recorded["height"].as_i64().unwrap(),
            ],
        )
        .unwrap();
    for entry in golden["entries"].as_array().unwrap().iter().rev() {
        connection
            .execute(
                "INSERT INTO entries VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    text(&entry["id"]),
                    asset.as_str(),
                    entry["sequence"].as_i64().unwrap(),
                    text(&entry["action_id"]),
                    entry["undo_parent"].as_str(),
                    serde_json::to_string(entry).unwrap(),
                ],
            )
            .unwrap();
    }
    connection
        .execute(
            "INSERT INTO asset_state VALUES (?1,?2,?3,?4)",
            params![
                asset.as_str(),
                text(&golden["state"]["current_entry"]["id"]),
                golden["state"]["revision"].as_i64().unwrap(),
                text(&golden["redo_json"]),
            ],
        )
        .unwrap();
    for (request_id, hash) in golden["request_hashes"].as_object().unwrap() {
        connection
            .execute(
                "INSERT INTO requests VALUES (?1,?2,?3,?4)",
                params![
                    asset.as_str(),
                    request_id,
                    text(hash),
                    serde_json::to_string(&golden["results"][request_id]).unwrap(),
                ],
            )
            .unwrap();
    }
    for version in golden["versions"].as_array().unwrap() {
        connection
            .execute(
                "INSERT INTO versions VALUES (?1,?2,?3,?4,?5)",
                params![
                    asset.as_str(),
                    text(&version["name"]),
                    text(&version["entry_id"]),
                    text(&version["actor"]),
                    version["created_ms"].as_i64().unwrap(),
                ],
            )
            .unwrap();
    }
    drop(connection);
    (catalog, source, asset)
}

fn check_samples(service: &EditorService, asset: &AssetId, entry: &EntryId, samples: &Value) {
    for sample in samples.as_array().unwrap() {
        let x = sample["x"].as_u64().unwrap() as u32;
        let y = sample["y"].as_u64().unwrap() as u32;
        match sample.get("error").and_then(Value::as_str) {
            Some(code) => assert_eq!(
                service
                    .sample_entry(asset, entry, x, y)
                    .unwrap_err()
                    .kind
                    .code(),
                code,
                "{entry} ({x}, {y})"
            ),
            None => {
                let actual = service.sample_entry(asset, entry, x, y).unwrap();
                assert_eq!(json!(actual.rgba), sample["rgba"], "{entry} ({x}, {y})");
                assert_eq!(json!(actual.width), sample["width"], "{entry} ({x}, {y})");
                assert_eq!(json!(actual.height), sample["height"], "{entry} ({x}, {y})");
            }
        }
    }
}

#[test]
fn a_pre_m3_journey_reopens_with_identical_state_history_pixels_and_navigation() {
    let golden = golden();
    let dir = temp("m2-journey");
    let (catalog, source, asset) = recorded_catalog(&dir, &golden);
    let bytes = std::fs::read(&source).unwrap();
    let mut service = EditorService::open(&catalog).unwrap();

    let state = service.state(&asset).unwrap();
    let expected: HistoryEntry =
        serde_json::from_value(golden["state"]["current_entry"].clone()).unwrap();
    assert_eq!(state.current_entry, expected);
    assert_eq!(
        json!(state.revision),
        golden["state"]["revision"],
        "revision survives"
    );
    assert_eq!(json!(state.redo), golden["state"]["redo"]);
    assert_eq!(state.asset.id, asset);
    assert_eq!(
        state.asset.fingerprint,
        text(&golden["state"]["asset"]["fingerprint"])
    );
    assert_eq!(
        state.asset.locator,
        source.canonicalize().unwrap(),
        "rebound to the local copy"
    );

    let recorded: Vec<HistoryEntry> = serde_json::from_value(golden["entries"].clone()).unwrap();
    let page = service.history(&asset, None, 100).unwrap();
    assert_eq!(page.entries, recorded, "every entry reopens unchanged");
    assert_eq!(page.next_before_sequence, None);

    for (entry_id, samples) in golden["samples"].as_object().unwrap() {
        let entry = EntryId::parse(entry_id.as_str()).unwrap();
        check_samples(&service, &asset, &entry, samples);
    }

    assert_eq!(
        serde_json::to_value(service.lineage(&asset, None, 100).unwrap()).unwrap(),
        golden["lineage"]
    );
    assert_eq!(
        serde_json::to_value(service.versions(&asset).unwrap()).unwrap(),
        golden["versions"]
    );

    // A pixel retry deduplicates: the M3 request hash for set-pixel is byte-identical to M2's.
    let retry = service
        .apply_pixel(&asset, mutation(0, "pixel-a", "golden-m2"), 0, 0, [1, 2, 3])
        .unwrap();
    assert!(retry.deduplicated);
    let mut expected = golden["results"]["pixel-a"].clone();
    expected["deduplicated"] = json!(true);
    assert_eq!(serde_json::to_value(&retry).unwrap(), expected);
    // Transform requests recorded before M3 hashed a different shape: a retry is a conflict, as
    // documented, and it changes nothing.
    let conflict = service
        .apply_transform(
            &asset,
            mutation(3, "rotate-right", "golden-m2"),
            Transform::RotateRight,
        )
        .unwrap_err();
    assert_eq!(conflict.kind, ErrorKind::Conflict);
    assert_eq!(service.state(&asset).unwrap(), state);

    // Navigation and restore keep working on the reopened stack.
    let revision = state.revision;
    let parent = state.current_entry.undo_parent.clone().unwrap();
    let undone = service
        .undo(&asset, mutation(revision, "m3-undo", "m3"))
        .unwrap();
    assert_eq!(undone.current_entry_id, parent);
    let redone = service
        .redo(&asset, mutation(revision + 1, "m3-redo", "m3"))
        .unwrap();
    assert_eq!(redone.current_entry_id, state.current_entry.id);
    let pixel_b = EntryId::parse(text(&golden["results"]["pixel-b"]["created_entry_id"])).unwrap();
    let restored = service
        .restore(&asset, mutation(revision + 2, "m3-restore", "m3"), &pixel_b)
        .unwrap();
    assert_eq!(restored.outcome, MutationOutcome::Applied);
    let restored_entry = service.entry(&asset, &restored.current_entry_id).unwrap();
    assert_eq!(restored_entry.action_id, "restore");
    assert_eq!(
        restored_entry.snapshot.recipe,
        service.entry(&asset, &pixel_b).unwrap().snapshot.recipe
    );
    // The recorded restore of the same entry evaluated to these pixels before M3.
    check_samples(
        &service,
        &asset,
        &restored.current_entry_id,
        &golden["samples"][text(&golden["results"]["restore-b"]["created_entry_id"])],
    );

    assert_eq!(std::fs::read(&source).unwrap(), bytes, "source unchanged");
    drop(service);
    std::fs::remove_dir_all(dir).unwrap();
}

/// The stored shape of one history entry, without the identities and timestamps that differ
/// between two equivalent journeys.
fn stacks(entries: &[Value]) -> Vec<Value> {
    entries
        .iter()
        .map(|entry| {
            json!({
                "action_id": entry["action_id"],
                "parameters": entry["parameters"],
                "layers": entry["snapshot"]["recipe"]["layers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|layer| json!({
                        "effect_id": layer["effect_id"],
                        "effect_format": layer["effect_format"],
                        "payload": layer["payload"],
                    }))
                    .collect::<Vec<_>>(),
            })
        })
        .collect()
}

const PROBES: [(u32, u32); 4] = [(0, 0), (2, 1), (1, 0), (479, 319)];

fn direct_journey(catalog: &Path, source: &Path, wrappers: bool) -> (Vec<Value>, Vec<Value>) {
    let mut service = EditorService::open(catalog).unwrap();
    let asset = service.import(source).unwrap().asset.id;
    if wrappers {
        service
            .apply_pixel(&asset, mutation(0, "a", "parity"), 0, 0, [1, 2, 3])
            .unwrap();
        service
            .apply_transform(&asset, mutation(1, "b", "parity"), Transform::RotateRight)
            .unwrap();
        service
            .apply_pixel(&asset, mutation(2, "c", "parity"), 2, 1, [4, 5, 6])
            .unwrap();
        service
            .apply_transform(
                &asset,
                mutation(3, "d", "parity"),
                Transform::MirrorHorizontal,
            )
            .unwrap();
    } else {
        for (revision, request, action, parameters) in [
            (0, "a", "set-pixel", json!({"x":0,"y":0,"rgb":[1,2,3]})),
            (1, "b", "transform", json!({"transform":"rotate-right"})),
            (2, "c", "set-pixel", json!({"x":2,"y":1,"rgb":[4,5,6]})),
            (
                3,
                "d",
                "transform",
                json!({"transform":"mirror-horizontal"}),
            ),
        ] {
            service
                .apply_action(
                    &asset,
                    mutation(revision, request, "parity"),
                    action,
                    parameters,
                )
                .unwrap();
        }
    }
    let entries: Vec<Value> = serde_json::to_value(service.history(&asset, None, 50).unwrap())
        .unwrap()["entries"]
        .as_array()
        .unwrap()
        .clone();
    let current = service.state(&asset).unwrap().current_entry.id;
    let samples = PROBES
        .iter()
        .map(|(x, y)| {
            let sampled = service.sample_entry(&asset, &current, *x, *y).unwrap();
            json!([sampled.rgba, sampled.width, sampled.height])
        })
        .collect();
    drop(service);
    (stacks(&entries), samples)
}

fn api_journey(catalog: &Path, source: &Path) -> (Vec<Value>, Vec<Value>, Value) {
    let (owner, join) = OwnerHandle::start(catalog).unwrap();
    let client = owner.register();
    let call = |client: ClientId, method: &str, params: Value| -> Value {
        let response = owner
            .call(
                client,
                ApiRequest {
                    id: method.into(),
                    method: method.into(),
                    params,
                    token: None,
                },
            )
            .unwrap();
        assert!(response.error.is_none(), "{method}: {:?}", response.error);
        response.result.unwrap()
    };
    let asset = call(client, "catalog.import", json!({"path": source}))["asset"]["id"].clone();
    for (revision, request, method, mut params) in [
        (0, "a", "edit.set-pixel", json!({"x":0,"y":0,"rgb":[1,2,3]})),
        (
            1,
            "b",
            "edit.transform",
            json!({"transform":"rotate-right"}),
        ),
        (2, "c", "edit.set-pixel", json!({"x":2,"y":1,"rgb":[4,5,6]})),
        (
            3,
            "d",
            "edit.transform",
            json!({"transform":"mirror-horizontal"}),
        ),
    ] {
        let object = params.as_object_mut().unwrap();
        object.insert("asset_id".into(), asset.clone());
        object.insert(
            "mutation".into(),
            json!({"expected_revision":revision,"request_id":request,"actor":"parity"}),
        );
        call(client, method, params);
    }
    let entries = call(client, "history.list", json!({"asset_id":asset,"limit":50}))["entries"]
        .as_array()
        .unwrap()
        .clone();
    let samples = PROBES
        .iter()
        .map(|(x, y)| {
            let sampled = call(
                client,
                "render.sample",
                json!({"asset_id":asset,"x":x,"y":y}),
            );
            json!([sampled["rgba"], sampled["width"], sampled["height"]])
        })
        .collect();
    let malformed = owner
        .call(
            client,
            ApiRequest {
                id: "malformed".into(),
                method: "edit.set-pixel".into(),
                params: json!({"asset_id":asset,"mutation":{"expected_revision":4,"request_id":"e","actor":"parity"},"x":0,"y":0,"rgb":[1,2]}),
                token: None,
            },
        )
        .unwrap();
    let failure = serde_json::to_value(malformed.error.unwrap()).unwrap();
    owner.stop();
    join.join().unwrap();
    (stacks(&entries), samples, failure)
}

#[test]
fn wrappers_actions_and_the_api_produce_identical_entries_pixels_and_errors() {
    let dir = temp("parity");
    let source = dir.join("orientation-6.jpg");
    std::fs::copy(fixture("orientation-6.jpg"), &source).unwrap();
    let bytes = std::fs::read(&source).unwrap();

    let wrapped = direct_journey(&dir.join("wrappers.sqlite"), &source, true);
    let actions = direct_journey(&dir.join("actions.sqlite"), &source, false);
    let (api_stacks, api_samples, failure) = api_journey(&dir.join("api.sqlite"), &source);

    assert_eq!(wrapped.0, actions.0, "wrappers and apply_action agree");
    assert_eq!(wrapped.0, api_stacks, "the API agrees with the core");
    assert_eq!(wrapped.1, actions.1);
    assert_eq!(wrapped.1, api_samples);
    assert_eq!(
        wrapped.0[0]["action_id"],
        json!("mirror-horizontal"),
        "durable transform action identities"
    );

    // One malformed request reports the same structured error everywhere.
    let mut service = EditorService::open(&dir.join("errors.sqlite")).unwrap();
    let asset = service.import(&source).unwrap().asset.id;
    let direct = service
        .apply_action(
            &asset,
            mutation(0, "e", "parity"),
            "set-pixel",
            json!({"x":0,"y":0,"rgb":[1,2]}),
        )
        .unwrap_err();
    assert_eq!(
        json!({"code": direct.kind.code(), "message": direct.detail}),
        failure
    );
    drop(service);

    assert_eq!(std::fs::read(&source).unwrap(), bytes, "source unchanged");
    std::fs::remove_dir_all(dir).unwrap();
}
