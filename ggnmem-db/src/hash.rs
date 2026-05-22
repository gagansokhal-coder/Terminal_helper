use sha2::{Digest, Sha256};

pub fn normalize_command(command: &str) -> String {
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[must_use]
pub fn content_hash(command: &str, cwd: &str) -> String {
    let normalized_command = normalize_command(command);
    let normalized_cwd = cwd.trim();
    let mut hasher = Sha256::new();
    hasher.update(normalized_command.as_bytes());
    hasher.update(b"\0");
    hasher.update(normalized_cwd.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_normalizes_command_spacing() {
        assert_eq!(
            content_hash("git   status", "/tmp/project"),
            content_hash("git status", "/tmp/project")
        );
    }

    #[test]
    fn content_hash_keeps_cwd_in_identity() {
        assert_ne!(
            content_hash("git status", "/tmp/project-a"),
            content_hash("git status", "/tmp/project-b")
        );
    }
}
