//! Files submitted as evidence by a learner's agent. The platform never reads the
//! learner's machine directly; agents send the relevant files through MCP.

use lp_core::ExtensionError;
use serde::Deserialize;

pub const MAX_FILES: usize = 25;
pub const MAX_FILE_BYTES: usize = 200 * 1024;
pub const MAX_TOTAL_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmittedFile {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileEvidence {
    pub files: Vec<SubmittedFile>,
}

impl FileEvidence {
    pub fn checked(self) -> Result<Self, ExtensionError> {
        if self.files.len() > MAX_FILES {
            return Err(invalid(format!(
                "at most {MAX_FILES} files can be submitted"
            )));
        }
        let mut total = 0;
        for file in &self.files {
            validate_relative_path(&file.path)?;
            if file.content.len() > MAX_FILE_BYTES {
                return Err(invalid(format!(
                    "`{}` exceeds {} KiB",
                    file.path,
                    MAX_FILE_BYTES / 1024
                )));
            }
            total += file.content.len();
        }
        if total > MAX_TOTAL_BYTES {
            return Err(invalid("submitted files exceed 1 MiB in total".into()));
        }
        Ok(self)
    }
}

pub fn validate_relative_path(path: &str) -> Result<(), ExtensionError> {
    let suspicious = path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|segment| segment == ".." || segment.is_empty());
    if suspicious {
        Err(invalid(format!(
            "`{path}` must be a relative path inside the project"
        )))
    } else {
        Ok(())
    }
}

fn invalid(message: String) -> ExtensionError {
    ExtensionError::InvalidSubmission(message)
}

/// Matches `/`-separated paths. `*` matches within one segment, `**` matches any
/// number of segments.
pub fn path_matches(pattern: &str, path: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').collect();
    let path: Vec<&str> = path.trim_start_matches("./").split('/').collect();
    match_segments(&pattern, &path)
}

fn match_segments(pattern: &[&str], path: &[&str]) -> bool {
    match (pattern.first(), path.first()) {
        (None, None) => true,
        (Some(&"**"), _) => {
            match_segments(&pattern[1..], path)
                || (!path.is_empty() && match_segments(pattern, &path[1..]))
        }
        (Some(segment), Some(candidate)) => {
            wildcard(segment, candidate) && match_segments(&pattern[1..], &path[1..])
        }
        _ => false,
    }
}

fn wildcard(pattern: &str, text: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == text,
        Some((prefix, rest)) => {
            let Some(remaining) = text.strip_prefix(prefix) else {
                return false;
            };
            (0..=remaining.len())
                .filter(|index| remaining.is_char_boundary(*index))
                .any(|index| wildcard(rest, &remaining[index..]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_globs() {
        assert!(path_matches("SKILL.md", "SKILL.md"));
        assert!(path_matches("skills/*/SKILL.md", "skills/review/SKILL.md"));
        assert!(!path_matches("skills/*/SKILL.md", "skills/a/b/SKILL.md"));
        assert!(path_matches("**/SKILL.md", "a/b/SKILL.md"));
        assert!(path_matches("**/SKILL.md", "SKILL.md"));
        assert!(path_matches("*.md", "README.md"));
        assert!(!path_matches("*.md", "README.txt"));
    }

    #[test]
    fn rejects_escaping_paths() {
        for path in ["../etc/passwd", "/etc/passwd", "a//b", "a\\b", ""] {
            assert!(validate_relative_path(path).is_err(), "{path}");
        }
        assert!(validate_relative_path(".github/skills/x/SKILL.md").is_ok());
    }
}
