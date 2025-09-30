#![allow(dead_code)]
use assert_cmd::Command;
use std::path::PathBuf;
use tempfile::TempDir;

pub struct TestEnv {
    _dir: TempDir,
    #[allow(dead_code)]
    pub db: PathBuf,
    pub cfg: PathBuf,
}

impl TestEnv {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = dir.path().join("config");
        std::fs::create_dir_all(&cfg).expect("cfg dir");
        let db = dir.path().join("ditox.db");
        Self { _dir: dir, db, cfg }
    }

    pub fn bin(&self) -> Command {
        let mut cmd = Command::cargo_bin("ditox-cli").unwrap();
        cmd.env("XDG_CONFIG_HOME", &self.cfg);
        // Force local sqlite to avoid picking up any Turso/libsql config from the environment
        cmd.arg("--store").arg("sqlite");
        cmd
    }

    pub fn write_tiny_png(&self) -> PathBuf {
        #[allow(dead_code)]
        use base64::{engine::general_purpose, Engine as _};
        let b64 = include_str!("fixtures/tiny.png.b64");
        let bytes = general_purpose::STANDARD.decode(b64.trim()).unwrap();
        let path = self.cfg.join("tiny.png");
        std::fs::write(&path, &bytes).unwrap();
        path
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new()
    }
}
