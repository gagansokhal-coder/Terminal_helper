//! Interactive TUI for ggnmem search.
//!
//! Full-screen ratatui interface with:
//! - Hybrid search input with live filtering (FTS + Semantic + RRF)
//! - Natural language queries ("check git changes" → git status)
//! - Search mode toggles: Ctrl+F (FTS), Ctrl+S (Semantic), Ctrl+H (Hybrid)
//! - Scrollable results list with match highlighting and source labels
//! - Detail preview panel (always visible, toggled with Tab)
//! - Enter inserts command into shell prompt
//! - Shift+Enter executes command immediately through the shell hook
//! - Ctrl+L clears query, Esc exits
//! - Shift+C copies to clipboard
//! - Shift+I toggles internal command visibility
//! - Shift+P pins a command, Shift+F shows favorites only
//! - Status bar: [MODE] N results | Xms

use std::collections::HashSet;
use std::io;
use std::path::PathBuf;
use std::process;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ggnmem_daemon::{
    protocol::{DaemonRequest, DaemonResponseKind, SearchMode, SearchResultSummary, SearchSource},
    DaemonConfig, IpcClient,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};

// ─── Color palette ───────────────────────────────────────────────────────────

const BG_PRIMARY: Color = Color::Rgb(18, 18, 24);
const BG_SECONDARY: Color = Color::Rgb(26, 26, 36);
const BG_HIGHLIGHT: Color = Color::Rgb(38, 38, 56);
const FG_PRIMARY: Color = Color::Rgb(220, 220, 235);
const FG_SECONDARY: Color = Color::Rgb(140, 140, 165);
const FG_DIM: Color = Color::Rgb(80, 80, 100);
const ACCENT_CYAN: Color = Color::Rgb(80, 200, 255);
const ACCENT_GREEN: Color = Color::Rgb(80, 220, 160);
const ACCENT_YELLOW: Color = Color::Rgb(255, 200, 80);
const ACCENT_PURPLE: Color = Color::Rgb(180, 120, 255);
const ACCENT_RED: Color = Color::Rgb(255, 100, 100);
const ACCENT_ORANGE: Color = Color::Rgb(255, 160, 80);
const ACCENT_PINK: Color = Color::Rgb(255, 120, 180);
const BORDER_ACTIVE: Color = Color::Rgb(80, 120, 200);
const BORDER_INACTIVE: Color = Color::Rgb(50, 50, 70);

// ─── Debounce ────────────────────────────────────────────────────────────────

const SEARCH_DEBOUNCE_MS: u64 = 120;
const SEARCH_LIMIT: u32 = 50;
const EXIT_INSERT: i32 = 10;
const EXIT_EXECUTE: i32 = 11;

// ─── App state ───────────────────────────────────────────────────────────────

struct App {
    query: String,
    cursor_pos: usize,
    /// Raw results from daemon (unfiltered).
    all_results: Vec<SearchResultSummary>,
    /// Filtered results for display.
    results: Vec<SearchResultSummary>,
    list_state: ListState,
    show_preview: bool,
    show_internal: bool,
    recent_only: bool,
    show_favorites_only: bool,
    show_source_labels: bool,
    pinned: HashSet<String>,
    status_msg: String,
    search_pending: bool,
    last_keystroke: Instant,
    last_search_query: String,
    /// Track which mode was used for last search so mode switches force re-search.
    last_search_mode: SearchMode,
    total_commands: u64,
    /// Current search mode.
    search_mode: SearchMode,
    /// Latency of the last search in milliseconds.
    search_latency_ms: Option<u64>,
    /// Command to insert into the shell's editable prompt after TUI exit.
    insert_command: Option<String>,
    /// Command to ask the shell hook to execute immediately after TUI exit.
    execute_command: Option<String>,
    /// Clipboard feedback: (message, success, timestamp).
    clipboard_feedback: Option<(String, bool, Instant)>,
    cwd: Option<String>,
}

