//! Profile inheritance helpers shared by the profile screens.
//!
//! * [`profile_tree`] lays profiles out as a depth-first tree for any nesting depth.
//! * [`layered_files`] groups a profile's resolved files by the layer that provides them:
//!   `common`, each ancestor from the root down, then the profile's own files.
//!
//! Both are pure functions over [`ProfileInfo`] / [`ResolvedFile`], so the screens only render.

use crate::utils::profile_manifest::{ProfileInfo, ResolvedFile};
use std::collections::{HashMap, HashSet};

/// Name of the layer shared by every profile.
pub const COMMON_LAYER: &str = "common";

/// Glyphs used to draw the branches of the profile tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeGlyphs {
    /// Branch with siblings below it.
    pub tee: &'static str,
    /// Last branch of a parent.
    pub last: &'static str,
    /// Vertical guide for an ancestor that has more siblings below.
    pub pipe: &'static str,
    /// Blank guide for an ancestor that was the last sibling.
    pub blank: &'static str,
}

impl TreeGlyphs {
    pub const UNICODE: Self = Self {
        tee: "├─ ",
        last: "└─ ",
        pipe: "│  ",
        blank: "   ",
    };
    pub const ASCII: Self = Self {
        tee: "|- ",
        last: "`- ",
        pipe: "|  ",
        blank: "   ",
    };
}

/// One row of the profile tree, in display (depth-first) order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeRow {
    /// Index into the `profiles` slice given to [`profile_tree`].
    pub index: usize,
    /// 0 for a root profile (no parent), 1 for its children, and so on.
    pub depth: usize,
    /// For every ancestor below the root: whether it was the last of its siblings.
    guides: Vec<bool>,
    /// Whether this row is the last of its siblings.
    is_last: bool,
}

impl TreeRow {
    /// The branch drawing placed before the profile name. Empty for roots.
    #[must_use]
    pub fn prefix(&self, glyphs: TreeGlyphs) -> String {
        if self.depth == 0 {
            return String::new();
        }
        let mut out = String::new();
        for ancestor_was_last in &self.guides {
            out.push_str(if *ancestor_was_last {
                glyphs.blank
            } else {
                glyphs.pipe
            });
        }
        out.push_str(if self.is_last {
            glyphs.last
        } else {
            glyphs.tee
        });
        out
    }
}

/// Lay profiles out as a tree, depth first, siblings sorted by name.
///
/// A profile whose parent is missing is treated as a root, and profiles caught in an inheritance
/// cycle are appended as roots so every profile appears exactly once.
#[must_use]
pub fn profile_tree(profiles: &[ProfileInfo]) -> Vec<TreeRow> {
    let by_name: HashMap<&str, usize> = profiles
        .iter()
        .enumerate()
        .map(|(i, p)| (p.name.as_str(), i))
        .collect();

    let mut children: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut roots: Vec<usize> = Vec::new();
    for (i, p) in profiles.iter().enumerate() {
        match p.inherits.as_deref().and_then(|parent| by_name.get(parent)) {
            Some(&parent) if parent != i => children.entry(parent).or_default().push(i),
            _ => roots.push(i),
        }
    }
    let sort_by_name =
        |v: &mut Vec<usize>| v.sort_by(|a, b| profiles[*a].name.cmp(&profiles[*b].name));
    sort_by_name(&mut roots);
    for kids in children.values_mut() {
        sort_by_name(kids);
    }

    let mut rows = Vec::with_capacity(profiles.len());
    let mut seen: HashSet<usize> = HashSet::new();

    fn walk(
        node: usize,
        guides: &[bool],
        is_last: bool,
        depth: usize,
        children: &HashMap<usize, Vec<usize>>,
        seen: &mut HashSet<usize>,
        rows: &mut Vec<TreeRow>,
    ) {
        if !seen.insert(node) {
            return;
        }
        rows.push(TreeRow {
            index: node,
            depth,
            guides: guides.to_vec(),
            is_last,
        });
        let Some(kids) = children.get(&node) else {
            return;
        };
        // A root draws no guide column; deeper ancestors draw one each.
        let mut child_guides = guides.to_vec();
        if depth > 0 {
            child_guides.push(is_last);
        }
        for (n, kid) in kids.iter().enumerate() {
            walk(
                *kid,
                &child_guides,
                n + 1 == kids.len(),
                depth + 1,
                children,
                seen,
                rows,
            );
        }
    }

    for (n, root) in roots.iter().enumerate() {
        walk(
            *root,
            &[],
            n + 1 == roots.len(),
            0,
            &children,
            &mut seen,
            &mut rows,
        );
    }
    // Profiles that only appear in an inheritance cycle are never reached from a root.
    for i in 0..profiles.len() {
        walk(i, &[], true, 0, &children, &mut seen, &mut rows);
    }
    rows
}

