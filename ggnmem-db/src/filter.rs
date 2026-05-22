//! Pre-ingestion command filter.
//!
//! Determines whether a captured shell command should be stored in the
//! database. Rejects noise like internal ggnmem commands, shell control
//! sequences, bare builtins, and credential-bearing commands.

/// Returns `true` if the command should be ingested, `false` if it should
/// be silently dropped.
#[must_use]
pub fn should_ingest(command: &str) -> bool {
    let trimmed = command.trim();

    // ── Empty / trivial ──────────────────────────────────────────────
    if trimmed.is_empty() {
        return false;
    }
    // Single-character noise (accidental keystrokes).
    if trimmed.len() <= 1 {
        return false;
    }

    let first_token = trimmed.split_whitespace().next().unwrap_or("");
    let first_lower = first_token.to_lowercase();

    // ── Internal ggnmem commands ─────────────────────────────────────
    if first_lower == "ggnmem" || first_lower.starts_with("ggnmem-") {
        return false;
    }

    // ── Shell control / session commands ──────────────────────────────
    const SHELL_CONTROL: &[&str] = &[
        "exit", "logout", "clear", "reset", "history", "true", "false", ":", "source", ".",
    ];
    if SHELL_CONTROL.contains(&first_lower.as_str()) {
        return false;
    }

    // ── Navigation-only commands (bare cd, pushd, popd) ──────────────
    const NAV_ONLY: &[&str] = &["cd", "pushd", "popd"];
    if NAV_ONLY.contains(&first_lower.as_str()) {
        return false;
    }

    // ── Environment manipulation without useful payload ──────────────
    const ENV_NOISE: &[&str] = &["export", "unset", "set", "alias", "unalias", "eval"];
    if ENV_NOISE.contains(&first_lower.as_str()) {
        return false;
    }

    // ── Credential / secret patterns ─────────────────────────────────
    // Drop commands that likely contain inline secrets.
    let lower = trimmed.to_lowercase();
    if lower.contains("password=")
        || lower.contains("passwd=")
        || lower.contains("secret=")
        || lower.contains("token=")
        || lower.contains("api_key=")
        || lower.contains("apikey=")
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Allowed commands ─────────────────────────────────────────────

    #[test]
    fn allows_normal_commands() {
        assert!(should_ingest("docker ps"));
        assert!(should_ingest("git status"));
        assert!(should_ingest("cargo build --release"));
        assert!(should_ingest("npm install"));
        assert!(should_ingest("kubectl get pods"));
        assert!(should_ingest("ls -la"));
        assert!(should_ingest("cat /etc/hosts"));
        assert!(should_ingest("grep -r TODO ."));
    }

    // ── Rejected: ggnmem internal ────────────────────────────────────

    #[test]
    fn rejects_ggnmem_commands() {
        assert!(!should_ingest("ggnmem search docker"));
        assert!(!should_ingest("ggnmem ping"));
        assert!(!should_ingest("ggnmem doctor"));
        assert!(!should_ingest("ggnmem count"));
        assert!(!should_ingest("ggnmem ingest --command foo"));
        assert!(!should_ingest("ggnmem-daemon"));
        assert!(!should_ingest("ggnmem-cli"));
    }

    // ── Rejected: shell control ──────────────────────────────────────

    #[test]
    fn rejects_shell_control() {
        assert!(!should_ingest("exit"));
        assert!(!should_ingest("logout"));
        assert!(!should_ingest("clear"));
        assert!(!should_ingest("reset"));
        assert!(!should_ingest("history"));
        assert!(!should_ingest("true"));
        assert!(!should_ingest("false"));
        assert!(!should_ingest(":"));
        assert!(!should_ingest("source ~/.bashrc"));
        assert!(!should_ingest(". ~/.zshrc"));
    }

    // ── Rejected: navigation only ────────────────────────────────────

    #[test]
    fn rejects_bare_navigation() {
        assert!(!should_ingest("cd"));
        assert!(!should_ingest("cd /tmp"));
        assert!(!should_ingest("pushd /tmp"));
        assert!(!should_ingest("popd"));
    }

    // ── Rejected: env noise ──────────────────────────────────────────

    #[test]
    fn rejects_env_manipulation() {
        assert!(!should_ingest("export PATH=/usr/bin:$PATH"));
        assert!(!should_ingest("unset FOO"));
        assert!(!should_ingest("set -e"));
        assert!(!should_ingest("alias ll='ls -la'"));
        assert!(!should_ingest("eval $(ssh-agent)"));
    }

    // ── Rejected: trivial / empty ────────────────────────────────────

    #[test]
    fn rejects_trivial() {
        assert!(!should_ingest(""));
        assert!(!should_ingest("   "));
        assert!(!should_ingest("a"));
        assert!(!should_ingest(" x "));
    }

    // ── Rejected: credential patterns ────────────────────────────────

    #[test]
    fn rejects_credential_patterns() {
        assert!(!should_ingest("curl -u user password=secret http://x"));
        assert!(!should_ingest("mysql -p passwd=hunter2"));
        assert!(!should_ingest("export API_KEY=abc123"));
        assert!(!should_ingest("MY_TOKEN=xyz ./run.sh"));
    }

    // ── Edge cases ───────────────────────────────────────────────────

    #[test]
    fn case_insensitive() {
        assert!(!should_ingest("GGNMEM search docker"));
        assert!(!should_ingest("EXIT"));
        assert!(!should_ingest("Clear"));
    }

    #[test]
    fn allows_commands_with_similar_names() {
        // "git" is not "ggnmem", etc.
        assert!(should_ingest("git commit -m 'exit fix'"));
        assert!(should_ingest("echo clear"));
        assert!(should_ingest("rm -rf /tmp/history"));
    }
}
