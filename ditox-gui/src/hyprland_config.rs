//! `--install-hyprland-config` / `--uninstall-hyprland-config`
//! helper (Phase 4 sub-task 4.11).
//!
//! Writes a self-contained Hyprland config snippet to
//! `~/.config/hypr/conf.d/ditox.conf` so the user can `source = …`
//! it from `hyprland.conf` and immediately get:
//!
//! - `exec-once` of `ditox-gui --hide` (daemon starts at session
//!   start, hidden until first summon).
//! - `bind = CTRL, grave, exec, ditox-gui --toggle` (Ctrl+~ summon).
//! - Window rules to force `class:ditox-gui` to float / pin /
//!   not animate (matching the launcher's intent).
//! - A second `source =` line for `ditox-binds.conf`, the
//!   per-clip hotkey file Phase 5 will populate. Created empty
//!   here so the user's `hyprland.conf` doesn't error on a
//!   missing file.
//!
//! Per `H5` we **never** auto-modify `hyprland.conf` itself —
//! only the conf.d files we own. After `--install` we print the
//! one-line `source = …` the user has to add manually.
//!
//! The snippet is wrapped between `# >>> ditox-managed >>>` and
//! `# <<< end ditox-managed <<<` markers so re-running
//! `--install` overwrites only the snippet (idempotent) and
//! `--uninstall` can remove only that block. We're conservative
//! about not touching the rest of `~/.config/hypr/`.

use ditox_core::error::{DitoxError, Result};
use std::path::{Path, PathBuf};

const SNIPPET_BEGIN: &str = "# >>> ditox-managed (do not edit between these markers) >>>";
const SNIPPET_END: &str = "# <<< end ditox-managed <<<";

const SNIPPET_BODY: &str = "exec-once = ditox-gui --hide
bind = CTRL, grave, exec, ditox-gui --toggle
windowrulev2 = float, class:^(ditox-gui)$
windowrulev2 = pin, class:^(ditox-gui)$
windowrulev2 = noborder, class:^(ditox-gui)$
windowrulev2 = noshadow, class:^(ditox-gui)$
windowrulev2 = noanim, class:^(ditox-gui)$
source = ~/.config/hypr/conf.d/ditox-binds.conf
";

/// Write the snippet (idempotent) and the empty
/// `ditox-binds.conf` placeholder. Returns the path of the main
/// snippet so the caller can show it in the user-facing message.
pub fn install() -> Result<PathBuf> {
    let conf_d = conf_d_dir()?;
    std::fs::create_dir_all(&conf_d)
        .map_err(|e| DitoxError::Other(format!("could not create {}: {e}", conf_d.display())))?;

    let main = conf_d.join("ditox.conf");
    let binds = conf_d.join("ditox-binds.conf");

    write_managed_block(&main)?;
    if !binds.exists() {
        std::fs::write(
            &binds,
            "# Per-clip hotkeys go here (managed by ditox in Phase 5).\n",
        )
        .map_err(|e| DitoxError::Other(format!("could not create {}: {e}", binds.display())))?;
    }

    Ok(main)
}

pub fn write_clip_binds(entries: &[ditox_core::Entry]) -> Result<PathBuf> {
    let conf_d = conf_d_dir()?;
    std::fs::create_dir_all(&conf_d)
        .map_err(|e| DitoxError::Other(format!("could not create {}: {e}", conf_d.display())))?;
    let path = conf_d.join("ditox-binds.conf");
    let tmp = path.with_extension("conf.tmp");

    let mut out = String::from("# >>> ditox-managed per-clip hotkeys >>>\n");
    for entry in entries.iter().take(50) {
        let Some(hotkey) = entry.global_hotkey.as_deref() else {
            continue;
        };
        if let Some((mods, key)) = hypr_bind_parts(hotkey) {
            out.push_str(&format!(
                "bind = {mods}, {key}, exec, ditox-gui paste-clip {}\n",
                entry.id
            ));
        }
    }
    out.push_str("# <<< end ditox-managed per-clip hotkeys <<<\n");
    std::fs::write(&tmp, out)
        .map_err(|e| DitoxError::Other(format!("could not write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, &path)
        .map_err(|e| DitoxError::Other(format!("could not install {}: {e}", path.display())))?;
    let _ = std::process::Command::new("hyprctl").arg("reload").spawn();
    Ok(path)
}

fn hypr_bind_parts(hotkey: &str) -> Option<(String, String)> {
    let mut mods = Vec::new();
    let mut key = None;
    for part in hotkey.split('+') {
        match part.trim().to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods.push("CTRL"),
            "alt" => mods.push("ALT"),
            "shift" => mods.push("SHIFT"),
            "super" | "meta" | "win" => mods.push("SUPER"),
            other if !other.is_empty() => key = Some(other.to_ascii_uppercase()),
            _ => {}
        }
    }
    Some((mods.join("_"), key?))
}

/// Remove the managed snippet from `ditox.conf`, deleting the
/// file entirely if nothing else remains. Leaves
/// `ditox-binds.conf` alone (the user may have customised it).
/// Returns `true` iff something was actually removed.
pub fn uninstall() -> Result<bool> {
    let conf_d = conf_d_dir()?;
    let main = conf_d.join("ditox.conf");

    if !main.exists() {
        return Ok(false);
    }

    let content = std::fs::read_to_string(&main)
        .map_err(|e| DitoxError::Other(format!("could not read {}: {e}", main.display())))?;
    let stripped = strip_managed_block(&content);

    if stripped.trim().is_empty() {
        // File is now empty — remove it entirely.
        std::fs::remove_file(&main)
            .map_err(|e| DitoxError::Other(format!("could not remove {}: {e}", main.display())))?;
    } else {
        std::fs::write(&main, stripped)
            .map_err(|e| DitoxError::Other(format!("could not rewrite {}: {e}", main.display())))?;
    }

    Ok(content.contains(SNIPPET_BEGIN))
}

/// Resolve `~/.config/hypr/conf.d`, honouring `XDG_CONFIG_HOME`.
/// Errors when neither `XDG_CONFIG_HOME` nor `HOME` is set.
fn conf_d_dir() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .ok_or_else(|| {
            DitoxError::Other(
                "neither XDG_CONFIG_HOME nor HOME is set; cannot locate Hyprland config directory"
                    .into(),
            )
        })?;
    Ok(base.join("hypr").join("conf.d"))
}

/// Read the existing file (if any), strip the previous managed
/// block, append a fresh one, and write back. Atomic via tmp +
/// rename so a partial write doesn't leave the user's config in
/// a broken state.
fn write_managed_block(path: &Path) -> Result<()> {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let stripped = strip_managed_block(&existing);

    let mut out = String::with_capacity(stripped.len() + SNIPPET_BODY.len() + 200);
    out.push_str(stripped.trim_end());
    if !out.is_empty() {
        out.push('\n');
        out.push('\n');
    }
    out.push_str(SNIPPET_BEGIN);
    out.push('\n');
    out.push_str(SNIPPET_BODY);
    out.push_str(SNIPPET_END);
    out.push('\n');

    let tmp = path.with_extension("conf.tmp");
    std::fs::write(&tmp, out)
        .map_err(|e| DitoxError::Other(format!("write {}: {e}", tmp.display())))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| DitoxError::Other(format!("rename {}: {e}", tmp.display())))?;
    Ok(())
}

