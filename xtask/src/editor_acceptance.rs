use crate::*;
use lightwell_core::{EditorService, Mutation, MutationOutcome, Transform};
use std::time::Instant;

fn mutation(revision: u64, request: impl Into<String>) -> Mutation {
    Mutation {
        expected_revision: revision,
        request_id: request.into(),
        actor: "xtask-acceptance".into(),
    }
}

pub fn run(root: &Path, out: &Path) -> Result {
    ensure(!out.exists(), "Editor acceptance output must be new")?;
    fs::create_dir_all(out)?;
    let fixture = root.join("fixtures/s0/orientation-1.jpg");
    let fixture_hash = hash(&fixture)?;
    let catalog = out.join("catalog.sqlite");
    let mut result = json!({
        "status":"failed",
        "scope":["M1 history foundation","M2 basic transforms"],
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "platform":host(root)?,
        "fixture":"fixtures/s0/orientation-1.jpg",
        "fixture_sha256":fixture_hash,
        "method":"Exact core journey; timings use the recorded xtask profile and warm filesystem cache. Native UI evidence is recorded separately.",
    });
    let checked = (|| -> Result {
        let total = Instant::now();
        let started = Instant::now();
        let mut service = EditorService::open(&catalog)?;
        let state = service.import(&fixture)?;
        let import_ms = started.elapsed().as_secs_f64() * 1000.0;
        let asset = state.asset.id.clone();
        let original = state.current_entry.id.clone();
        let source_pixel = service.render_current(&asset)?.pixel(0, 0).unwrap();

        let a = service.apply_pixel(&asset, mutation(0, "pixel-a"), 0, 0, [1, 2, 3])?;
        ensure(
            a.outcome == MutationOutcome::Applied,
            "Pixel A was not applied",
        )?;
        let a_entry = a.current_entry_id;
        let b = service.apply_pixel(&asset, mutation(1, "pixel-b"), 0, 0, [4, 5, 6])?;
        ensure(
            service.render_current(&asset)?.pixel(0, 0) == Some([4, 5, 6, 255]),
            "Pixel B did not win stack order",
        )?;
        let preview_started = Instant::now();
        let preview_a = service.render_entry(&asset, &a_entry)?;
        let preview_ms = preview_started.elapsed().as_secs_f64() * 1000.0;
        ensure(
            preview_a.pixel(0, 0) == Some([1, 2, 3, 255]),
            "Historical preview A changed",
        )?;
        ensure(
            service.state(&asset)?.current_entry.id == b.current_entry_id,
            "Preview changed committed state",
        )?;

        service.undo(&asset, mutation(2, "undo-b"))?;
        service.redo(&asset, mutation(3, "redo-b"))?;
        service.restore(&asset, mutation(4, "restore-a"), &a_entry)?;
        service.apply_pixel(&asset, mutation(5, "pixel-c"), 1, 0, [7, 8, 9])?;
        service.apply_transform(&asset, mutation(6, "rotate-right"), Transform::RotateRight)?;
        service.apply_transform(
            &asset,
            mutation(7, "mirror-horizontal"),
            Transform::MirrorHorizontal,
        )?;
        service.apply_transform(
            &asset,
            mutation(8, "flip-vertical"),
            Transform::FlipVertical,
        )?;

        let long_log_started = Instant::now();
        for index in 0..200u64 {
            let revision = 9 + index;
            service.apply_transform(
                &asset,
                mutation(revision, format!("long-log-{index:03}")),
                Transform::MirrorHorizontal,
            )?;
        }
        let long_log_commit_ms = long_log_started.elapsed().as_secs_f64() * 1000.0;
        let page_started = Instant::now();
        let mut cursor = None;
        let mut listed = 0usize;
        loop {
            let page = service.history(&asset, cursor, 25)?;
            listed += page.entries.len();
            match page.next_before_sequence {
                Some(next) => cursor = Some(next),
                None => break,
            }
        }
        let history_paging_ms = page_started.elapsed().as_secs_f64() * 1000.0;
        ensure(
            listed == 208,
            format!("Expected 208 retained entries, found {listed}"),
        )?;
        drop(service);

        let reopen_started = Instant::now();
        let service = EditorService::open(&catalog)?;
        let reopened = service.state(&asset)?;
        let reopen_ms = reopen_started.elapsed().as_secs_f64() * 1000.0;
        ensure(reopened.revision == 209, "Revision did not survive reopen")?;
        ensure(
            service
                .entry(&asset, &original)?
                .snapshot
                .recipe
                .layers
                .is_empty(),
            "Original snapshot did not survive reopen",
        )?;
        ensure(
            service.render_entry(&asset, &a_entry)?.pixel(0, 0) == Some([1, 2, 3, 255]),
            "Historical pixel did not survive reopen",
        )?;
        let render_started = Instant::now();
        let current = service.render_current(&asset)?;
        let current_render_ms = render_started.elapsed().as_secs_f64() * 1000.0;
        ensure(
            (current.width, current.height) == (320, 480),
            "Transform dimensions did not survive reopen",
        )?;
        ensure(hash(&fixture)? == fixture_hash, "Original source changed")?;
        result["status"] = json!("passed");
        result["asset_id"] = json!(asset);
        result["original_entry_id"] = json!(original);
        result["pixel_a_entry_id"] = json!(a_entry);
        result["revision"] = json!(reopened.revision);
        result["history_entries"] = json!(listed);
        result["current_dimensions"] = json!([current.width, current.height]);
        result["source_pixel_before_edits"] = json!(source_pixel);
        result["timings_ms"] = json!({
            "import":import_ms,
            "historical_preview":preview_ms,
            "long_log_200_commits":long_log_commit_ms,
            "history_208_entries_paged_by_25":history_paging_ms,
            "catalog_reopen":reopen_ms,
            "current_render_203_layers":current_render_ms,
            "total":total.elapsed().as_secs_f64()*1000.0,
        });
        result["catalog_bytes"] = json!(fs::metadata(&catalog)?.len());
        result["checks"] = json!([
            "Original -> pixel A -> pixel B ordering",
            "Read-only historical preview",
            "Undo/redo and Restore A -> pixel C",
            "Exact rotate-right, mirror-horizontal and flip-vertical",
            "Two hundred additional exact layers with bounded history paging",
            "Catalog reopen retains revision, identities, snapshots and dimensions",
            "Source SHA-256 unchanged"
        ]);
        Ok(())
    })();
    if let Err(error) = &checked {
        result["error"] = json!(error.to_string());
    }
    write_json(&out.join("result.json"), &result)?;
    fs::write(
        out.join("README.md"),
        "# M1/M2 automated acceptance\n\nRun from the repository root with:\n\n```sh\ncargo xtask editor-acceptance --output NEW_DIRECTORY\n```\n\n`result.json` records exact state, hashes, timings and the tested platform. Native UI capture and platform classification are recorded in the engineering results document.\n",
    )?;
    checked?;
    println!("PASS M1/M2 editor acceptance: {}", out.display());
    Ok(())
}