/// Ancestors of `profile`, root first, `profile` itself excluded. Stops at a missing parent or a
/// cycle, so it always terminates.
#[must_use]
pub fn ancestor_chain(profile: &str, profiles: &[ProfileInfo]) -> Vec<String> {
    let parent_of: HashMap<&str, &str> = profiles
        .iter()
        .filter_map(|p| {
            p.inherits
                .as_deref()
                .map(|parent| (p.name.as_str(), parent))
        })
        .collect();
    let mut chain: Vec<String> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::from([profile]);
    let mut current = profile;
    while let Some(&parent) = parent_of.get(current) {
        if !seen.insert(parent) {
            break;
        }
        chain.push(parent.to_string());
        current = parent;
    }
    chain.reverse();
    chain
}

/// The files one layer contributes to a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLayer {
    /// `common`, an ancestor's name, or the profile's own name.
    pub name: String,
    /// True for the profile's own layer (the last one).
    pub is_own: bool,
    /// Relative paths, in resolved (sorted) order.
    pub files: Vec<String>,
}

/// Group `resolved` by providing layer: `common`, then each ancestor from the root down, then the
/// profile's own files. Layers with no files are left out. A file overridden further down appears
/// only under the layer that wins, which matches what actually gets linked.
#[must_use]
pub fn layered_files(
    profile: &str,
    profiles: &[ProfileInfo],
    resolved: &[ResolvedFile],
) -> Vec<FileLayer> {
    let mut order: Vec<(String, bool)> = vec![(COMMON_LAYER.to_string(), false)];
    order.extend(
        ancestor_chain(profile, profiles)
            .into_iter()
            .map(|a| (a, false)),
    );
    order.push((profile.to_string(), true));

    let mut layers: Vec<FileLayer> = order
        .into_iter()
        .map(|(name, is_own)| FileLayer {
            name,
            is_own,
            files: Vec::new(),
        })
        .collect();
    for file in resolved {
        if let Some(layer) = layers.iter_mut().find(|l| l.name == file.source_profile) {
            layer.files.push(file.relative_path.clone());
        }
    }
    layers.retain(|l| !l.files.is_empty());
    layers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(name: &str, inherits: Option<&str>) -> ProfileInfo {
        ProfileInfo {
            name: name.to_string(),
            description: None,
            synced_files: Vec::new(),
            packages: Vec::new(),
            inherits: inherits.map(str::to_string),
        }
    }

    fn resolved(path: &str, source: &str) -> ResolvedFile {
        ResolvedFile {
            relative_path: path.to_string(),
            source_profile: source.to_string(),
        }
    }

    fn names(profiles: &[ProfileInfo], rows: &[TreeRow]) -> Vec<String> {
        rows.iter()
            .map(|r| profiles[r.index].name.clone())
            .collect()
    }

    fn drawn(profiles: &[ProfileInfo], rows: &[TreeRow]) -> Vec<String> {
        rows.iter()
            .map(|r| {
                format!(
                    "{}{}",
                    r.prefix(TreeGlyphs::UNICODE),
                    profiles[r.index].name
                )
            })
            .collect()
    }

    #[test]
    fn tree_nests_to_any_depth_and_sorts_siblings() {
        // Deliberately shuffled input, three levels deep, two roots.
        let profiles = vec![
            profile("macbook-pro", Some("macos")),
            profile("media-server", Some("linux")),
            profile("macos", None),
            profile("ubuntu-server", Some("linux")),
            profile("macbook-air-m1", Some("macbook-pro")),
            profile("linux", None),
        ];
        let rows = profile_tree(&profiles);
        assert_eq!(
            drawn(&profiles, &rows),
            vec![
                "linux",
                "├─ media-server",
                "└─ ubuntu-server",
                "macos",
                "└─ macbook-pro",
                "   └─ macbook-air-m1",
            ]
        );
        assert_eq!(
            rows.iter().map(|r| r.depth).collect::<Vec<_>>(),
            [0, 1, 1, 0, 1, 2]
        );
    }

    #[test]
    fn tree_draws_guides_for_ancestors_that_have_later_siblings() {
        let profiles = vec![
            profile("a", None),
            profile("a1", Some("a")),
            profile("a1x", Some("a1")),
            profile("a2", Some("a")),
        ];
        let rows = profile_tree(&profiles);
        assert_eq!(
            drawn(&profiles, &rows),
            vec!["a", "├─ a1", "│  └─ a1x", "└─ a2"]
        );
    }

    #[test]
    fn tree_ascii_glyphs() {
        let profiles = vec![profile("a", None), profile("b", Some("a"))];
        let rows = profile_tree(&profiles);
        assert_eq!(rows[1].prefix(TreeGlyphs::ASCII), "`- ");
    }

    #[test]
    fn tree_lists_every_profile_once_even_with_cycles_and_missing_parents() {
        let profiles = vec![
            profile("x", Some("y")),
            profile("y", Some("x")),
            profile("orphan", Some("does-not-exist")),
            profile("self", Some("self")),
        ];
        let rows = profile_tree(&profiles);
        let mut got = names(&profiles, &rows);
        got.sort();
        assert_eq!(got, vec!["orphan", "self", "x", "y"]);
    }

    #[test]
    fn ancestor_chain_is_root_first_and_terminates_on_cycles() {
        let profiles = vec![
            profile("macos", None),
            profile("macbook-pro", Some("macos")),
            profile("air", Some("macbook-pro")),
        ];
        assert_eq!(
            ancestor_chain("air", &profiles),
            vec!["macos", "macbook-pro"]
        );
        assert_eq!(ancestor_chain("macos", &profiles), Vec::<String>::new());

        let cyc = vec![profile("p", Some("q")), profile("q", Some("p"))];
        assert_eq!(ancestor_chain("p", &cyc), vec!["q"]);
    }

    #[test]
    fn layered_files_orders_common_ancestors_then_own() {
        let profiles = vec![
            profile("macos", None),
            profile("macbook-pro", Some("macos")),
            profile("air", Some("macbook-pro")),
        ];
        // `resolved` is sorted by path, as ProfileManifest::resolve_files returns it.
        let files = vec![
            resolved(".a", "common"),
            resolved(".b", "macos"),
            resolved(".c", "air"),
            resolved(".d", "macbook-pro"),
            resolved(".e", "common"),
        ];
        let layers = layered_files("air", &profiles, &files);
        let summary: Vec<(String, bool, Vec<String>)> = layers
            .into_iter()
            .map(|l| (l.name, l.is_own, l.files))
            .collect();
        assert_eq!(
            summary,
            vec![
                (
                    "common".to_string(),
                    false,
                    vec![".a".to_string(), ".e".to_string()]
                ),
                ("macos".to_string(), false, vec![".b".to_string()]),
                ("macbook-pro".to_string(), false, vec![".d".to_string()]),
                ("air".to_string(), true, vec![".c".to_string()]),
            ]
        );
    }

    #[test]
    fn layered_files_skips_empty_layers() {
        let profiles = vec![profile("macos", None), profile("air", Some("macos"))];
        let layers = layered_files("air", &profiles, &[resolved(".z", "common")]);
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].name, "common");
    }
}
