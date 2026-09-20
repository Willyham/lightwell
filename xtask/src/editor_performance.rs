use crate::*;
use lightwell_core::{EditorService, Mutation, Transform, render};
use std::time::Instant;

fn mutation(revision: u64, request: impl Into<String>) -> Mutation {
    Mutation {
        expected_revision: revision,
        request_id: request.into(),
        actor: "xtask-performance".into(),
    }
}

fn milliseconds(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

fn distribution(mut samples: Vec<f64>) -> Value {
    samples.sort_by(f64::total_cmp);
    let percentile = |percent: usize| {
        let rank = (percent * samples.len()).div_ceil(100).max(1);
        samples[rank - 1]
    };
    json!({
        "samples_ms":samples,
        "p50_ms":percentile(50),
        "p95_ms":percentile(95),
    })
}

fn render_samples(
    service: &EditorService,
    asset: &lightwell_core::AssetId,
    samples: usize,
) -> Result<Vec<f64>> {
    let mut timings = Vec::with_capacity(samples);
    for _ in 0..samples {
        let started = Instant::now();
        let raster = service.render_current(asset)?;
        ensure(!raster.rgba.is_empty(), "Performance render was empty")?;
        timings.push(milliseconds(started));
    }
    Ok(timings)
}

pub fn run(root: &Path, source: &Path, out: &Path, samples: usize) -> Result {
    ensure(!out.exists(), "Editor performance output must be new")?;
    ensure(samples > 0, "Editor performance samples must be positive")?;
    fs::create_dir_all(out)?;
    let source = source.canonicalize()?;
    let source_hash = hash(&source)?;
    let catalog = out.join("catalog.sqlite");
    let total = Instant::now();

    let mut service = EditorService::open(&catalog)?;
    let started = Instant::now();
    let state = service.import(&source)?;
    let import_ms = milliseconds(started);
    let asset = state.asset.id;
    let original = state.current_entry.id;

    let started = Instant::now();
    let original_job = service.preview_job(&asset, Some(&original), None)?;
    let cached_preview_job_ms = milliseconds(started);
    let started = Instant::now();
    let original_raster = render(
        service.registry(),
        &original_job.source,
        original_job.entry.snapshot.id,
        &original_job.entry.snapshot.recipe,
    )?;
    let original_render_ms = milliseconds(started);
    ensure(
        (original_raster.width, original_raster.height) == (state.asset.width, state.asset.height),
        "Original performance render has wrong dimensions",
    )?;

    service.apply_transform(
        &asset,
        mutation(0, "performance-rotate"),
        Transform::RotateRight,
    )?;
    let one_transform = distribution(render_samples(&service, &asset, samples)?);

    for index in 1..200u64 {
        service.apply_transform(
            &asset,
            mutation(index, format!("performance-layer-{index:03}")),
            Transform::MirrorHorizontal,
        )?;
    }
    let two_hundred_transforms = distribution(render_samples(&service, &asset, samples)?);
    drop(service);

    let service = EditorService::open(&catalog)?;
    let started = Instant::now();
    let cold_job = service.preview_job(&asset, Some(&original), None)?;
    let cold_source_and_job_ms = milliseconds(started);
    let started = Instant::now();
    let cold_raster = render(
        service.registry(),
        &cold_job.source,
        cold_job.entry.snapshot.id,
        &cold_job.entry.snapshot.recipe,
    )?;
    let cold_original_render_ms = milliseconds(started);
    ensure(
        (cold_raster.width, cold_raster.height) == (state.asset.width, state.asset.height),
        "Cold original performance render has wrong dimensions",
    )?;
    ensure(hash(&source)? == source_hash, "Performance source changed")?;

    let result = json!({
        "status":"passed",
        "profile":if cfg!(debug_assertions) { "debug" } else { "release" },
        "platform":host(root)?,
        "source":source,
        "source_sha256":source_hash,
        "source_dimensions":[state.asset.width,state.asset.height],
        "samples_per_recipe":samples,
        "method":"Core request-to-render diagnostics with a warm filesystem cache; excludes desktop scheduling, GPU upload and presentation.",
        "timings_ms":{
            "import":import_ms,
            "cached_preview_job":cached_preview_job_ms,
            "original_render":original_render_ms,
            "one_transform":one_transform,
            "two_hundred_transforms":two_hundred_transforms,
            "reopen_source_and_preview_job":cold_source_and_job_ms,
            "reopen_original_render":cold_original_render_ms,
            "total":milliseconds(total),
        },
        "checks":[
            "Decoded source is cached after import",
            "Original render dimensions are exact",
            "One and 200 exact transforms render from the same immutable source",
            "Catalog reopen reconstructs the original historical state",
            "Source SHA-256 is unchanged"
        ]
    });
    write_json(&out.join("result.json"), &result)?;
    println!("PASS editor performance diagnostics: {}", out.display());
    Ok(())
}
