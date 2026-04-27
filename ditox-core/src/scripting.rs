use crate::capture::RawClip;
use crate::error::{DitoxError, Result};
use rhai::{Dynamic, Engine, Map, Scope, AST};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Continue,
    Drop,
}

#[derive(Clone)]
pub struct Script {
    pub id: String,
    ast: AST,
}

pub struct ScriptEngine {
    engine: Engine,
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        engine.set_max_operations(1_000_000);
        engine.set_max_string_size(64_000);
        engine.set_max_call_levels(64);
        engine.set_max_array_size(1024);
        engine.set_max_map_size(1024);
        engine.disable_symbol("eval");
        engine.disable_symbol("import");
        Self { engine }
    }

    pub fn compile(&self, id: impl Into<String>, source: &str) -> Result<Script> {
        let ast = self
            .engine
            .compile(source)
            .map_err(|e| DitoxError::Config(format!("script compile failed: {e}")))?;
        Ok(Script { id: id.into(), ast })
    }

    pub fn load_dir(&self, dir: &Path) -> Result<Vec<Script>> {
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut scripts = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rhai") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let source = std::fs::read_to_string(&path)?;
            scripts.push(self.compile(id.to_string(), &source)?);
        }
        scripts.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(scripts)
    }

    pub fn run_capture_script(&self, script: &Script, clip: &mut RawClip) -> Result<Decision> {
        let mut scope = Scope::new();
        scope.push("clip", clip_to_map(clip));
        let value = self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &script.ast)
            .map_err(|e| DitoxError::Other(format!("script '{}' failed: {e}", script.id)))?;
        let decision = parse_decision(value);
        if let Some(map) = scope.get_value::<Map>("clip") {
            apply_clip_map(clip, map);
        }
        Ok(decision)
    }
}

pub fn scripts_root() -> Result<PathBuf> {
    Ok(crate::config::Config::get_config_path()?
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("scripts"))
}

fn clip_to_map(clip: &RawClip) -> Map {
    let mut map = Map::new();
    map.insert(
        "text".into(),
        clip.text_content().unwrap_or_default().into(),
    );
    map.insert(
        "source_app".into(),
        clip.source_app.clone().unwrap_or_default().into(),
    );
    map
}

fn apply_clip_map(clip: &mut RawClip, map: Map) {
    if let Some(value) = map.get("text") {
        if let Some(text) = value.clone().try_cast::<String>() {
            clip.set_text(text);
        }
    }
}

fn parse_decision(value: Dynamic) -> Decision {
    value
        .try_cast::<String>()
        .map(|s| {
            if s.eq_ignore_ascii_case("drop") {
                Decision::Drop
            } else {
                Decision::Continue
            }
        })
        .unwrap_or(Decision::Continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_can_mutate_clip_text() {
        let engine = ScriptEngine::new();
        let script = engine
            .compile("mutate", r#"clip["text"] = "changed";"#)
            .unwrap();
        let mut clip = RawClip::text("original".into());
        let decision = engine.run_capture_script(&script, &mut clip).unwrap();
        assert_eq!(decision, Decision::Continue);
        assert_eq!(clip.text_content().as_deref(), Some("changed"));
    }

    #[test]
    fn script_can_drop_clip() {
        let engine = ScriptEngine::new();
        let script = engine.compile("drop", r#""drop""#).unwrap();
        let mut clip = RawClip::text("x".into());
        assert_eq!(
            engine.run_capture_script(&script, &mut clip).unwrap(),
            Decision::Drop
        );
    }

    #[test]
    fn infinite_loop_is_stopped_by_operation_limit() {
        let engine = ScriptEngine::new();
        let script = engine.compile("loop", "while true {} ").unwrap();
        let mut clip = RawClip::text("x".into());
        assert!(engine.run_capture_script(&script, &mut clip).is_err());
    }
}
