use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Current version of the manifest file format.
/// Increment this when making breaking changes to the schema.
const CURRENT_VERSION: u32 = 1;

/// Package manager types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Brew,   // Homebrew (macOS/Linux)
    Apt,    // Advanced Package Tool (Debian/Ubuntu)
    Yum,    // Yellowdog Updater Modified (RHEL/CentOS)
    Dnf,    // Dandified Yum (Fedora)
    Pacman, // Arch Linux
    Snap,   // Snap packages
    Cargo,  // Rust packages
    Npm,    // Node.js packages
    Pip,    // Python packages (pip)
    Pip3,   // Python packages (pip3)
    Gem,    // Ruby gems
    Custom, // Custom install command
}

/// Package definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Display name for the package
    pub name: String,
    /// Optional description (cached metadata, not required)
    #[serde(default)]
    pub description: Option<String>,
    /// Package manager type
    pub manager: PackageManager,
    /// Package name in the manager (e.g., "eza" for brew)
    /// None for custom packages
    #[serde(default)]
    pub package_name: Option<String>,
    /// Binary name to check for existence (cached, can be derived but stored for performance)
    /// For packages with multiple binaries, this is the primary one
    pub binary_name: String,
    /// Install command (only for custom packages, derived for managed packages)
    #[serde(default)]
    pub install_command: Option<String>,
    /// Command to check if package exists (optional for custom packages, derived for managed packages)
    /// If None or empty for custom packages, the system will perform a standard existence check
    /// derived from the binary name (checking if binary exists in PATH)
    #[serde(default)]
    pub existence_check: Option<String>,
    /// Optional manager-native check command (fallback when `binary_name` check fails)
    /// e.g., "brew list eza" or "dpkg -s git"
    #[serde(default)]
    pub manager_check: Option<String>,
}

/// Common section for files and packages shared across all profiles
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CommonSection {
    /// Files synced to all profiles (relative paths from home directory)
    #[serde(default)]
    pub synced_files: Vec<String>,
    /// Packages shared across all profiles
    #[serde(default)]
    pub packages: Vec<Package>,
}

/// Reserved profile names that cannot be used
pub const RESERVED_PROFILE_NAMES: &[&str] = &["common"];

/// Profile manifest stored in the repository root
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileManifest {
    /// Schema version for migration support. Missing = v0.
    #[serde(default)]
    pub version: u32,
    /// Common files shared across all profiles
    #[serde(default)]
    pub common: CommonSection,
    /// List of profile names
    #[serde(default)]
    pub profiles: Vec<ProfileInfo>,
}

impl Default for ProfileManifest {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            common: CommonSection::default(),
            profiles: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    /// Profile name (must match folder name)
    pub name: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Files synced for this profile (relative paths from home directory)
    #[serde(default)]
    pub synced_files: Vec<String>,
    /// Packages/dependencies for this profile
    #[serde(default)]
    pub packages: Vec<Package>,
}

impl ProfileManifest {
    /// Get the path to the manifest file in the repo
    #[must_use]
    pub fn manifest_path(repo_path: &Path) -> PathBuf {
        repo_path.join(".dotstate-profiles.toml")
    }

    /// Load the manifest from the repository.
    /// Automatically migrates old manifest versions to the current version.
    pub fn load(repo_path: &Path) -> Result<Self> {
        let manifest_path = Self::manifest_path(repo_path);

        if manifest_path.exists() {
            let content = std::fs::read_to_string(&manifest_path)
                .with_context(|| format!("Failed to read profile manifest: {manifest_path:?}"))?;
            let mut manifest: ProfileManifest =
                toml::from_str(&content).with_context(|| "Failed to parse profile manifest")?;

            // Migrate if needed
            if manifest.version < CURRENT_VERSION {
                let old_version = manifest.version;
                tracing::info!(
                    "Migrating manifest from v{} to v{}",
                    old_version,
                    CURRENT_VERSION
                );
                manifest = Self::migrate(manifest)?;

                // Backup, save, cleanup
                super::migrate_file(&manifest_path, old_version, "toml", || {
                    manifest.save(repo_path)
                })?;
            }

            // Sort synced_files alphabetically to ensure consistent ordering
            manifest.common.synced_files.sort();
            manifest.common.packages.sort_by(|a, b| a.name.cmp(&b.name));
            for profile in &mut manifest.profiles {
                profile.synced_files.sort();
            }

            Ok(manifest)
        } else {
            // Return empty manifest if file doesn't exist
            Ok(Self::default())
        }
    }

