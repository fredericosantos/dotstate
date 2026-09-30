//! CLI integration tests for `dotstate packages ... --common`.
//!
//! These spawn the real `dotstate` binary. Isolation is the whole point, so every spawn goes
//! through [`Cli::run`], which points every directory the binary can touch at the test's temp dir:
//!
//! * `DOTSTATE_TEST_HOME` / `DOTSTATE_TEST_CONFIG_DIR`: honoured by `get_home_dir()` /
//!   `get_config_dir()`, so the config file, package cache and symlink tracking live in the temp dir.
//! * `HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, `XDG_DATA_HOME`: belt and braces for anything that
//!   goes through `dirs::*` directly (the CLI log file lives under `dirs::cache_dir()`).
//! * `DOTSTATE_GITHUB_TOKEN` is removed and stdin is `/dev/null`, so nothing can prompt or reach a
//!   remote.
//!
//! The config file written by `TestEnv` points at a temp storage repo (`.git` + a valid
//! `.dotstate-profiles.toml`). `isolation_redirects_log_and_config_into_temp_dir` proves the
//! redirection actually happens by checking that the binary wrote its log file inside the temp dir.
//!
//! All packages use the `custom` manager with `--existence-check true|false` and
//! `--install-command false`, so no real package manager is invoked and nothing can be installed.

mod common;

use anyhow::Result;
use common::TestEnv;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

struct Cli {
    env: TestEnv,
    cache_dir: PathBuf,
}

struct Run {
    status_ok: bool,
    stdout: String,
    stderr: String,
}

impl Run {
    fn all(&self) -> String {
        format!("{}{}", self.stdout, self.stderr)
    }
}

impl Cli {
    fn new() -> Result<Self> {
        let env = TestEnv::new().with_profile("default").with_git().build()?;
        // `TestEnv` lays out home/, repo/, config/ and backups/ under one temp dir.
        let base = env
            .config_dir
            .parent()
            .expect("config dir has a parent")
            .to_path_buf();
        let cache_dir = base.join("cache");
        std::fs::create_dir_all(&cache_dir)?;
        std::fs::create_dir_all(base.join("xdg-config"))?;
        std::fs::create_dir_all(base.join("xdg-data"))?;
        Ok(Self { env, cache_dir })
    }

    fn base(&self) -> PathBuf {
        self.env.config_dir.parent().unwrap().to_path_buf()
    }

    fn run(&self, args: &[&str]) -> Run {
        let base = self.base();
        // Hard guard: never spawn against anything outside the temp dir.
        assert!(self.env.home_dir.starts_with(&base));
        assert!(self.env.config_dir.starts_with(&base));
        assert!(self.env.repo_path.starts_with(&base));

        let out: Output = Command::new(env!("CARGO_BIN_EXE_dotstate"))
            .args(args)
            .current_dir(&base)
            .env("DOTSTATE_TEST_HOME", &self.env.home_dir)
            .env("DOTSTATE_TEST_CONFIG_DIR", &self.env.config_dir)
            .env("DOTSTATE_TEST_BACKUP_DIR", &self.env.backup_dir)
            .env("HOME", &self.env.home_dir)
            .env("XDG_CONFIG_HOME", base.join("xdg-config"))
            .env("XDG_CACHE_HOME", &self.cache_dir)
            .env("XDG_DATA_HOME", base.join("xdg-data"))
            .env("NO_COLOR", "1")
            .env_remove("DOTSTATE_GITHUB_TOKEN")
            .stdin(Stdio::null())
            .output()
            .expect("failed to spawn dotstate binary");
        Run {
            status_ok: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    }

    /// `packages add` with every value passed explicitly (no stdin prompts).
    fn add(&self, scope: &[&str], name: &str, existence_check: &str) -> Run {
        let mut args = vec!["packages", "add"];
        args.extend_from_slice(scope);
        args.extend_from_slice(&[
            "--name",
            name,
            "--manager",
            "custom",
            "--binary",
            name,
            "--description",
            "",
            "--install-command",
            "false",
            "--existence-check",
            existence_check,
        ]);
        self.run(&args)
    }
}

#[test]
fn isolation_redirects_log_and_config_into_temp_dir() -> Result<()> {
    let cli = Cli::new()?;
    let r = cli.run(&["packages", "list", "--common"]);
    assert!(r.status_ok, "{}", r.all());
    assert!(
        r.stdout.contains("No common packages configured"),
        "{}",
        r.all()
    );
    // The binary logs under dirs::cache_dir(); finding the log here proves XDG redirection works.
    assert!(
        cli.cache_dir.join("dotstate").join("dotstate.log").exists(),
        "log file not written under the temp cache dir"
    );
    Ok(())
}

#[test]
fn add_common_then_list_common_shows_it() -> Result<()> {
    let cli = Cli::new()?;
    let r = cli.add(&["--common"], "mytool", "true");
    assert!(r.status_ok, "{}", r.all());
    assert!(
        r.stdout.contains("Common package 'mytool' added"),
        "{}",
        r.all()
    );

    let r = cli.run(&["packages", "list", "--common"]);
    assert!(r.status_ok, "{}", r.all());
    assert!(r.stdout.contains("mytool"), "{}", r.all());
    assert!(r.stdout.contains("custom"), "{}", r.all());

    // It was persisted to the temp repo's manifest, not to the profile's own list.
    let manifest = cli.env.load_manifest()?;
    assert_eq!(manifest.common.packages.len(), 1);
    assert!(manifest.profiles[0].packages.is_empty());

    // And the profile list does not show it.
    let r = cli.run(&["packages", "list", "--profile", "default"]);
    assert!(r.status_ok, "{}", r.all());
    assert!(!r.stdout.contains("mytool"), "{}", r.all());
    Ok(())
}

#[test]
fn add_common_duplicate_name_fails_with_service_message() -> Result<()> {
    let cli = Cli::new()?;
    assert!(cli.add(&["--common"], "mytool", "true").status_ok);

    let r = cli.add(&["--common"], "mytool", "true");
    assert!(!r.status_ok, "duplicate must exit non-zero: {}", r.all());
    assert!(
        r.stderr
            .contains("Package 'mytool' is already defined in common"),
        "{}",
        r.all()
    );
    assert_eq!(cli.env.load_manifest()?.common.packages.len(), 1);
    Ok(())
}

#[test]
fn add_profile_package_clashing_with_common_fails_naming_common() -> Result<()> {
    let cli = Cli::new()?;
    assert!(cli.add(&["--common"], "mytool", "true").status_ok);

    let r = cli.add(&["--profile", "default"], "mytool", "true");
    assert!(!r.status_ok, "clash must exit non-zero: {}", r.all());
    assert!(
        r.stderr.contains("already defined in common"),
        "{}",
        r.all()
    );
    assert!(cli.env.load_manifest()?.profiles[0].packages.is_empty());
    Ok(())
}

#[test]
fn add_common_clashing_with_profile_package_fails_naming_profile() -> Result<()> {
    let cli = Cli::new()?;
    assert!(
        cli.add(&["--profile", "default"], "mytool", "true")
            .status_ok
    );

    let r = cli.add(&["--common"], "mytool", "true");
    assert!(!r.status_ok, "clash must exit non-zero: {}", r.all());
    assert!(r.stderr.contains("in profile 'default'"), "{}", r.all());
    assert!(cli.env.load_manifest()?.common.packages.is_empty());
    Ok(())
}

#[test]
fn remove_common_removes_it() -> Result<()> {
    let cli = Cli::new()?;
    assert!(cli.add(&["--common"], "mytool", "true").status_ok);
    assert!(cli.add(&["--common"], "other", "true").status_ok);

    let r = cli.run(&["packages", "remove", "--common", "--yes", "mytool"]);
    assert!(r.status_ok, "{}", r.all());
    assert!(
        r.stdout.contains("Common package 'mytool' removed"),
        "{}",
        r.all()
    );

    let names: Vec<_> = cli
        .env
        .load_manifest()?
        .common
        .packages
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, ["other"]);