impl App {
    fn new() -> Self {
        let cwd = std::env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(String::from));
        Self {
            query: String::new(),
            cursor_pos: 0,
            all_results: Vec::new(),
            results: Vec::new(),
            list_state: ListState::default(),
            show_preview: true,
            show_internal: false,
            recent_only: false,
            show_favorites_only: false,
            show_source_labels: true,
            pinned: HashSet::new(),
            status_msg: String::new(),
            search_pending: false,
            last_keystroke: Instant::now(),
            last_search_query: String::new(),
            last_search_mode: SearchMode::Hybrid,
            total_commands: 0,
            search_mode: SearchMode::Hybrid,
            search_latency_ms: None,
            insert_command: None,
            execute_command: None,
            clipboard_feedback: None,
            cwd,
        }
    }

    fn selected_result(&self) -> Option<&SearchResultSummary> {
        self.list_state.selected().and_then(|i| self.results.get(i))
    }

    fn select_next(&mut self) {
        if self.results.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) if i >= self.results.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn select_prev(&mut self) {
        if self.results.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(0) => self.results.len() - 1,
            Some(i) => i - 1,
            None => self.results.len().saturating_sub(1),
        };
        self.list_state.select(Some(i));
    }

    fn needs_search(&self) -> bool {
        self.search_pending
            && self.last_keystroke.elapsed() >= Duration::from_millis(SEARCH_DEBOUNCE_MS)
            && (self.query != self.last_search_query || self.search_mode != self.last_search_mode)
    }

    fn mark_dirty(&mut self) {
        self.search_pending = true;
        self.last_keystroke = Instant::now();
    }

    /// Re-filter `all_results` into `results` based on current toggles.
    fn apply_filters(&mut self) {
        self.results = self
            .all_results
            .iter()
            .filter(|r| {
                // Hide internal commands unless toggled on.
                if !self.show_internal && ggnmem_db::is_internal_command(&r.command) {
                    return false;
                }
                // Favorites mode: only show pinned commands.
                if self.show_favorites_only && !self.pinned.contains(&r.command) {
                    return false;
                }
                true
            })
            .cloned()
            .collect();

        // Sort pinned commands to top.
        let pinned = &self.pinned;
        self.results.sort_by(|a, b| {
            let a_pin = pinned.contains(&a.command);
            let b_pin = pinned.contains(&b.command);
            b_pin.cmp(&a_pin)
        });

        // Fix selection.
        if self.results.is_empty() {
            self.list_state.select(None);
        } else {
            let sel = self
                .list_state
                .selected()
                .unwrap_or(0)
                .min(self.results.len() - 1);
            self.list_state.select(Some(sel));
        }
    }

    fn update_status(&mut self) {
        let count = self.results.len();
        let mode_str = format!("{}", self.search_mode);
        let latency_str = self
            .search_latency_ms
            .map(|ms| format!(" | {ms} ms"))
            .unwrap_or_default();

        if self.show_favorites_only {
            self.status_msg = format!("[{mode_str}] ★ {count} pinned{latency_str}");
        } else if self.recent_only {
            self.status_msg = format!("[{mode_str}] ⏱ {count} recent{latency_str}");
        } else if self.query.is_empty() {
            self.status_msg = format!("[{mode_str}] {count} commands{latency_str}");
        } else {
            self.status_msg = format!(
                "[{mode_str}] {count} result{}{latency_str}",
                if count == 1 { "" } else { "s" }
            );
        }
    }
}

// ─── Public entry point ──────────────────────────────────────────────────────

pub async fn run_tui() -> Result<()> {
    enable_raw_mode().context("enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("create terminal")?;
    terminal.clear()?;

    let mut app = App::new();

    // Fetch initial command count.
    if let Ok(count) = fetch_count().await {
        app.total_commands = count;
    }

    // Load recent commands as initial results.
    if let Ok((results, latency_ms)) = do_search("", app.cwd.clone(), app.search_mode).await {
        app.all_results = results;
        app.search_latency_ms = Some(latency_ms);
        app.apply_filters();
        app.update_status();
    }

    let result = run_event_loop(&mut terminal, &mut app).await;

    // Restore terminal.
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Post-exit actions. The shell widget owns readline insertion/execution.
    if let Some(cmd) = &app.execute_command {
        write_action_file("execute", cmd)?;
        process::exit(EXIT_EXECUTE);
    } else if let Some(cmd) = &app.insert_command {
        write_action_file("insert", cmd)?;
        process::exit(EXIT_INSERT);
    }

    result
}

async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| draw_ui(f, app))?;

        let poll_timeout = if app.search_pending {
            Duration::from_millis(20)
        } else {
            Duration::from_millis(50)
        };

        if event::poll(poll_timeout)? {
            if let Event::Key(key) = event::read()? {
                if handle_key(app, key) {
                    break;
                }
            }
        }

        // Fire debounced search.
        if app.needs_search() {
            let query = app.query.clone();
            let mode = app.search_mode;
            app.last_search_query = query.clone();
            app.last_search_mode = mode;
            app.search_pending = false;

            match do_search(&query, app.cwd.clone(), mode).await {
                Ok((results, latency_ms)) => {
                    app.all_results = results;
                    app.search_latency_ms = Some(latency_ms);
                    app.apply_filters();
                    app.update_status();
                }
                Err(e) => {
                    app.status_msg = format!("search error: {e}");
                    app.all_results.clear();
                    app.results.clear();
                    app.list_state.select(None);
                }
            }
        }

        // Clear clipboard feedback after 2s.
        if let Some((_, _, ts)) = &app.clipboard_feedback {
            if ts.elapsed() > Duration::from_secs(2) {
                app.clipboard_feedback = None;
            }
        }
    }
    Ok(())
}