    /// Backfill manifest from existing profile folders in the repo
    /// This is useful for repos created before the manifest system was added
    pub fn backfill_from_repo(repo_path: &Path) -> Result<Self> {
        let mut manifest = Self::default();

        // Scan repo directory for profile folders
        // Profile folders are directories at the repo root that aren't .git or other system files
        if let Ok(entries) = std::fs::read_dir(repo_path) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Skip if not a directory
                if !path.is_dir() {
                    continue;
                }

                let name = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n,
                    None => continue,
                };

                // Skip system directories
                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }

                // Check if this looks like a profile/common folder (has files in it)
                if let Ok(dir_entries) = std::fs::read_dir(&path) {
                    let has_files = dir_entries.into_iter().next().is_some();
                    if has_files {
                        if name == "common" {
                            // This is the common folder - backfill common files
                            if let Ok(common_files) = Self::scan_folder_files(&path) {
                                manifest.common.synced_files = common_files;
                            }
                        } else {
                            // This looks like a profile folder
                            manifest.add_profile(name.to_string(), None);
                        }
                    }
                }
            }
        }

        Ok(manifest)
    }

    /// Scan a folder for files (used during backfill)
    fn scan_folder_files(folder_path: &Path) -> Result<Vec<String>> {
        let mut files = Vec::new();
        Self::scan_folder_files_recursive(folder_path, folder_path, &mut files)?;
        files.sort();
        Ok(files)
    }

    /// Recursively scan folder for files
    fn scan_folder_files_recursive(
        base_path: &Path,
        current_path: &Path,
        files: &mut Vec<String>,
    ) -> Result<()> {
        if let Ok(entries) = std::fs::read_dir(current_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                let relative = path
                    .strip_prefix(base_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();

                if path.is_dir() {
                    // Recurse into subdirectories
                    Self::scan_folder_files_recursive(base_path, &path, files)?;
                } else {
                    files.push(relative);
                }
            }
        }
        Ok(())
    }

    /// Update packages for a profile
    #[allow(dead_code)] // Reserved for future use
    pub fn update_packages(&mut self, profile_name: &str, packages: Vec<Package>) -> Result<()> {
        if let Some(profile) = self.profiles.iter_mut().find(|p| p.name == profile_name) {
            profile.packages = packages;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Profile '{profile_name}' not found in manifest"
            ))
        }
    }

    /// Load manifest, backfilling from repo if it doesn't exist
    pub fn load_or_backfill(repo_path: &Path) -> Result<Self> {
        let manifest_path = Self::manifest_path(repo_path);

        if manifest_path.exists() {
            Self::load(repo_path)
        } else {
            // Manifest doesn't exist, try to backfill from existing folders
            let manifest = Self::backfill_from_repo(repo_path)?;

            // Save the backfilled manifest so it's available next time
            if !manifest.profiles.is_empty() {
                manifest.save(repo_path)?;
            }

            Ok(manifest)
        }
    }

    /// Save the manifest to the repository.
    /// Uses atomic write (temp file + rename) to prevent corruption on crash.
    pub fn save(&self, repo_path: &Path) -> Result<()> {
        let manifest_path = Self::manifest_path(repo_path);
        let temp_path = manifest_path.with_extension("toml.tmp");

        let content =
            toml::to_string_pretty(self).with_context(|| "Failed to serialize profile manifest")?;

        // Write to temp file first
        std::fs::write(&temp_path, &content)
            .with_context(|| format!("Failed to write temp manifest: {temp_path:?}"))?;

        // Atomic rename (on POSIX systems)
        std::fs::rename(&temp_path, &manifest_path)
            .with_context(|| format!("Failed to rename temp manifest to {manifest_path:?}"))?;

        Ok(())
    }

    /// Add a profile to the manifest
    pub fn add_profile(&mut self, name: String, description: Option<String>) {
        // Check if profile already exists
        if !self.profiles.iter().any(|p| p.name == name) {
            self.profiles.push(ProfileInfo {
                name,
                description,
                synced_files: Vec::new(),
                packages: Vec::new(),
            });
        }
    }

    /// Update synced files for a profile
    pub fn update_synced_files(
        &mut self,
        profile_name: &str,
        synced_files: Vec<String>,
    ) -> Result<()> {
        if let Some(profile) = self.profiles.iter_mut().find(|p| p.name == profile_name) {
            // Sort alphabetically to ensure consistent ordering and prevent unnecessary diffs
            let mut sorted_files = synced_files;
            sorted_files.sort();
            profile.synced_files = sorted_files;
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Profile '{profile_name}' not found in manifest"
            ))
        }
    }

    // get_synced_files method removed - access synced_files directly from ProfileInfo

    /// Remove a profile from the manifest
    pub fn remove_profile(&mut self, name: &str) -> bool {
        let initial_len = self.profiles.len();
        self.profiles.retain(|p| p.name != name);
        self.profiles.len() < initial_len
    }

    /// Update a profile's name (for rename)
    pub fn rename_profile(&mut self, old_name: &str, new_name: &str) -> Result<()> {
        if let Some(profile) = self.profiles.iter_mut().find(|p| p.name == old_name) {
            profile.name = new_name.to_string();
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Profile '{old_name}' not found in manifest"
            ))
        }
    }

    /// Get all profile names
    #[allow(dead_code)] // Kept for potential future use in CLI or programmatic access
    #[must_use]
    pub fn profile_names(&self) -> Vec<String> {
        self.profiles.iter().map(|p| p.name.clone()).collect()
    }

    /// Check if a profile exists in the manifest
    #[allow(dead_code)] // Kept for potential future use in CLI or programmatic access
    #[must_use]
    pub fn has_profile(&self, name: &str) -> bool {
        self.profiles.iter().any(|p| p.name == name)
    }

    /// Check if a name is reserved and cannot be used as a profile name
    #[must_use]
    pub fn is_reserved_name(name: &str) -> bool {
        RESERVED_PROFILE_NAMES.contains(&name.to_lowercase().as_str())
    }

    /// Add a file to the common section
    pub fn add_common_file(&mut self, relative_path: &str) {
        let path = relative_path.to_string();
        if !self.common.synced_files.contains(&path) {
            self.common.synced_files.push(path);
            self.common.synced_files.sort();
        }
    }

    /// Remove a file from the common section
    pub fn remove_common_file(&mut self, relative_path: &str) -> bool {
        let initial_len = self.common.synced_files.len();
        self.common.synced_files.retain(|f| f != relative_path);
        self.common.synced_files.len() < initial_len
    }

    /// Get all common files
    #[must_use]
    pub fn get_common_files(&self) -> &[String] {
        &self.common.synced_files
    }

    /// Check if a file is in the common section
    #[must_use]
    pub fn is_common_file(&self, relative_path: &str) -> bool {
        self.common
            .synced_files
            .contains(&relative_path.to_string())
    }

    // ==================== Migration Methods ====================

    /// Run all necessary migrations to bring manifest to current version.
    fn migrate(mut manifest: Self) -> Result<Self> {
        if manifest.version == 0 {
            manifest = Self::migrate_v0_to_v1(manifest)?;
        }
        // Future migrations:
        // if manifest.version == 1 { manifest = Self::migrate_v1_to_v2(manifest)?; }
        Ok(manifest)
    }

    /// Migrate from v0 (no version field) to v1.
    /// This is a no-op migration that just sets the version field.
    fn migrate_v0_to_v1(mut manifest: Self) -> Result<Self> {
        tracing::debug!("Migrating manifest v0 -> v1");
        manifest.version = 1;
        Ok(manifest)
    }

    /// Move a file from a profile to common
    pub fn move_to_common(&mut self, profile_name: &str, relative_path: &str) -> Result<()> {
        // Remove from profile
        if let Some(profile) = self.profiles.iter_mut().find(|p| p.name == profile_name) {
            profile.synced_files.retain(|f| f != relative_path);
        } else {
            return Err(anyhow::anyhow!(
                "Profile '{profile_name}' not found in manifest"
            ));
        }

        // Add to common
        self.add_common_file(relative_path);
        Ok(())
    }

    /// Move a file from common to a profile
    pub fn move_from_common(&mut self, profile_name: &str, relative_path: &str) -> Result<()> {
        // Remove from common
        if !self.remove_common_file(relative_path) {
            return Err(anyhow::anyhow!(
                "File '{relative_path}' not found in common section"
            ));
        }

        // Add to profile
        if let Some(profile) = self.profiles.iter_mut().find(|p| p.name == profile_name) {
            if !profile.synced_files.contains(&relative_path.to_string()) {
                profile.synced_files.push(relative_path.to_string());
                profile.synced_files.sort();
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Profile '{profile_name}' not found in manifest"
            ))
        }
    }

    // ==================== Common Package Methods ====================

    /// Add a package to the common section (shared across all profiles).
    /// Deduplicates by name; keeps list sorted alphabetically.
    pub fn add_common_package(&mut self, package: Package) {
        if !self.common.packages.iter().any(|p| p.name == package.name) {
            self.common.packages.push(package);
            self.common.packages.sort_by(|a, b| a.name.cmp(&b.name));
        }
    }

    /// Remove a package from the common section by name.
    /// Returns true if the package was found and removed.
    pub fn remove_common_package(&mut self, name: &str) -> bool {
        let initial_len = self.common.packages.len();
        self.common.packages.retain(|p| p.name != name);
        self.common.packages.len() < initial_len
    }

    /// Get all common packages.
    #[must_use]
    pub fn get_common_packages(&self) -> &[Package] {
        &self.common.packages
    }

    /// Check if a package with the given name exists in the common section.
    #[must_use]
    pub fn is_common_package(&self, name: &str) -> bool {
        self.common.packages.iter().any(|p| p.name == name)
    }

    /// Move a package from a profile to the common section.
    pub fn move_package_to_common(&mut self, profile_name: &str, index: usize) -> Result<()> {
        // Extract the package from the profile first (releases mutable borrow)
        let package =
            if let Some(profile) = self.profiles.iter_mut().find(|p| p.name == profile_name) {
                if index < profile.packages.len() {
                    profile.packages.remove(index)
                } else {
                    return Err(anyhow::anyhow!(
                        "Package index {index} out of bounds in profile '{profile_name}'"
                    ));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Profile '{profile_name}' not found in manifest"
                ));
            };

        self.add_common_package(package);
        Ok(())
    }

    /// Move a package from the common section (by index) to a profile.
    pub fn move_package_from_common_by_index(
        &mut self,
        profile_name: &str,
        index: usize,
    ) -> Result<()> {
        if index >= self.common.packages.len() {
            return Err(anyhow::anyhow!(
                "Common package index {index} out of bounds"
            ));
        }

        let package = self.common.packages.remove(index);

        if let Some(profile) = self.profiles.iter_mut().find(|p| p.name == profile_name) {
            if !profile.packages.iter().any(|p| p.name == package.name) {
                profile.packages.push(package);
            }
            Ok(())
        } else {
            // Restore the package to common if profile not found
            self.common.packages.insert(index, package);
            Err(anyhow::anyhow!(
                "Profile '{profile_name}' not found in manifest"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_profile_manifest() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Create new manifest
        let mut manifest = ProfileManifest::default();

        // Add profiles
        manifest.add_profile("Personal".to_string(), Some("Personal Mac".to_string()));
        manifest.add_profile("Work".to_string(), None);

        // Add packages to a profile
        let packages = vec![Package {
            name: "eza".to_string(),
            description: Some("Modern replacement for ls".to_string()),
            manager: PackageManager::Brew,
            package_name: Some("eza".to_string()),
            binary_name: "eza".to_string(),
            install_command: None,
            existence_check: None,
            manager_check: None,
        }];
        manifest.update_packages("Personal", packages).unwrap();

        // Save
        manifest.save(repo_path).unwrap();

        // Load
        let mut loaded = ProfileManifest::load(repo_path).unwrap();
        assert_eq!(loaded.profiles.len(), 2);
        assert!(loaded.has_profile("Personal"));
        assert!(loaded.has_profile("Work"));

        // Rename
        loaded.rename_profile("Personal", "Personal-Mac").unwrap();
        assert!(!loaded.has_profile("Personal"));
        assert!(loaded.has_profile("Personal-Mac"));

        // Remove
        loaded.remove_profile("Work");
        assert!(!loaded.has_profile("Work"));
    }

    #[test]
    fn test_reserved_names() {
        assert!(ProfileManifest::is_reserved_name("common"));
        assert!(ProfileManifest::is_reserved_name("Common"));
        assert!(ProfileManifest::is_reserved_name("COMMON"));
        assert!(!ProfileManifest::is_reserved_name("work"));
        assert!(!ProfileManifest::is_reserved_name("personal"));
    }

    #[test]
    fn test_common_files() {
        let mut manifest = ProfileManifest::default();

        // Add common files
        manifest.add_common_file(".gitconfig");
        manifest.add_common_file(".tmux.conf");
        assert_eq!(manifest.get_common_files().len(), 2);
        assert!(manifest.is_common_file(".gitconfig"));
        assert!(manifest.is_common_file(".tmux.conf"));

        // Adding duplicate should not increase count
        manifest.add_common_file(".gitconfig");
        assert_eq!(manifest.get_common_files().len(), 2);

        // Remove common file
        assert!(manifest.remove_common_file(".tmux.conf"));
        assert_eq!(manifest.get_common_files().len(), 1);
        assert!(!manifest.is_common_file(".tmux.conf"));

        // Remove non-existent should return false
        assert!(!manifest.remove_common_file(".nonexistent"));
    }

    #[test]
    fn test_move_to_common() {
        let mut manifest = ProfileManifest::default();
        manifest.add_profile("work".to_string(), None);

        // Add file to profile
        manifest
            .update_synced_files("work", vec![".zshrc".to_string()])
            .unwrap();

        // Move to common
        manifest.move_to_common("work", ".zshrc").unwrap();

        // Verify file is in common and not in profile
        assert!(manifest.is_common_file(".zshrc"));
        let profile = manifest.profiles.iter().find(|p| p.name == "work").unwrap();
        assert!(!profile.synced_files.contains(&".zshrc".to_string()));
    }

    #[test]
    fn test_move_from_common() {
        let mut manifest = ProfileManifest::default();
        manifest.add_profile("work".to_string(), None);

        // Add file to common
        manifest.add_common_file(".gitconfig");

        // Move to profile
        manifest.move_from_common("work", ".gitconfig").unwrap();

        // Verify file is in profile and not in common
        assert!(!manifest.is_common_file(".gitconfig"));
        let profile = manifest.profiles.iter().find(|p| p.name == "work").unwrap();
        assert!(profile.synced_files.contains(&".gitconfig".to_string()));
    }

    #[test]
    fn test_manifest_migration_v0_to_v1() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Write a v0 manifest (no version field)
        let v0_manifest = r#"
[common]
synced_files = [".gitconfig"]

[[profiles]]
name = "work"
synced_files = [".zshrc"]
"#;
        std::fs::write(ProfileManifest::manifest_path(repo_path), v0_manifest).unwrap();

        // Load should auto-migrate to v1
        let loaded = ProfileManifest::load(repo_path).unwrap();
        assert_eq!(loaded.version, 1);
        assert!(loaded.is_common_file(".gitconfig"));
        assert!(loaded.has_profile("work"));

        // File should be updated with version
        let content = std::fs::read_to_string(ProfileManifest::manifest_path(repo_path)).unwrap();
        assert!(content.contains("version = 1"));

        // Backup should be cleaned up
        let backup_path =
            ProfileManifest::manifest_path(repo_path).with_extension("toml.backup-v0");
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_manifest_already_at_current_version() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Write a v1 manifest
        let v1_manifest = r#"
version = 1

[common]
synced_files = []

[[profiles]]
name = "test"
synced_files = []
"#;
        std::fs::write(ProfileManifest::manifest_path(repo_path), v1_manifest).unwrap();

        // Load should not create backup (no migration needed)
        let loaded = ProfileManifest::load(repo_path).unwrap();
        assert_eq!(loaded.version, 1);

        // No backup should exist
        let backup_path =
            ProfileManifest::manifest_path(repo_path).with_extension("toml.backup-v0");
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_new_manifest_has_current_version() {
        let manifest = ProfileManifest::default();
        assert_eq!(manifest.version, 1);
    }

    fn make_package(name: &str) -> Package {
        Package {
            name: name.to_string(),
            description: None,
            manager: PackageManager::Brew,
            package_name: Some(name.to_string()),
            binary_name: name.to_string(),
            install_command: None,
            existence_check: None,
            manager_check: None,
        }
    }

    #[test]
    fn test_common_packages() {
        let mut manifest = ProfileManifest::default();

        manifest.add_common_package(make_package("git"));
        manifest.add_common_package(make_package("ripgrep"));
        assert_eq!(manifest.get_common_packages().len(), 2);
        assert!(manifest.is_common_package("git"));
        assert!(manifest.is_common_package("ripgrep"));

        // Adding duplicate should not increase count
        manifest.add_common_package(make_package("git"));
        assert_eq!(manifest.get_common_packages().len(), 2);

        // Remove
        assert!(manifest.remove_common_package("ripgrep"));
        assert_eq!(manifest.get_common_packages().len(), 1);
        assert!(!manifest.is_common_package("ripgrep"));

        // Remove non-existent returns false
        assert!(!manifest.remove_common_package("nonexistent"));
    }

    #[test]
    fn test_common_packages_sorted() {
        let mut manifest = ProfileManifest::default();
        manifest.add_common_package(make_package("zsh"));
        manifest.add_common_package(make_package("awk"));
        manifest.add_common_package(make_package("git"));
        let names: Vec<&str> = manifest
            .get_common_packages()
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(names, vec!["awk", "git", "zsh"]);
    }

    #[test]
    fn test_move_package_to_common() {
        let mut manifest = ProfileManifest::default();
        manifest.add_profile("work".to_string(), None);
        if let Some(p) = manifest.profiles.iter_mut().find(|p| p.name == "work") {
            p.packages.push(make_package("git"));
        }

        manifest.move_package_to_common("work", 0).unwrap();

        assert!(manifest.is_common_package("git"));
        let profile = manifest.profiles.iter().find(|p| p.name == "work").unwrap();
        assert!(profile.packages.is_empty());
    }

    #[test]
    fn test_move_package_from_common_by_index() {
        let mut manifest = ProfileManifest::default();
        manifest.add_profile("work".to_string(), None);
        manifest.add_common_package(make_package("git"));

        manifest
            .move_package_from_common_by_index("work", 0)
            .unwrap();

        assert!(!manifest.is_common_package("git"));
        let profile = manifest.profiles.iter().find(|p| p.name == "work").unwrap();
        assert_eq!(profile.packages.len(), 1);
        assert_eq!(profile.packages[0].name, "git");
    }

    #[test]
    fn test_common_packages_toml_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        let mut manifest = ProfileManifest::default();
        manifest.add_common_package(make_package("git"));
        manifest.add_common_package(make_package("ripgrep"));
        manifest.save(repo_path).unwrap();

        let loaded = ProfileManifest::load(repo_path).unwrap();
        assert_eq!(loaded.get_common_packages().len(), 2);
        assert!(loaded.is_common_package("git"));
        assert!(loaded.is_common_package("ripgrep"));
    }

    #[test]
    fn test_common_packages_backward_compat() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();

        // Old manifest without packages in [common]
        let old_manifest = r#"
version = 1

[common]
synced_files = [".gitconfig"]

[[profiles]]
name = "work"
synced_files = []
"#;
        std::fs::write(ProfileManifest::manifest_path(repo_path), old_manifest).unwrap();

        let loaded = ProfileManifest::load(repo_path).unwrap();
        assert_eq!(loaded.get_common_packages().len(), 0);
        assert!(loaded.is_common_file(".gitconfig"));
    }
}
