//! Preset keymaps: Standard, Vim, Emacs
//!
//! Each preset provides a complete set of key bindings for all actions.

use super::{Action, KeyBinding};
use serde::{Deserialize, Serialize};

/// Available keymap presets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum KeymapPreset {
    /// Standard keyboard navigation (arrows, Enter, Esc)
    #[default]
    Standard,
    /// Vim-style navigation (hjkl, etc.)
    Vim,
    /// Emacs-style navigation (Ctrl+N/P, etc.)
    Emacs,
}

impl KeymapPreset {
    /// Get all key bindings for this preset
    #[must_use]
    pub fn bindings(&self) -> Vec<KeyBinding> {
        match self {
            KeymapPreset::Standard => standard_bindings(),
            KeymapPreset::Vim => vim_bindings(),
            KeymapPreset::Emacs => emacs_bindings(),
        }
    }

    /// Get human-readable name
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            KeymapPreset::Standard => "Standard",
            KeymapPreset::Vim => "Vim",
            KeymapPreset::Emacs => "Emacs",
        }
    }
}

/// Standard keyboard bindings (arrows, Enter, Esc)
fn standard_bindings() -> Vec<KeyBinding> {
    vec![
        // Navigation
        KeyBinding::new("up", Action::MoveUp),
        KeyBinding::new("down", Action::MoveDown),
        KeyBinding::new("left", Action::MoveLeft),
        KeyBinding::new("right", Action::MoveRight),
        KeyBinding::new("pageup", Action::PageUp),
        KeyBinding::new("pagedown", Action::PageDown),
        KeyBinding::new("home", Action::GoToTop),
        KeyBinding::new("end", Action::GoToEnd),
        // Selection
        KeyBinding::new("enter", Action::Confirm),
        KeyBinding::new("esc", Action::Cancel),
        KeyBinding::new("space", Action::ToggleSelect),
        KeyBinding::new("ctrl+a", Action::SelectAll),
        // Alt+A, not Ctrl+Shift+A (plain terminals deliver that as Ctrl+A = SelectAll) and not
        // Shift+A (the import popup's filter box is a text input; Shift+A must stay typeable).
        KeyBinding::new("alt+a", Action::DeselectAll),
        // Global
        KeyBinding::new("q", Action::Quit),
        KeyBinding::new("ctrl+c", Action::Quit),
        KeyBinding::new("?", Action::Help),
        // Actions
        KeyBinding::new("d", Action::Delete),
        KeyBinding::new("e", Action::Edit),
        KeyBinding::new("c", Action::Create),
        KeyBinding::new("shift+c", Action::CreateCommon),
        KeyBinding::new("/", Action::Search),
        KeyBinding::new("r", Action::Refresh),
        KeyBinding::new("s", Action::CheckStatus),
        KeyBinding::new("shift+s", Action::Sync),
        KeyBinding::new("i", Action::Install),
        KeyBinding::new("shift+i", Action::Import),
        KeyBinding::new("ctrl+s", Action::Save),
        KeyBinding::new("b", Action::ToggleBackup),
        KeyBinding::new("m", Action::Move),
        // Text editing
        KeyBinding::new("backspace", Action::Backspace),
        KeyBinding::new("delete", Action::DeleteChar),
        // Tab navigation
        KeyBinding::new("tab", Action::NextTab),
        KeyBinding::new("shift+backtab", Action::PrevTab),
        // Scroll (with shift modifier for preview panes)
        KeyBinding::new("shift+up", Action::ScrollUp),
        KeyBinding::new("shift+down", Action::ScrollDown),
        // Yes/No prompts
        KeyBinding::new("y", Action::Yes),
        KeyBinding::new("n", Action::No),
    ]
}