    // Removing something that is not there is an error.
    let r = cli.run(&["packages", "remove", "--common", "--yes", "mytool"]);
    assert!(!r.status_ok, "{}", r.all());
    assert!(r.stderr.contains("not found"), "{}", r.all());
    Ok(())
}

#[test]
fn common_and_profile_flags_conflict() -> Result<()> {
    let cli = Cli::new()?;
    for sub in ["list", "add", "remove", "check", "install"] {
        let r = cli.run(&["packages", sub, "--common", "--profile", "default"]);
        assert!(!r.status_ok, "`{sub}` accepted --common with --profile");
        assert!(
            r.stderr.contains("cannot be used with"),
            "{sub}: {}",
            r.all()
        );
    }
    // Nothing was written.
    assert!(cli.env.load_manifest()?.common.packages.is_empty());
    Ok(())
}

#[test]
fn check_common_operates_on_the_common_list() -> Result<()> {
    let cli = Cli::new()?;
    assert!(cli.add(&["--common"], "present", "true").status_ok);
    assert!(cli.add(&["--common"], "absent", "false").status_ok);
    // A profile package that must not show up in the common check.
    assert!(
        cli.add(&["--profile", "default"], "profileonly", "true")
            .status_ok
    );

    let r = cli.run(&["packages", "check", "--common"]);
    assert!(r.status_ok, "{}", r.all());
    assert!(r.stdout.contains("Checking common packages"), "{}", r.all());
    assert!(r.stdout.contains("present"), "{}", r.all());
    assert!(r.stdout.contains("absent"), "{}", r.all());
    assert!(!r.stdout.contains("profileonly"), "{}", r.all());
    assert!(
        r.stdout.contains("1 of 2 packages installed (1 missing)"),
        "{}",
        r.all()
    );
    Ok(())
}

#[test]
fn install_common_operates_on_common_list_without_installing() -> Result<()> {
    let cli = Cli::new()?;
    // Everything is "already installed" (existence check `true`), so the installer is never
    // reached; the install command is `false` anyway so nothing could be installed.
    assert!(cli.add(&["--common"], "present", "true").status_ok);

    let r = cli.run(&["packages", "install", "--common"]);
    assert!(r.status_ok, "{}", r.all());
    assert!(
        r.stdout.contains("All packages are already installed"),
        "{}",
        r.all()
    );

    // With no common packages the command says so, proving it reads the common list.
    let r = cli.run(&["packages", "remove", "--common", "--yes", "present"]);
    assert!(r.status_ok, "{}", r.all());
    let r = cli.run(&["packages", "install", "--common"]);
    assert!(r.status_ok, "{}", r.all());
    assert!(
        r.stdout.contains("No common packages configured"),
        "{}",
        r.all()
    );
    Ok(())
}
