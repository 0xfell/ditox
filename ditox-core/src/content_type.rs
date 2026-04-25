//! Smart content type detection for clipboard entries
//!
//! Automatically detects the type of content in clipboard entries
//! for enhanced display and filtering.

#![allow(dead_code)]

use regex::Regex;
use std::sync::LazyLock;

/// Detected content type for an entry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// Plain text (default)
    Text,
    /// URL (http, https, ftp, etc.)
    Url,
    /// Email address
    Email,
    /// File path (Unix or Windows style)
    FilePath,
    /// JSON content
    Json,
    /// YAML content
    Yaml,
    /// Code snippet (detected by syntax patterns)
    Code,
    /// Shell command
    Shell,
    /// Hex color code
    Color,
    /// UUID
    Uuid,
    /// IP address (IPv4 or IPv6)
    IpAddress,
    /// Phone number
    Phone,
}

impl ContentType {
    /// Get a short label for display
    pub fn label(&self) -> &'static str {
        match self {
            ContentType::Text => "txt",
            ContentType::Url => "url",
            ContentType::Email => "mail",
            ContentType::FilePath => "path",
            ContentType::Json => "json",
            ContentType::Yaml => "yaml",
            ContentType::Code => "code",
            ContentType::Shell => "sh",
            ContentType::Color => "color",
            ContentType::Uuid => "uuid",
            ContentType::IpAddress => "ip",
            ContentType::Phone => "tel",
        }
    }

    /// Get a description for tooltips/help
    pub fn description(&self) -> &'static str {
        match self {
            ContentType::Text => "Plain text",
            ContentType::Url => "Web URL",
            ContentType::Email => "Email address",
            ContentType::FilePath => "File path",
            ContentType::Json => "JSON data",
            ContentType::Yaml => "YAML data",
            ContentType::Code => "Code snippet",
            ContentType::Shell => "Shell command",
            ContentType::Color => "Color code",
            ContentType::Uuid => "UUID",
            ContentType::IpAddress => "IP address",
            ContentType::Phone => "Phone number",
        }
    }
}

// Pre-compiled regex patterns for content detection
static URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(https?|ftp|file)://[^\s]+$").unwrap());

static EMAIL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap());

static UNIX_PATH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Match Unix-style absolute paths
    Regex::new(r"^(/[^/]+)+/?$").unwrap()
});

static WINDOWS_PATH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z]:[/\\]").unwrap());

static UUID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")
        .unwrap()
});

static HEX_COLOR_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$").unwrap());

static IPV4_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9]{1,3}\.){3}[0-9]{1,3}$").unwrap());

static IPV6_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$").unwrap());

static PHONE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Matches various phone formats: +1234567890, (123) 456-7890, 123-456-7890, etc.
    Regex::new(
        r"^[+]?[(]?[0-9]{1,4}[)]?[-\s.]?[(]?[0-9]{1,4}[)]?[-\s.]?[0-9]{1,4}[-\s.]?[0-9]{1,9}$",
    )
    .unwrap()
});

/// Detect the content type of a text string
pub fn detect(content: &str) -> ContentType {
    let trimmed = content.trim();

    // Empty or whitespace-only content
    if trimmed.is_empty() {
        return ContentType::Text;
    }

    // Single-line content detection (most specific patterns)
    if !trimmed.contains('\n') {
        // URL detection (must be complete URL)
        if URL_REGEX.is_match(trimmed) {
            return ContentType::Url;
        }

        // Email detection
        if EMAIL_REGEX.is_match(trimmed) {
            return ContentType::Email;
        }

        // UUID detection
        if UUID_REGEX.is_match(trimmed) {
            return ContentType::Uuid;
        }

        // Hex color detection
        if HEX_COLOR_REGEX.is_match(trimmed) {
            return ContentType::Color;
        }

        // IP address detection
        if IPV4_REGEX.is_match(trimmed) || IPV6_REGEX.is_match(trimmed) {
            return ContentType::IpAddress;
        }

        // Phone number detection
        if PHONE_REGEX.is_match(trimmed) && trimmed.len() >= 7 && trimmed.len() <= 20 {
            return ContentType::Phone;
        }

        // File path detection (only if it looks like a path)
        if trimmed.starts_with('/') && UNIX_PATH_REGEX.is_match(trimmed) {
            return ContentType::FilePath;
        }
        if WINDOWS_PATH_REGEX.is_match(trimmed) {
            return ContentType::FilePath;
        }
    }

    // Multi-line or complex content detection

    // JSON detection (try to parse as JSON)
    if ((trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']')))
        && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
    {
        return ContentType::Json;
    }

    // YAML detection (check for YAML-like structure)
    if is_yaml_like(trimmed) {
        return ContentType::Yaml;
    }

    // Shell command detection
    if is_shell_command(trimmed) {
        return ContentType::Shell;
    }

    // Code detection (heuristic-based)
    if is_code_like(trimmed) {
        return ContentType::Code;
    }

    ContentType::Text
}

/// Check if content looks like YAML
fn is_yaml_like(content: &str) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 2 {
        return false;
    }

    // Check for YAML-specific patterns
    let yaml_patterns = [
        "---", // YAML document start
        "...", // YAML document end
    ];

    if lines
        .first()
        .map(|l| yaml_patterns.contains(l))
        .unwrap_or(false)
    {
        return true;
    }

    // Check for key: value patterns with consistent indentation
    let key_value_count = lines
        .iter()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && trimmed.contains(':')
                && !trimmed.starts_with("http")
        })
        .count();

    // If more than half the lines are key-value pairs, likely YAML
    key_value_count > lines.len() / 2
}