/// Vim-style keyboard bindings (hjkl navigation)
fn vim_bindings() -> Vec<KeyBinding> {
    vec![
        // Navigation - vim style + arrows
        KeyBinding::new("k", Action::MoveUp),
        KeyBinding::new("up", Action::MoveUp),
        KeyBinding::new("j", Action::MoveDown),
        KeyBinding::new("down", Action::MoveDown),
        KeyBinding::new("h", Action::MoveLeft),
        KeyBinding::new("left", Action::MoveLeft),
        KeyBinding::new("l", Action::MoveRight),
        KeyBinding::new("right", Action::MoveRight),
        KeyBinding::new("ctrl+u", Action::PageUp),
        KeyBinding::new("pageup", Action::PageUp),
        KeyBinding::new("ctrl+d", Action::PageDown),
        KeyBinding::new("pagedown", Action::PageDown),
        KeyBinding::new("g", Action::GoToTop), // gg in real vim, but single g works
        KeyBinding::new("home", Action::GoToTop),
        KeyBinding::new("shift+g", Action::GoToEnd),
        KeyBinding::new("end", Action::GoToEnd),
        // Selection
        KeyBinding::new("enter", Action::Confirm),
        KeyBinding::new("esc", Action::Cancel),
        KeyBinding::new("space", Action::ToggleSelect),
        KeyBinding::new("ctrl+a", Action::SelectAll),
        // Alt+A, not Ctrl+Shift+A (plain terminals deliver that as Ctrl+A = SelectAll) and not
        // Shift+A (the import popup's filter box is a text input; Shift+A must stay typeable).
        KeyBinding::new("alt+a", Action::DeselectAll),
        // Global - vim uses q to quit
        KeyBinding::new("q", Action::Quit),
        KeyBinding::new("ctrl+c", Action::Quit),
        KeyBinding::new("?", Action::Help),
        // Actions
        KeyBinding::new("d", Action::Delete),
        KeyBinding::new("e", Action::Edit),
        KeyBinding::new("o", Action::Create), // 'o' for open/new in vim style
        KeyBinding::new("shift+o", Action::CreateCommon),
        KeyBinding::new("/", Action::Search),
        KeyBinding::new("r", Action::Refresh),
        KeyBinding::new("s", Action::CheckStatus),
        KeyBinding::new("shift+s", Action::Sync),
        KeyBinding::new("i", Action::Install),
        KeyBinding::new("shift+i", Action::Import),
        KeyBinding::new("ctrl+s", Action::Save),
        KeyBinding::new("b", Action::ToggleBackup),
        KeyBinding::new("m", Action::Move),
        // Text editing
        KeyBinding::new("backspace", Action::Backspace),
        KeyBinding::new("x", Action::DeleteChar), // vim style delete char
        KeyBinding::new("delete", Action::DeleteChar),
        // Tab navigation
        KeyBinding::new("tab", Action::NextTab),
        KeyBinding::new("shift+backtab", Action::PrevTab),
        // Scroll (vim style)
        KeyBinding::new("ctrl+y", Action::ScrollUp),
        KeyBinding::new("ctrl+e", Action::ScrollDown),
        // Yes/No prompts
        KeyBinding::new("y", Action::Yes),
        KeyBinding::new("n", Action::No),
    ]
}