/// Return `content` with the `# >>> ditox-managed >>> … # <<< end
/// ditox-managed <<<` block removed (including the surrounding
/// blank-line padding). If the markers aren't both present the
/// content is returned unchanged.
fn strip_managed_block(content: &str) -> String {
    let begin_idx = content.find(SNIPPET_BEGIN);
    let end_idx = content.find(SNIPPET_END);
    let (Some(begin), Some(end)) = (begin_idx, end_idx) else {
        return content.to_string();
    };
    if begin >= end {
        return content.to_string();
    }
    let block_end = end + SNIPPET_END.len();
    // Consume the trailing newline if present so the reassembled
    // file doesn't have a stray blank line where the block was.
    let mut tail_start = block_end;
    if content.as_bytes().get(tail_start) == Some(&b'\n') {
        tail_start += 1;
    }
    let mut out = String::with_capacity(content.len());
    out.push_str(content[..begin].trim_end());
    out.push_str(&content[tail_start..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_returns_unchanged_when_no_markers() {
        let input = "exec-once = some-other-app\n";
        assert_eq!(strip_managed_block(input), input);
    }

    #[test]
    fn strip_removes_full_managed_block() {
        let input = format!(
            "exec-once = some-other-app\n\n{}\n{}\n{}\n",
            SNIPPET_BEGIN, "exec-once = ditox-gui --hide", SNIPPET_END
        );
        let out = strip_managed_block(&input);
        assert_eq!(out, "exec-once = some-other-app");
    }

    #[test]
    fn strip_handles_block_at_start() {
        let input = format!("{}\nfoo\n{}\nrest\n", SNIPPET_BEGIN, SNIPPET_END);
        let out = strip_managed_block(&input);
        // The `rest\n` survives.
        assert!(out.contains("rest"));
        assert!(!out.contains(SNIPPET_BEGIN));
    }

    #[test]
    fn strip_swallows_trailing_newline_after_end_marker() {
        let input = format!("a\n{}\nbody\n{}\nb\n", SNIPPET_BEGIN, SNIPPET_END);
        let out = strip_managed_block(&input);
        // No double-newline between `a` and `b`.
        assert!(!out.contains("\n\nb"));
    }

    #[test]
    fn strip_only_begin_marker_returns_unchanged() {
        let input = format!("{}\nincomplete\n", SNIPPET_BEGIN);
        assert_eq!(strip_managed_block(&input), input);
    }

    #[test]
    fn strip_only_end_marker_returns_unchanged() {
        let input = format!("incomplete\n{}\n", SNIPPET_END);
        assert_eq!(strip_managed_block(&input), input);
    }

    #[test]
    fn write_managed_block_round_trip_to_tempdir() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("ditox.conf");
        write_managed_block(&path).expect("write");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains(SNIPPET_BEGIN));
        assert!(content.contains(SNIPPET_END));
        assert!(content.contains("ditox-gui --toggle"));
        assert!(content.contains("ditox-gui --hide"));
        // Re-running is idempotent — content unchanged on second
        // call.
        write_managed_block(&path).expect("write again");
        let content2 = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, content2);
    }

    #[test]
    fn write_preserves_user_content_outside_markers() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("ditox.conf");
        std::fs::write(&path, "exec-once = my-other-thing\n").unwrap();
        write_managed_block(&path).expect("write");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("my-other-thing"));
        assert!(content.contains(SNIPPET_BEGIN));
    }
}
