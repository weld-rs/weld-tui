use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Detected line ending style of a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
}

impl LineEnding {
    pub fn as_str(&self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
        }
    }
}

/// A file's content, split into lines with metadata.
#[derive(Debug, Clone)]
pub struct Content {
    /// Path used to load this file content.
    pub(crate) path: PathBuf,
    /// File content split into lines, without line terminators.
    pub(crate) lines: Vec<String>,
    /// Detected line ending style, preserved on save.
    pub(crate) line_ending: LineEnding,
    /// Whether the original file ended with a newline.
    pub(crate) has_trailing_newline: bool,
}

impl Content {
    /// Load a file from disk as UTF-8 text.
    /// Detects line ending style (LF vs CRLF) and normalizes internally.
    /// Fails loudly if the file doesn't exist or isn't readable.
    pub fn load(path: &Path) -> io::Result<Self> {
        let raw = fs::read_to_string(path).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("failed to read {}: {err}", path.display()),
            )
        })?;

        if raw.is_empty() {
            return Ok(Content {
                path: path.to_path_buf(),
                lines: vec![],
                line_ending: LineEnding::Lf,
                has_trailing_newline: false,
            });
        }

        let line_ending = if raw.contains("\r\n") {
            LineEnding::CrLf
        } else {
            LineEnding::Lf
        };

        let has_trailing_newline = raw.ends_with('\n');

        let normalized = raw.replace("\r\n", "\n");
        let lines: Vec<String> = normalized.split('\n').map(String::from).collect();

        // Remove trailing empty string from final newline
        let lines = if lines.last().is_some_and(|l| l.is_empty()) {
            lines[..lines.len() - 1].to_vec()
        } else {
            lines
        };

        Ok(Content {
            path: path.to_path_buf(),
            lines,
            line_ending,
            has_trailing_newline,
        })
    }

    /// Save lines back to disk using the original line ending style.
    pub fn save(&self) -> io::Result<()> {
        let ending = self.line_ending.as_str();
        let mut content = self.lines.join(ending);
        if self.has_trailing_newline {
            content.push_str(ending);
        }
        fs::write(&self.path, content)
    }

    /// Reconstruct the full text content (LF-normalized) for diffing.
    pub fn text(&self) -> String {
        let mut text = self.lines.join("\n");
        if self.has_trailing_newline {
            text.push('\n');
        }
        text
    }

    /// The lines of the file, without line terminators.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// The path this content was loaded from (or empty for in-memory content).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Replace a range of lines with new content.
    ///
    /// # Panics
    /// Panics if `range` is out of bounds (i.e., `range.end > self.lines.len()`).
    pub fn splice_lines(&mut self, range: std::ops::Range<usize>, replacement: Vec<String>) {
        assert!(
            range.end <= self.lines.len(),
            "splice_lines: range {}..{} out of bounds for {} lines",
            range.start,
            range.end,
            self.lines.len(),
        );
        self.lines.splice(range, replacement);
    }

    /// Construct a Content from raw lines (for testing outside weld-core).
    pub fn from_lines(lines: &[&str]) -> Self {
        Content {
            path: PathBuf::new(),
            lines: lines.iter().map(|s| s.to_string()).collect(),
            line_ending: LineEnding::Lf,
            has_trailing_newline: !lines.is_empty(),
        }
    }
}

/// Replace the home directory prefix with ~ for display.
/// Uses path-based prefix matching to avoid false positives
/// (e.g., /Users/al matching /Users/alice).
pub fn shorten_dir(path: &str) -> String {
    let path = Path::new(path);
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    if let Some(home) = home {
        let home = PathBuf::from(home);
        if let Ok(rest) = path.strip_prefix(&home) {
            return if rest.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~/{}", rest.display())
            };
        }
    }
    path.display().to_string()
}

impl fmt::Display for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for line in &self.lines {
            writeln!(f, "{line}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_lf_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2\nline3\n").unwrap();

        let content = Content::load(&path).unwrap();