// ─── Key handling ────────────────────────────────────────────────────────────

/// Returns true if the app should exit.
fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        // ── Exit ──
        KeyCode::Esc => return true,

        // ── Ctrl+C: copy selected command + exit ──
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(result) = app.selected_result() {
                let _ = copy_to_clipboard(&result.command.clone());
            }
            return true;
        }

        // ── Shift+Enter: ask the shell hook to execute selected command ──
        KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
            if let Some(result) = app.selected_result() {
                app.execute_command = Some(result.command.clone());
                return true;
            }
        }

        // ── Enter: insert selected command into shell prompt ──
        KeyCode::Enter => {
            if let Some(result) = app.selected_result() {
                app.insert_command = Some(result.command.clone());
                return true;
            }
        }

        // ── Ctrl+R: same as Enter (backward compat) ──
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(result) = app.selected_result() {
                app.insert_command = Some(result.command.clone());
                return true;
            }
        }

        // ── Ctrl+L: clear query ──
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.query.clear();
            app.cursor_pos = 0;
            app.last_search_query = String::new();
            app.mark_dirty();
        }

        // ── Ctrl+F: toggle FTS-only mode ──
        KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.search_mode = if app.search_mode == SearchMode::FtsOnly {
                SearchMode::Hybrid
            } else {
                SearchMode::FtsOnly
            };
            app.mark_dirty();
        }

        // ── Ctrl+S: toggle Semantic-only mode ──
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.search_mode = if app.search_mode == SearchMode::SemanticOnly {
                SearchMode::Hybrid
            } else {
                SearchMode::SemanticOnly
            };
            app.mark_dirty();
        }

        // ── Ctrl+H: toggle Hybrid mode ──
        KeyCode::Char('h') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.search_mode = SearchMode::Hybrid;
            app.mark_dirty();
        }

        // ── Tab: toggle preview ──
        KeyCode::Tab => {
            app.show_preview = !app.show_preview;
        }

        // ── Navigation ──
        KeyCode::Down => app.select_next(),
        KeyCode::Up => app.select_prev(),

        // ── Action keys (UPPERCASE = action, lowercase = type into search) ──

        // Shift+C: copy to clipboard (stay in UI)
        KeyCode::Char('C') => {
            if let Some(result) = app.selected_result() {
                let cmd = result.command.clone();
                if copy_to_clipboard(&cmd) {
                    let short = if cmd.len() > 40 {
                        format!("{}…", &cmd[..40])
                    } else {
                        cmd
                    };
                    app.clipboard_feedback =
                        Some((format!("Copied: {short}"), true, Instant::now()));
                } else {
                    app.clipboard_feedback =
                        Some(("Clipboard unavailable".to_string(), false, Instant::now()));
                }
            }
        }

        // Shift+I: toggle internal command visibility
        KeyCode::Char('I') => {
            app.show_internal = !app.show_internal;
            app.apply_filters();
            app.update_status();
        }

        // Shift+P: pin/unpin selected command
        KeyCode::Char('P') => {
            if let Some(result) = app.selected_result() {
                let cmd = result.command.clone();
                if app.pinned.contains(&cmd) {
                    app.pinned.remove(&cmd);
                } else {
                    app.pinned.insert(cmd);
                }
                app.apply_filters();
            }
        }

        // Shift+F: show favorites (pinned) only
        KeyCode::Char('F') => {
            app.show_favorites_only = !app.show_favorites_only;
            app.apply_filters();
            app.update_status();
        }

        // Shift+R: toggle recent-only mode
        KeyCode::Char('R') => {
            app.recent_only = !app.recent_only;
            // Force re-search to switch between search and recent modes.
            app.last_search_query = String::new();
            app.mark_dirty();
        }

        // ── Text input (lowercase chars) ──
        KeyCode::Char(c) => {
            app.query.insert(app.cursor_pos, c);
            app.cursor_pos += 1;
            app.mark_dirty();
        }

        KeyCode::Backspace if app.cursor_pos > 0 => {
            app.cursor_pos -= 1;
            app.query.remove(app.cursor_pos);
            app.mark_dirty();
        }

        KeyCode::Delete if app.cursor_pos < app.query.len() => {
            app.query.remove(app.cursor_pos);
            app.mark_dirty();
        }

        KeyCode::Left if app.cursor_pos > 0 => {
            app.cursor_pos -= 1;
        }

        KeyCode::Right if app.cursor_pos < app.query.len() => {
            app.cursor_pos += 1;
        }

        KeyCode::Home => app.cursor_pos = 0,
        KeyCode::End => app.cursor_pos = app.query.len(),

        _ => {}
    }
    false
}