/// Emacs-style keyboard bindings (Ctrl+N/P navigation)
fn emacs_bindings() -> Vec<KeyBinding> {
    vec![
        // Navigation - emacs style + arrows
        KeyBinding::new("ctrl+p", Action::MoveUp),
        KeyBinding::new("up", Action::MoveUp),
        KeyBinding::new("ctrl+n", Action::MoveDown),
        KeyBinding::new("down", Action::MoveDown),
        KeyBinding::new("ctrl+b", Action::MoveLeft),
        KeyBinding::new("left", Action::MoveLeft),
        KeyBinding::new("ctrl+f", Action::MoveRight),
        KeyBinding::new("right", Action::MoveRight),
        KeyBinding::new("alt+v", Action::PageUp),
        KeyBinding::new("pageup", Action::PageUp),
        KeyBinding::new("ctrl+v", Action::PageDown),
        KeyBinding::new("pagedown", Action::PageDown),
        KeyBinding::new("alt+shift+,", Action::GoToTop), // M-< in emacs
        KeyBinding::new("home", Action::GoToTop),
        KeyBinding::new("alt+shift+.", Action::GoToEnd), // M-> in emacs
        KeyBinding::new("end", Action::GoToEnd),
        // Selection
        KeyBinding::new("enter", Action::Confirm),
        KeyBinding::new("ctrl+g", Action::Cancel), // C-g is cancel in emacs
        KeyBinding::new("esc", Action::Cancel),
        KeyBinding::new("space", Action::ToggleSelect),
        KeyBinding::new("ctrl+a", Action::SelectAll),
        // Alt+A, not Ctrl+Shift+A (plain terminals deliver that as Ctrl+A = SelectAll) and not
        // Shift+A (the import popup's filter box is a text input; Shift+A must stay typeable).
        KeyBinding::new("alt+a", Action::DeselectAll),
        // Global
        KeyBinding::new("q", Action::Quit),
        KeyBinding::new("ctrl+c", Action::Quit),
        KeyBinding::new("ctrl+h", Action::Help),
        KeyBinding::new("?", Action::Help),
        // Actions
        KeyBinding::new("d", Action::Delete), // Use 'd' since Ctrl+D is DeleteChar in Emacs
        KeyBinding::new("ctrl+e", Action::Edit),
        KeyBinding::new("ctrl+o", Action::Create),
        // Plain-terminal safe: Ctrl+Shift+<letter> is indistinguishable from Ctrl+<letter>
        // without the kitty keyboard protocol, so it would be shadowed by ctrl+o (Create).
        KeyBinding::new("shift+o", Action::CreateCommon),
        KeyBinding::new("/", Action::Search), // Use / for search (Ctrl+S is used for Save)
        KeyBinding::new("ctrl+r", Action::Refresh),
        // Chords (C-x s, C-x C-c) are not supported: the parser rejects them and they never
        // match. Sync uses Shift+S as in the other presets; Quit is covered by q / Ctrl+C.
        KeyBinding::new("shift+s", Action::Sync),
        KeyBinding::new("s", Action::CheckStatus),
        KeyBinding::new("i", Action::Install),
        KeyBinding::new("shift+i", Action::Import),
        KeyBinding::new("ctrl+s", Action::Save),
        KeyBinding::new("b", Action::ToggleBackup), // Use 'b' since Ctrl+B is MoveLeft in Emacs
        KeyBinding::new("m", Action::Move),
        // Text editing
        KeyBinding::new("backspace", Action::Backspace),
        KeyBinding::new("ctrl+d", Action::DeleteChar), // Forward delete (Emacs standard)
        KeyBinding::new("delete", Action::DeleteChar),
        // Tab navigation
        KeyBinding::new("tab", Action::NextTab),
        KeyBinding::new("shift+backtab", Action::PrevTab),
        // Scroll
        KeyBinding::new("alt+p", Action::ScrollUp),
        KeyBinding::new("alt+n", Action::ScrollDown),
        // Yes/No prompts
        KeyBinding::new("y", Action::Yes),
        KeyBinding::new("n", Action::No),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_names() {
        assert_eq!(KeymapPreset::Standard.name(), "Standard");
        assert_eq!(KeymapPreset::Vim.name(), "Vim");
        assert_eq!(KeymapPreset::Emacs.name(), "Emacs");
    }

    #[test]
    fn test_standard_bindings_complete() {
        let bindings = KeymapPreset::Standard.bindings();
        // Should have navigation bindings
        assert!(bindings.iter().any(|b| b.action == Action::MoveUp));
        assert!(bindings.iter().any(|b| b.action == Action::MoveDown));
        assert!(bindings.iter().any(|b| b.action == Action::Confirm));
        assert!(bindings.iter().any(|b| b.action == Action::Cancel));
        assert!(bindings.iter().any(|b| b.action == Action::Quit));
        assert!(bindings.iter().any(|b| b.action == Action::Help));
    }

    #[test]
    fn test_vim_has_hjkl() {
        let bindings = KeymapPreset::Vim.bindings();
        assert!(bindings
            .iter()
            .any(|b| b.key == "j" && b.action == Action::MoveDown));
        assert!(bindings
            .iter()
            .any(|b| b.key == "k" && b.action == Action::MoveUp));
        assert!(bindings
            .iter()
            .any(|b| b.key == "h" && b.action == Action::MoveLeft));
        assert!(bindings
            .iter()
            .any(|b| b.key == "l" && b.action == Action::MoveRight));
    }

    #[test]
    fn test_emacs_has_ctrl_np() {
        let bindings = KeymapPreset::Emacs.bindings();
        assert!(bindings
            .iter()
            .any(|b| b.key == "ctrl+n" && b.action == Action::MoveDown));
        assert!(bindings
            .iter()
            .any(|b| b.key == "ctrl+p" && b.action == Action::MoveUp));
    }

    /// Actions handled by the Manage Packages list view (no popup open).
    const MANAGE_PACKAGES_ACTIONS: &[Action] = &[
        Action::MoveUp,
        Action::MoveDown,
        Action::Refresh,
        Action::CheckStatus,
        Action::Install,
        Action::Create,
        Action::CreateCommon,
        Action::Edit,
        Action::Delete,
        Action::Move,
        Action::Import,
        Action::Cancel,
        Action::Quit,
    ];

    /// Key as a plain terminal delivers it: without the kitty keyboard protocol,
    /// Ctrl+Shift+<letter> arrives as the same byte as Ctrl+<letter>.
    fn plain_terminal_key(
        key: &str,
    ) -> (crossterm::event::KeyCode, crossterm::event::KeyModifiers) {
        use crossterm::event::{KeyCode, KeyModifiers};
        let parsed = super::super::binding::parse_key_string(key)
            .unwrap_or_else(|e| panic!("unparsable binding {key:?}: {e}"));
        let mut mods = parsed.modifiers;
        if mods.contains(KeyModifiers::CONTROL) && matches!(parsed.code, KeyCode::Char(_)) {
            mods.remove(KeyModifiers::SHIFT);
        }
        (parsed.code, mods)
    }

    const ALL_PRESETS: [KeymapPreset; 3] = [
        KeymapPreset::Standard,
        KeymapPreset::Vim,
        KeymapPreset::Emacs,
    ];

    /// Collect bindings whose plain-terminal key is shared by two different actions.
    /// Returns `(preset, key_a, action_a, key_b, action_b)` tuples.
    fn find_collisions(
        preset: KeymapPreset,
        only: Option<&[Action]>,
    ) -> Vec<(String, Action, String, Action)> {
        use std::collections::HashMap;
        let mut seen: HashMap<_, (Action, String)> = HashMap::new();
        let mut collisions = Vec::new();
        for b in preset
            .bindings()
            .into_iter()
            .filter(|b| only.is_none_or(|a| a.contains(&b.action)))
        {
            let key = plain_terminal_key(&b.key);
            if let Some((other, other_key)) = seen.insert(key, (b.action, b.key.clone())) {
                if other != b.action {
                    collisions.push((other_key, other, b.key.clone(), b.action));
                }
            }
        }
        collisions
    }

    /// An unparsable key string never matches (`KeyBinding::matches` swallows the parse
    /// error), so a typo or a chord like "ctrl+x s" would silently create a dead binding.
    #[test]
    fn test_all_preset_keys_parse() {
        for preset in ALL_PRESETS {
            for b in preset.bindings() {
                if let Err(e) = b.parse() {
                    panic!(
                        "{}: binding {:?} for {:?} does not parse: {e}",
                        preset.name(),
                        b.key,
                        b.action
                    );
                }
            }
        }
    }

    #[test]
    fn test_manage_packages_bindings_have_no_collisions() {
        for preset in ALL_PRESETS {
            let collisions = find_collisions(preset, Some(MANAGE_PACKAGES_ACTIONS));
            assert!(
                collisions.is_empty(),
                "{}: colliding bindings (key_a, action_a, key_b, action_b): {collisions:?}",
                preset.name()
            );
        }
    }

    /// Whole-preset flat check: no two actions share a key (after plain-terminal
    /// normalisation). Every preset is a single context-agnostic map consulted by
    /// `Keymap::get_action`, so the first binding wins and any shared key makes the
    /// later action unreachable. There are currently no intentional overlaps; if one is
    /// ever needed, whitelist it here with a comment instead of weakening the check.
    const INTENTIONAL_OVERLAPS: &[(&str, &str)] = &[
        // (preset name, key string) -- none at present.
    ];

    #[test]
    fn test_no_action_shares_a_key_within_a_preset() {
        for preset in ALL_PRESETS {
            let collisions: Vec<_> = find_collisions(preset, None)
                .into_iter()
                .filter(|(ka, _, kb, _)| {
                    !INTENTIONAL_OVERLAPS
                        .iter()
                        .any(|(p, k)| *p == preset.name() && (k == ka || k == kb))
                })
                .collect();
            assert!(
                collisions.is_empty(),
                "{}: colliding bindings (key_a, action_a, key_b, action_b): {collisions:?}",
                preset.name()
            );
        }
    }

    /// Every action must be reachable on a plain terminal: the binding, as a plain
    /// terminal delivers it, must resolve back to that same action.
    #[test]
    fn test_every_binding_resolves_to_its_action_on_plain_terminal() {
        use crate::keymap::Keymap;
        for preset in ALL_PRESETS {
            let keymap = Keymap {
                preset,
                overrides: Vec::new(),
            };
            for b in preset.bindings() {
                let (code, mods) = plain_terminal_key(&b.key);
                assert_eq!(
                    keymap.get_action(code, mods),
                    Some(b.action),
                    "{}: {:?} ({}) is shadowed or unreachable on a plain terminal",
                    preset.name(),
                    b.action,
                    b.key
                );
            }
        }
    }

    #[test]
    fn test_preset_serialization() {
        let preset = KeymapPreset::Vim;
        let json = serde_json::to_string(&preset).unwrap();
        assert_eq!(json, "\"vim\"");
    }

    #[test]
    fn test_preset_deserialization() {
        let preset: KeymapPreset = serde_json::from_str("\"emacs\"").unwrap();
        assert_eq!(preset, KeymapPreset::Emacs);
    }
}