        assert_eq!(content.lines, vec!["line1", "line2", "line3"]);
        assert_eq!(content.line_ending, LineEnding::Lf);
        assert!(content.has_trailing_newline);
    }

    #[test]
    fn load_crlf_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\r\nline2\r\nline3\r\n").unwrap();

        let content = Content::load(&path).unwrap();

        assert_eq!(content.lines, vec!["line1", "line2", "line3"]);
        assert_eq!(content.line_ending, LineEnding::CrLf);
        assert!(content.has_trailing_newline);
    }

    #[test]
    fn load_no_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2").unwrap();

        let content = Content::load(&path).unwrap();

        assert_eq!(content.lines, vec!["line1", "line2"]);
        assert!(!content.has_trailing_newline);
    }

    #[test]
    fn save_preserves_lf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2\n").unwrap();

        let content = Content::load(&path).unwrap();
        content.save().unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert_eq!(raw, "line1\nline2\n");
    }

    #[test]
    fn save_preserves_crlf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\r\nline2\r\n").unwrap();

        let content = Content::load(&path).unwrap();
        content.save().unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert_eq!(raw, "line1\r\nline2\r\n");
    }

    #[test]
    fn save_preserves_no_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "line1\nline2").unwrap();

        let content = Content::load(&path).unwrap();
        content.save().unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert_eq!(raw, "line1\nline2");
    }

    #[test]
    fn text_returns_lf_normalized() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "a\r\nb\r\n").unwrap();

        let content = Content::load(&path).unwrap();
        assert_eq!(content.text(), "a\nb\n");
    }

    #[test]
    fn text_no_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        fs::write(&path, "a\nb").unwrap();

        let content = Content::load(&path).unwrap();
        assert_eq!(content.text(), "a\nb");
    }

    #[test]
    fn load_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.txt");
        fs::write(&path, "").unwrap();

        let content = Content::load(&path).unwrap();
        assert!(content.lines.is_empty());
        assert!(!content.has_trailing_newline);
        assert_eq!(content.line_ending, LineEnding::Lf);
    }

    #[test]
    fn round_trip_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.txt");
        fs::write(&path, "").unwrap();

        let content = Content::load(&path).unwrap();
        content.save().unwrap();

        let raw = fs::read_to_string(&path).unwrap();
        assert_eq!(raw, "");
    }

    #[test]
    fn round_trip_no_trailing_newline() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let original = "hello\nworld";
        fs::write(&path, original).unwrap();

        let content = Content::load(&path).unwrap();
        content.save().unwrap();

        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(original, after);
    }

    #[test]
    fn round_trip_identical_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("round.txt");
        let original = "func main() {\n\tfmt.Println(\"hello\")\n}\n";
        fs::write(&path, original).unwrap();

        let content = Content::load(&path).unwrap();
        content.save().unwrap();

        let after = fs::read_to_string(&path).unwrap();
        assert_eq!(original, after);
    }

    #[test]
    fn mixed_line_endings_normalizes_to_detected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mixed.txt");
        fs::write(&path, "line1\r\nline2\nline3\n").unwrap();

        let content = Content::load(&path).unwrap();
        assert_eq!(content.line_ending, LineEnding::CrLf);
        assert_eq!(content.lines, vec!["line1", "line2", "line3"]);

        content.save().unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        assert_eq!(raw, "line1\r\nline2\r\nline3\r\n");
    }

    #[test]
    fn shorten_dir_replaces_home() {
        let home = std::env::var("HOME").unwrap();
        let input = format!("{home}/projects/weld");
        assert_eq!(shorten_dir(&input), "~/projects/weld");
    }

    #[test]
    fn shorten_dir_home_alone() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(shorten_dir(&home), "~");
    }

    #[test]
    fn shorten_dir_no_match() {
        assert_eq!(shorten_dir("/tmp/other/path"), "/tmp/other/path");
    }

    #[test]
    fn shorten_dir_no_false_prefix() {
        // /Users/al should NOT match /Users/alice
        let home = std::env::var("HOME").unwrap();
        let fake = format!("{home}extra/something");
        assert_eq!(shorten_dir(&fake), fake);
    }

    #[test]
    fn load_missing_file_fails() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.txt");
        let result = Content::load(&missing);
        assert!(result.is_err());
    }

    #[test]
    fn load_error_includes_path() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope.txt");
        let err = Content::load(&missing).unwrap_err();
        assert!(
            err.to_string().contains("nope.txt"),
            "error should include filename: {err}"
        );
    }
}
