//! Shell hook generation for bash and zsh integration.
//!
//! These hooks capture command metadata (command text, cwd, exit code,
//! duration, timestamp) and send it to the ggnmem daemon via the CLI
//! `ingest` subcommand.
//!
//! Both hooks:
//! - Generate a session ID at source time
//! - Run `ggnmem ingest` in the background (`&`) with `disown` for zero prompt latency
//! - Use millisecond timestamps via `date +%s%3N`
//! - Capture hostname for session context

/// Returns the zsh shell integration script.
///
/// Users source this via: `eval "$(ggnmem init zsh)"`
#[must_use]
pub fn zsh_hook() -> &'static str {
    r#"# ggnmem — zsh shell integration
# Add to your .zshrc:
#   eval "$(ggnmem init zsh)"

# Generate a unique session ID for this shell instance.
__ggnmem_session_id="${__ggnmem_session_id:-$(cat /proc/sys/kernel/random/uuid 2>/dev/null || echo "$$-$(date +%s)")}"
__ggnmem_hostname="$(hostname 2>/dev/null || echo 'unknown')"

__ggnmem_preexec() {
    __ggnmem_cmd="$1"
    __ggnmem_start_ms="$(date +%s%3N 2>/dev/null || echo 0)"
}

__ggnmem_precmd() {
    local exit_code=$?
    [ -z "$__ggnmem_cmd" ] && return

    local end_ms
    end_ms="$(date +%s%3N 2>/dev/null || echo 0)"
    local duration_ms=$(( end_ms - __ggnmem_start_ms ))

    # Fire and forget — do not block the prompt.
    ggnmem ingest \
        --command "$__ggnmem_cmd" \
        --cwd "$PWD" \
        --exit-code "$exit_code" \
        --duration-ms "$duration_ms" \
        --shell zsh \
        --session-id "$__ggnmem_session_id" \
        --hostname "$__ggnmem_hostname" \
        --started-at-ms "$__ggnmem_start_ms" \
        --completed-at-ms "$end_ms" &>/dev/null &
    disown 2>/dev/null

    unset __ggnmem_cmd
    unset __ggnmem_start_ms
}

autoload -Uz add-zsh-hook
add-zsh-hook preexec __ggnmem_preexec
add-zsh-hook precmd __ggnmem_precmd
"#
}

/// Returns the bash shell integration script.
///
/// Users source this via: `eval "$(ggnmem init bash)"`
#[must_use]
pub fn bash_hook() -> &'static str {
    r#"# ggnmem — bash shell integration
# Add to your .bashrc:
#   eval "$(ggnmem init bash)"

# Generate a unique session ID for this shell instance.
__ggnmem_session_id="${__ggnmem_session_id:-$(cat /proc/sys/kernel/random/uuid 2>/dev/null || echo "$$-$(date +%s)")}"
__ggnmem_hostname="$(hostname 2>/dev/null || echo 'unknown')"

__ggnmem_preexec() {
    # Skip completion and prompt-command invocations.
    [ -n "$COMP_LINE" ] && return
    [ "$BASH_COMMAND" = "$PROMPT_COMMAND" ] && return

    # Capture the actual command from history (more reliable than BASH_COMMAND).
    __ggnmem_cmd="$(HISTTIMEFORMAT= history 1 | sed 's/^[ ]*[0-9]*[ ]*//')"
    __ggnmem_start_ms="$(date +%s%3N 2>/dev/null || echo 0)"
}

__ggnmem_precmd() {
    local exit_code=$?
    [ -z "$__ggnmem_cmd" ] && return

    local end_ms
    end_ms="$(date +%s%3N 2>/dev/null || echo 0)"
    local duration_ms=$(( end_ms - __ggnmem_start_ms ))

    # Fire and forget — do not block the prompt.
    ggnmem ingest \
        --command "$__ggnmem_cmd" \
        --cwd "$PWD" \
        --exit-code "$exit_code" \
        --duration-ms "$duration_ms" \
        --shell bash \
        --session-id "$__ggnmem_session_id" \
        --hostname "$__ggnmem_hostname" \
        --started-at-ms "$__ggnmem_start_ms" \
        --completed-at-ms "$end_ms" &>/dev/null &
    disown 2>/dev/null

    unset __ggnmem_cmd
    unset __ggnmem_start_ms
}

trap '__ggnmem_preexec' DEBUG
PROMPT_COMMAND="__ggnmem_precmd${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
"#
}