// ─── Drawing ─────────────────────────────────────────────────────────────────

fn draw_ui(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // Fill background.
    f.render_widget(
        Block::default().style(Style::default().bg(BG_PRIMARY)),
        area,
    );

    // Main layout: [search 3] [body flex] [footer 2]
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

    draw_search_input(f, app, outer[0]);
    draw_body(f, app, outer[1]);
    draw_footer(f, app, outer[2]);
}

fn draw_search_input(f: &mut Frame, app: &App, area: Rect) {
    let mode_color = match app.search_mode {
        SearchMode::Hybrid => ACCENT_YELLOW,
        SearchMode::FtsOnly => ACCENT_CYAN,
        SearchMode::SemanticOnly => ACCENT_PURPLE,
    };
    let mut title_parts = vec![
        Span::styled(
            " 🔍 ggnmem ",
            Style::default()
                .fg(ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("[{}] ", app.search_mode),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ),
    ];

    // Mode indicators.
    if app.show_internal {
        title_parts.push(Span::styled(
            " ⚙ INTERNAL ",
            Style::default().fg(ACCENT_ORANGE),
        ));
    }
    if app.recent_only {
        title_parts.push(Span::styled(
            " ⏱ RECENT ",
            Style::default().fg(ACCENT_PURPLE),
        ));
    }
    if app.show_favorites_only {
        title_parts.push(Span::styled(
            " ★ FAVORITES ",
            Style::default().fg(ACCENT_YELLOW),
        ));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_ACTIVE))
        .title(Line::from(title_parts))
        .style(Style::default().bg(BG_SECONDARY));

    let input_text = if app.query.is_empty() {
        vec![
            Span::styled(
                "type to search commands…",
                Style::default().fg(FG_DIM).add_modifier(Modifier::ITALIC),
            ),
            Span::styled(" ", Style::default().bg(ACCENT_CYAN)),
        ]
    } else {
        let before = &app.query[..app.cursor_pos];
        if app.cursor_pos < app.query.len() {
            let cursor_ch = &app.query[app.cursor_pos..app.cursor_pos + 1];
            let after = &app.query[app.cursor_pos + 1..];
            vec![
                Span::styled(before, Style::default().fg(FG_PRIMARY)),
                Span::styled(
                    cursor_ch,
                    Style::default()
                        .fg(BG_PRIMARY)
                        .bg(ACCENT_CYAN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(after, Style::default().fg(FG_PRIMARY)),
            ]
        } else {
            vec![
                Span::styled(before, Style::default().fg(FG_PRIMARY)),
                Span::styled(" ", Style::default().bg(ACCENT_CYAN)),
            ]
        }
    };

    let paragraph = Paragraph::new(Line::from(input_text))
        .block(block)
        .style(Style::default().bg(BG_SECONDARY));
    f.render_widget(paragraph, area);
}

fn draw_body(f: &mut Frame, app: &mut App, area: Rect) {
    if app.show_preview {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(area);
        draw_results_list(f, app, chunks[0]);
        draw_preview(f, app, chunks[1]);
    } else {
        draw_results_list(f, app, area);
    }
}

fn draw_results_list(f: &mut Frame, app: &mut App, area: Rect) {
    let show_labels = app.show_source_labels;
    let items: Vec<ListItem> = app
        .results
        .iter()
        .map(|result| result_to_list_item(result, &app.query, &app.pinned, show_labels))
        .collect();

    let count = app.results.len();
    let title = if app.show_favorites_only {
        format!(" ★ Pinned ({count}) ")
    } else if app.query.is_empty() {
        format!(" Recent Commands ({count}) ")
    } else {
        format!(" Results ({count}) ")
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_INACTIVE))
        .title(Span::styled(
            title,
            Style::default()
                .fg(ACCENT_GREEN)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(BG_PRIMARY));

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(BG_HIGHLIGHT)
            .add_modifier(Modifier::BOLD),
    );

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn result_to_list_item<'a>(
    result: &SearchResultSummary,
    query: &str,
    pinned: &HashSet<String>,
    show_source_labels: bool,
) -> ListItem<'a> {
    let icon = command_icon(&result.command);
    let is_pinned = pinned.contains(&result.command);

    let score_pct = (result.score * 100.0) as u32;
    let score_color = match score_pct {
        80..=100 => ACCENT_GREEN,
        50..=79 => ACCENT_YELLOW,
        20..=49 => ACCENT_ORANGE,
        _ => ACCENT_RED,
    };

    let exit_span = match result.exit_code {
        Some(0) => Span::styled(" ✓ ", Style::default().fg(ACCENT_GREEN)),
        Some(c) => Span::styled(format!(" ✗{c} "), Style::default().fg(ACCENT_RED)),
        None => Span::styled(" ? ", Style::default().fg(FG_DIM)),
    };

    let pin_span = if is_pinned {
        Span::styled("📌", Style::default().fg(ACCENT_YELLOW))
    } else {
        Span::raw("  ")
    };

    let match_badge = match_kind_span(&result.match_kind);
    let source_badge = if show_source_labels {
        source_kind_span(&result.source)
    } else {
        Span::raw("")
    };
    let cmd_spans = highlight_command(&result.command, query);

    let mut line1 = vec![
        Span::styled(format!("{icon} "), Style::default().fg(FG_SECONDARY)),
        exit_span,
        pin_span,
        match_badge,
        source_badge,
        Span::styled(
            format!("{score_pct:>3}% "),
            Style::default().fg(score_color),
        ),
    ];
    line1.extend(cmd_spans);

    let ts = format_ts_compact(result.completed_at_ms);
    let dur = format_dur_compact(result.duration_ms);
    let runs = if result.run_count > 1 {
        format!("  ×{}", result.run_count)
    } else {
        String::new()
    };

    let line2 = Line::from(vec![
        Span::styled("         ", Style::default()),
        Span::styled(ts, Style::default().fg(FG_DIM)),
        Span::styled(format!("  {dur}"), Style::default().fg(FG_DIM)),
        Span::styled(runs, Style::default().fg(ACCENT_PURPLE)),
        Span::styled(
            format!("  {}", result.cwd),
            Style::default().fg(FG_SECONDARY),
        ),
    ]);

    ListItem::new(vec![Line::from(line1), line2])
}

