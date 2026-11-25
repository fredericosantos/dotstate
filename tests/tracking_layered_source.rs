//! Regression: a common file overridden by a parent layer must be tracked
//! with the overriding (most specific) source, exactly once.
//!
//! Setup mirrors common -> macos -> macbook-pro, where `macos` overrides
//! the common `.claude/claude.d/os.md`.
//!
//! Run serially (env overrides): `cargo test --test tracking_layered_source -- --test-threads=1`

mod common;

use anyhow::Result;
use common::TestEnv;
use dotstate::services::ProfileService;
use dotstate::utils::symlink_manager::TrackedSymlink;

const FILE: &str = ".claude/claude.d/os.md";

fn layered_env() -> Result<TestEnv> {
    let env = TestEnv::new()
        .with_profile("macos")
        .with_profile("macbook-pro")
        .with_common_file(FILE, "common")
        .with_synced_file("macos", FILE, "macos")
        .with_env_override()
        .build()?;
    let mut manifest = env.load_manifest()?;
    manifest
        .profiles
        .iter_mut()
        .find(|p| p.name == "macbook-pro")
        .unwrap()
        .inherits = Some("macos".to_string());
    manifest.save(&env.repo_path)?;
    Ok(env)
}

fn entries_for(env: &TestEnv) -> Result<Vec<TrackedSymlink>> {
    let target = env.home_path(FILE);
    Ok(env
        .load_tracking()?
        .symlinks
        .into_iter()
        .filter(|s| s.target == target)
        .collect())
}

fn assert_tracked_once_from_macos(env: &TestEnv) -> Result<()> {
    let expected = env.profile_file_path("macos", FILE);
    env.assert_symlink_points_to(FILE, &expected);
    let entries = entries_for(env)?;
    assert_eq!(entries.len(), 1, "duplicate tracking entries: {entries:?}");
    assert_eq!(entries[0].source, expected);
    Ok(())
}

#[test]
fn ensure_common_symlinks_skips_files_overridden_by_profile_chain() -> Result<()> {
    let env = layered_env()?;
    ProfileService::activate_profile(&env.repo_path, "macbook-pro", false)?;
    assert_tracked_once_from_macos(&env)?;

    // What `dotstate sync` runs after a pull.
    ProfileService::ensure_profile_symlinks(&env.repo_path, "macbook-pro", false)?;
    ProfileService::ensure_common_symlinks(&env.repo_path, false)?;
    assert_tracked_once_from_macos(&env)
}

#[test]
fn reactivate_replaces_stale_common_tracking_entry() -> Result<()> {
    let env = layered_env()?;
    ProfileService::activate_profile(&env.repo_path, "macbook-pro", false)?;

    // Seed the stale state found on disk: an extra entry claiming `common`.
    let mut tracking = env.load_tracking()?;
    tracking.symlinks.insert(
        0,
        TrackedSymlink {
            target: env.home_path(FILE),
            source: env.common_path().join(FILE),
            created_at: chrono::Utc::now(),
            backup: None,
        },
    );
    env.save_tracking(&tracking)?;

    // What `dotstate doctor --fix` ("Re-activate profile") runs.
    ProfileService::activate_profile(&env.repo_path, "macbook-pro", false)?;
    assert_tracked_once_from_macos(&env)
}
