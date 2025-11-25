//! Common test utilities and fixtures for ditox tests.

#![allow(dead_code)] // Test utilities may not all be used in every test file

use std::path::PathBuf;
use tempfile::TempDir;

/// Test fixture that creates an isolated environment for testing.
/// Includes a temporary directory with database and config.
pub struct TestFixture {
    pub temp_dir: TempDir,
    pub db_path: PathBuf,
    pub config_path: PathBuf,
    pub images_dir: PathBuf,
}

impl TestFixture {
    /// Create a new test fixture with temporary directories.
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let base = temp_dir.path();

        let db_path = base.join("data").join("ditox.db");
        let config_path = base.join("config").join("config.toml");
        let images_dir = base.join("data").join("images");

        // Create directories
        std::fs::create_dir_all(db_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&images_dir).unwrap();

        Self {
            temp_dir,
            db_path,
            config_path,
            images_dir,
        }
    }

    /// Get environment variables to override ditox paths for testing.
    pub fn env_vars(&self) -> Vec<(&str, String)> {
        vec![
            ("XDG_DATA_HOME", self.temp_dir.path().join("data").to_string_lossy().to_string()),
            ("XDG_CONFIG_HOME", self.temp_dir.path().join("config").to_string_lossy().to_string()),
        ]
    }

    /// Create a minimal config file.
    pub fn create_config(&self, content: &str) {
        std::fs::create_dir_all(self.config_path.parent().unwrap()).unwrap();
        let ditox_config = self.temp_dir.path().join("config").join("ditox").join("config.toml");
        std::fs::create_dir_all(ditox_config.parent().unwrap()).unwrap();
        std::fs::write(&ditox_config, content).unwrap();
    }

    /// Create default config.
    pub fn create_default_config(&self) {
        self.create_config(
            r#"
[general]
max_entries = 100
poll_interval_ms = 500

[ui]
show_preview = true
date_format = "relative"
"#,
        );
    }
}

impl Default for TestFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Sample test data for entries.
pub mod sample_data {
    pub const TEXT_SAMPLES: &[&str] = &[
        "Hello, World!",
        "fn main() { println!(\"Hello\"); }",
        "https://github.com/oxfell/ditox",
        "SELECT * FROM users WHERE id = 1;",
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
        "{ \"key\": \"value\", \"number\": 42 }",
        "export PATH=$HOME/.cargo/bin:$PATH",
        "The quick brown fox jumps over the lazy dog.",
        "#!/bin/bash\necho 'test'",
        "multiline\ntext\nwith\nnewlines",
    ];

    pub const LONG_TEXT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. \
        Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \
        Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris \
        nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in \
        reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur.";

    pub const UNICODE_TEXT: &str = "Hello 世界 🌍 مرحبا Привет";

    pub const WHITESPACE_TEXT: &str = "  \t  text with   whitespace  \n\t  ";

    pub const EMPTY_TEXT: &str = "";
}