// ─── Command icons ───────────────────────────────────────────────────────────

fn command_icon(command: &str) -> &'static str {
    let first_word = command.split_whitespace().next().unwrap_or("");
    let base = first_word.rsplit('/').next().unwrap_or(first_word);

    match base {
        "docker" | "docker-compose" | "podman" => "🐳",
        "git" | "gh" => "🌿",
        "cargo" | "rustc" | "rustup" => "📦",
        "npm" | "npx" | "yarn" | "pnpm" | "bun" | "node" | "deno" => "📦",
        "pip" | "pip3" | "python" | "python3" | "uv" | "poetry" | "conda" => "🐍",
        "go" => "🔵",
        "ls" | "find" | "tree" | "du" | "df" | "stat" | "file" | "cat" | "less" | "head"
        | "tail" | "wc" | "sort" | "uniq" | "cut" | "awk" | "sed" | "grep" | "rg" | "fd"
        | "bat" | "exa" | "eza" => "📁",
        "cd" | "pushd" | "popd" | "mkdir" | "rmdir" | "rm" | "cp" | "mv" | "ln" | "chmod"
        | "chown" | "touch" => "📁",
        "systemctl" | "journalctl" | "service" | "sudo" | "su" | "doas" => "⚙\u{fe0f}",
        "ssh" | "scp" | "rsync" | "curl" | "wget" | "httpie" => "🌐",
        "make" | "cmake" | "ninja" | "meson" | "gcc" | "g++" | "clang" | "cc" => "🔧",
        "vim" | "nvim" | "nano" | "emacs" | "code" | "vi" => "✏\u{fe0f}",
        "kubectl" | "k9s" | "helm" | "terraform" | "ansible" => "☸\u{fe0f}",
        _ => "▸",
    }
}