/// Check if content looks like a shell command
fn is_shell_command(content: &str) -> bool {
    let trimmed = content.trim();
    let first_line = trimmed.lines().next().unwrap_or("");

    // Common shell command prefixes
    let shell_prefixes = [
        "sudo ",
        "cd ",
        "ls ",
        "cat ",
        "grep ",
        "find ",
        "sed ",
        "awk ",
        "echo ",
        "export ",
        "source ",
        "chmod ",
        "chown ",
        "mkdir ",
        "rm ",
        "cp ",
        "mv ",
        "touch ",
        "curl ",
        "wget ",
        "git ",
        "docker ",
        "npm ",
        "cargo ",
        "python ",
        "pip ",
        "node ",
        "make ",
        "cmake ",
        "apt ",
        "brew ",
        "yum ",
        "dnf ",
        "pacman ",
        "systemctl ",
        "journalctl ",
        "./",
        "sh ",
        "bash ",
        "zsh ",
    ];

    // Check for shebang
    if first_line.starts_with("#!") {
        return true;
    }

    // Check for common shell command patterns
    let lower = first_line.to_lowercase();
    for prefix in shell_prefixes {
        if lower.starts_with(prefix) {
            return true;
        }
    }

    // Check for pipe chains
    if trimmed.contains(" | ") {
        return true;
    }

    false
}

/// Check if content looks like code
fn is_code_like(content: &str) -> bool {
    let trimmed = content.trim();

    // Check for common code patterns
    let code_indicators = [
        // Function/method definitions
        "fn ",
        "def ",
        "func ",
        "function ",
        "async fn ",
        // Class/struct definitions
        "class ",
        "struct ",
        "interface ",
        "enum ",
        // Control flow
        "if (",
        "if(",
        "for (",
        "for(",
        "while (",
        "while(",
        "switch (",
        "switch(",
        "match {",
        "match(",
        // Variable declarations
        "let ",
        "const ",
        "var ",
        "mut ",
        // Imports
        "import ",
        "from ",
        "use ",
        "require(",
        "#include",
        "using namespace",
        // Return statements
        "return ",
        "return;",
        "return(",
        // Common syntax
        "=> {",
        "=> (",
        "() {",
        "() =>",
        // Comments
        "//",
        "/*",
        "*/",
        "# ",
    ];

    for indicator in code_indicators {
        if trimmed.contains(indicator) {
            return true;
        }
    }

    // Check for high density of special characters (code-like)
    let special_chars: usize = trimmed
        .chars()
        .filter(|c| matches!(c, '{' | '}' | '(' | ')' | '[' | ']' | ';' | '=' | '<' | '>'))
        .count();

    let char_count = trimmed.chars().count();
    if char_count > 20 && (special_chars as f64 / char_count as f64) > 0.1 {
        return true;
    }

    // Check for indentation patterns (common in code)
    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() >= 3 {
        let indented_lines = lines
            .iter()
            .filter(|l| l.starts_with("    ") || l.starts_with('\t'))
            .count();
        if indented_lines > lines.len() / 3 {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_url() {
        assert_eq!(detect("https://example.com"), ContentType::Url);
        assert_eq!(detect("http://localhost:8080/path"), ContentType::Url);
        assert_eq!(detect("ftp://files.example.com/file.txt"), ContentType::Url);
    }

    #[test]
    fn test_detect_email() {
        assert_eq!(detect("user@example.com"), ContentType::Email);
        assert_eq!(detect("test.user+tag@sub.domain.org"), ContentType::Email);
    }

    #[test]
    fn test_detect_uuid() {
        assert_eq!(
            detect("550e8400-e29b-41d4-a716-446655440000"),
            ContentType::Uuid
        );
        assert_eq!(
            detect("AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE"),
            ContentType::Uuid
        );
    }

    #[test]
    fn test_detect_color() {
        assert_eq!(detect("#fff"), ContentType::Color);
        assert_eq!(detect("#ff5500"), ContentType::Color);
        assert_eq!(detect("#ff550080"), ContentType::Color);
    }

    #[test]
    fn test_detect_ip() {
        assert_eq!(detect("192.168.1.1"), ContentType::IpAddress);
        assert_eq!(detect("127.0.0.1"), ContentType::IpAddress);
    }

    #[test]
    fn test_detect_path() {
        assert_eq!(detect("/home/user/documents"), ContentType::FilePath);
        assert_eq!(detect("/usr/local/bin/"), ContentType::FilePath);
    }

    #[test]
    fn test_detect_json() {
        assert_eq!(detect(r#"{"key": "value"}"#), ContentType::Json);
        assert_eq!(detect(r#"[1, 2, 3]"#), ContentType::Json);
        assert_eq!(detect("{\n  \"name\": \"test\"\n}"), ContentType::Json);
    }

    #[test]
    fn test_detect_shell() {
        assert_eq!(detect("sudo apt install vim"), ContentType::Shell);
        assert_eq!(detect("git commit -m 'test'"), ContentType::Shell);
        assert_eq!(detect("cat file.txt | grep pattern"), ContentType::Shell);
        assert_eq!(detect("#!/bin/bash\necho hello"), ContentType::Shell);
    }

    #[test]
    fn test_detect_code() {
        assert_eq!(
            detect("fn main() {\n    println!(\"Hello\");\n}"),
            ContentType::Code
        );
        assert_eq!(
            detect("def hello():\n    print('world')"),
            ContentType::Code
        );
        assert_eq!(
            detect("const x = () => {\n    return 42;\n}"),
            ContentType::Code
        );
    }

    #[test]
    fn test_detect_plain_text() {
        assert_eq!(detect("Hello, world!"), ContentType::Text);
        assert_eq!(detect("This is just some regular text."), ContentType::Text);
        assert_eq!(detect(""), ContentType::Text);
    }
}