fn match_kind_span(kind: &ggnmem_db::MatchKind) -> Span<'static> {
    use ggnmem_db::MatchKind;
    match kind {
        MatchKind::Exact => Span::styled(
            "EXACT ",
            Style::default()
                .fg(ACCENT_GREEN)
                .add_modifier(Modifier::BOLD),
        ),
        MatchKind::Prefix => Span::styled("PRFX  ", Style::default().fg(ACCENT_CYAN)),
        MatchKind::Partial => Span::styled("PART  ", Style::default().fg(ACCENT_YELLOW)),
        MatchKind::Fuzzy => Span::styled(
            "FUZZY ",
            Style::default()
                .fg(ACCENT_ORANGE)
                .add_modifier(Modifier::ITALIC),
        ),
    }
}

fn source_kind_span(source: &SearchSource) -> Span<'static> {
    match source {
        SearchSource::Fts => Span::styled(
            "[FTS] ",
            Style::default()
                .fg(ACCENT_CYAN)
                .add_modifier(Modifier::BOLD),
        ),
        SearchSource::Semantic => Span::styled(
            "[SEM] ",
            Style::default()
                .fg(ACCENT_PURPLE)
                .add_modifier(Modifier::BOLD),
        ),
        SearchSource::Hybrid => Span::styled(
            "[HYB] ",
            Style::default()
                .fg(ACCENT_YELLOW)
                .add_modifier(Modifier::BOLD),
        ),
    }
}

fn highlight_command<'a>(command: &str, query: &str) -> Vec<Span<'a>> {
    if query.is_empty() {
        return vec![Span::styled(
            command.to_owned(),
            Style::default().fg(FG_PRIMARY),
        )];
    }

    let cmd_lower = command.to_lowercase();
    let query_lower = query.to_lowercase();
    let mut spans = Vec::new();
    let mut last_end = 0;
    let mut search_from = 0;

    while let Some(start) = cmd_lower[search_from..].find(&query_lower) {
        let abs_start = search_from + start;
        let abs_end = abs_start + query.len();

        if abs_start > last_end {
            spans.push(Span::styled(
                command[last_end..abs_start].to_owned(),
                Style::default().fg(FG_PRIMARY),
            ));
        }

        spans.push(Span::styled(
            command[abs_start..abs_end].to_owned(),
            Style::default()
                .fg(ACCENT_CYAN)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ));

        last_end = abs_end;
        search_from = abs_end;
    }

    if last_end < command.len() {
        spans.push(Span::styled(
            command[last_end..].to_owned(),
            Style::default().fg(FG_PRIMARY),
        ));
    }

    if spans.is_empty() {
        spans.push(Span::styled(
            command.to_owned(),
            Style::default().fg(FG_PRIMARY),
        ));
    }

    spans
}

// ─── Preview panel ───────────────────────────────────────────────────────────

fn draw_preview(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_ACTIVE))
        .title(Span::styled(
            " ⌕ Preview ",
            Style::default()
                .fg(ACCENT_PURPLE)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(BG_SECONDARY));

    if let Some(result) = app.selected_result() {
        let ts = format_ts_full(result.completed_at_ms);
        let dur = format_dur_compact(result.duration_ms);
        let exit_str = result
            .exit_code
            .map(|c| format!("{c}"))
            .unwrap_or_else(|| "—".to_owned());
        let score_pct = (result.score * 100.0) as u32;
        let match_kind = format!("{:?}", result.match_kind);
        let icon = command_icon(&result.command);
        let is_pinned = app.pinned.contains(&result.command);
        let is_internal = ggnmem_db::is_internal_command(&result.command);

        let mut lines = vec![
            Line::from(vec![
                Span::styled("Command   ", Style::default().fg(FG_DIM)),
                Span::styled(format!("{icon} "), Style::default().fg(FG_SECONDARY)),
                Span::styled(
                    result.command.clone(),
                    Style::default()
                        .fg(ACCENT_CYAN)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("CWD       ", Style::default().fg(FG_DIM)),
                Span::styled(result.cwd.clone(), Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(vec![
                Span::styled("Time      ", Style::default().fg(FG_DIM)),
                Span::styled(ts, Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(vec![
                Span::styled("Duration  ", Style::default().fg(FG_DIM)),
                Span::styled(dur, Style::default().fg(FG_PRIMARY)),
            ]),
            Line::from(vec![
                Span::styled("Exit Code ", Style::default().fg(FG_DIM)),
                Span::styled(
                    exit_str,
                    Style::default().fg(if result.exit_code == Some(0) {
                        ACCENT_GREEN
                    } else {
                        ACCENT_RED
                    }),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Match     ", Style::default().fg(FG_DIM)),
                Span::styled(match_kind, Style::default().fg(ACCENT_YELLOW)),
            ]),
            Line::from(vec![
                Span::styled("Score     ", Style::default().fg(FG_DIM)),
                Span::styled(format!("{score_pct}%"), Style::default().fg(ACCENT_GREEN)),
            ]),
            Line::from(vec![
                Span::styled("Runs      ", Style::default().fg(FG_DIM)),
                Span::styled(
                    format!("{}", result.run_count),
                    Style::default().fg(ACCENT_PURPLE),
                ),
            ]),
        ];

        // Badges.
        let mut badge_spans = vec![Span::styled("Flags     ", Style::default().fg(FG_DIM))];
        if is_pinned {
            badge_spans.push(Span::styled(
                "📌 pinned ",
                Style::default().fg(ACCENT_YELLOW),
            ));
        }
        if is_internal {
            badge_spans.push(Span::styled(
                "⚙ internal ",
                Style::default().fg(ACCENT_ORANGE),
            ));
        }
        if !is_pinned && !is_internal {
            badge_spans.push(Span::styled("—", Style::default().fg(FG_DIM)));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(badge_spans));

        let paragraph = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    } else {
        let paragraph = Paragraph::new(Span::styled(
            "  No result selected",
            Style::default().fg(FG_DIM),
        ))
        .block(block);
        f.render_widget(paragraph, area);
    }
}

// ─── Footer ──────────────────────────────────────────────────────────────────

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let (feedback_text, feedback_color) = if let Some((msg, success, _)) = &app.clipboard_feedback {
        let icon = if *success { "✓" } else { "✗" };
        (
            format!("  {icon} {msg}"),
            if *success { ACCENT_GREEN } else { ACCENT_RED },
        )
    } else {
        (String::new(), FG_DIM)
    };

    let status_spans = vec![
        Span::styled(
            format!("  {} ", app.status_msg),
            Style::default().fg(FG_SECONDARY),
        ),
        Span::styled(
            feedback_text,
            Style::default()
                .fg(feedback_color)
                .add_modifier(Modifier::BOLD),
        ),
    ];

    let help_spans = vec![
        Span::styled("^F", Style::default().fg(ACCENT_CYAN)),
        Span::styled(" fts ", Style::default().fg(FG_DIM)),
        Span::styled("^S", Style::default().fg(ACCENT_CYAN)),
        Span::styled(" sem ", Style::default().fg(FG_DIM)),
        Span::styled("^H", Style::default().fg(ACCENT_CYAN)),
        Span::styled(" hyb ", Style::default().fg(FG_DIM)),
        Span::styled("^L", Style::default().fg(ACCENT_CYAN)),
        Span::styled(" clear ", Style::default().fg(FG_DIM)),
        Span::styled("Enter", Style::default().fg(ACCENT_CYAN)),
        Span::styled(" ins ", Style::default().fg(FG_DIM)),
        Span::styled("C", Style::default().fg(ACCENT_CYAN)),
        Span::styled(" copy ", Style::default().fg(FG_DIM)),
        Span::styled("Tab", Style::default().fg(ACCENT_CYAN)),
        Span::styled(" preview ", Style::default().fg(FG_DIM)),
        Span::styled("Esc", Style::default().fg(ACCENT_PINK)),
        Span::styled(" quit ", Style::default().fg(FG_DIM)),
    ];

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let status_bar =
        Paragraph::new(Line::from(status_spans)).style(Style::default().bg(BG_SECONDARY));
    f.render_widget(status_bar, chunks[0]);

    let help_bar = Paragraph::new(Line::from(help_spans))
        .style(Style::default().bg(BG_SECONDARY))
        .alignment(Alignment::Right);
    f.render_widget(help_bar, chunks[1]);
}

// ─── IPC helpers ─────────────────────────────────────────────────────────────

async fn do_search(
    query: &str,
    cwd: Option<String>,
    search_mode: SearchMode,
) -> Result<(Vec<SearchResultSummary>, u64)> {
    let config = DaemonConfig::load().context("load config")?;
    let mut client = IpcClient::connect(&config.endpoint)
        .await
        .context("connect to daemon")?;

    let start = Instant::now();
    let request = if query.trim().is_empty() {
        DaemonRequest::query_recent(SEARCH_LIMIT)
    } else {
        DaemonRequest::search_commands_with_mode(query, SEARCH_LIMIT, cwd, false, search_mode)
    };

    let response: ggnmem_daemon::DaemonResponse =
        client.request(&request).await.context("daemon request")?;
    let client_latency_ms = start.elapsed().as_millis() as u64;

    match response.kind {
        DaemonResponseKind::SearchResults {
            results,
            latency_ms,
        } => {
            // Prefer server-reported latency; fall back to client-measured.
            let latency = latency_ms.unwrap_or(client_latency_ms);
            Ok((results, latency))
        }
        DaemonResponseKind::RecentCommands { commands } => {
            let results: Vec<SearchResultSummary> = commands
                .into_iter()
                .map(|c| SearchResultSummary {
                    command: c.command,
                    cwd: c.cwd,
                    exit_code: c.exit_code,
                    duration_ms: c.duration_ms,
                    completed_at_ms: c.completed_at_ms,
                    run_count: 1,
                    match_kind: ggnmem_db::MatchKind::Exact,
                    score: 1.0,
                    source: SearchSource::Fts,
                })
                .collect();
            Ok((results, client_latency_ms))
        }
        DaemonResponseKind::Error { code, message } => anyhow::bail!("{code}: {message}"),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn fetch_count() -> Result<u64> {
    let config = DaemonConfig::load()?;
    let mut client = IpcClient::connect(&config.endpoint).await?;
    let response: ggnmem_daemon::DaemonResponse =
        client.request(&DaemonRequest::count_commands()).await?;
    match response.kind {
        DaemonResponseKind::CommandCount { count } => Ok(count),
        _ => anyhow::bail!("unexpected response"),
    }
}

// ─── Shell action files ──────────────────────────────────────────────────────

fn action_file(name: &str) -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("$HOME is not set")?;
    Ok(PathBuf::from(home)
        .join(".local")
        .join("state")
        .join("ggnmem")
        .join(name))
}

fn write_action_file(name: &str, cmd: &str) -> Result<()> {
    let path = action_file(name)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(&path, cmd).with_context(|| format!("write {}", path.display()))
}

// ─── Clipboard ───────────────────────────────────────────────────────────────

/// Copy text to the OS clipboard. Returns true if a clipboard tool was found
/// and exited successfully.
fn copy_to_clipboard(text: &str) -> bool {
    use std::process::{Command, Stdio};

    // Tool list ordered by preference.
    // clip.exe works inside WSL to reach the Windows clipboard.
    let tools: &[(&str, &[&str])] = &[
        ("clip.exe", &[]),
        ("xclip", &["-selection", "clipboard"]),
        ("xsel", &["--clipboard", "--input"]),
        ("wl-copy", &[]),
    ];

    for (tool, args) in tools {
        if let Ok(mut child) = Command::new(tool)
            .args(*args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            let mut write_ok = false;
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                write_ok = stdin.write_all(text.as_bytes()).is_ok();
            }
            if let Ok(status) = child.wait() {
                if write_ok && status.success() {
                    return true;
                }
            }
        }
    }
    false
}

// ─── Formatting helpers ──────────────────────────────────────────────────────

fn format_ts_compact(millis: i64) -> String {
    let secs = (millis / 1000) as u64;
    let secs_in_day = secs % 86400;
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;
    let days = secs / 86400;
    let (y, m, d) = epoch_days_to_date(days);
    format!("{y:04}-{m:02}-{d:02} {hours:02}:{minutes:02}")
}

fn format_ts_full(millis: i64) -> String {
    let secs = (millis / 1000) as u64;
    let secs_in_day = secs % 86400;
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;
    let seconds = secs_in_day % 60;
    let days = secs / 86400;
    let (y, m, d) = epoch_days_to_date(days);
    format!("{y:04}-{m:02}-{d:02} {hours:02}:{minutes:02}:{seconds:02}")
}

fn epoch_days_to_date(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn format_dur_compact(millis: Option<i64>) -> String {
    match millis {
        Some(ms) if ms < 1000 => format!("{ms}ms"),
        Some(ms) if ms < 60_000 => format!("{:.1}s", ms as f64 / 1000.0),
        Some(ms) => format!("{:.1}m", ms as f64 / 60_000.0),
        None => "—".to_owned(),
    }
}
