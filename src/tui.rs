use ratatui::prelude::{Layout, Constraint, Direction};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::style::{Style, Color};
use crossterm::event::{self, Event, KeyCode};
use crate::core;
use tokio::runtime::Runtime;
use crate::utils::async_get_pkgbuild_cached;
use mlua::Lua;
use std::sync::{Arc, Mutex};

// Log pane state
struct LogPane {
    lines: Arc<Mutex<Vec<String>>>,
}

impl LogPane {
    fn new() -> Self {
        Self { lines: Arc::new(Mutex::new(Vec::new())) }
    }
    fn push(&self, line: &str) {
        let mut lines = self.lines.lock().unwrap();
        lines.push(line.to_string());
        if lines.len() > 1000 { lines.remove(0); }
    }
    fn get(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }
    /// Clear the log pane contents
    pub fn clear(&self) {
        let mut lines = self.lines.lock().unwrap();
        lines.clear();
    }
}

fn run_lua_hook(hook: &str, pkg: &str) {
    let config_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".config/ghostbrew/brew.lua");
    if let Ok(script) = std::fs::read_to_string(&config_path) {
        let lua = Lua::new();
        if let Ok(_) = lua.load(&script).exec() {
            let globals = lua.globals();
            if let Ok(func) = globals.get::<_, mlua::Function>(hook) {
                let _ = func.call::<_, ()>(pkg);
            }
        }
    }
}

pub fn run() {
    let rt = Runtime::new().unwrap();
    let mut terminal = setup_terminal();
    let mut query = prompt_for_query();
    let mut results = rt.block_on(async { core::unified_search(&query) });
    let mut selected = 0;
    let mut status = String::new();
    let config = crate::config::BrewConfig::load();
    let mut show_details = false;
    let mut selected_pkgs = vec![];
    let mut pkgb_preview = String::new();
    let mut show_help = false;
    let mut show_log = false;
    let log_pane = LogPane::new();
    let help_text = "[ghostbrew TUI]\n/ search | d details | space select | enter install | l log | h help | c clear log | q quit";

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default().direction(Direction::Vertical).margin(0).constraints([
                Constraint::Min(10), Constraint::Length(7)
            ]).split(size);
            let block = Block::default().title("ghostbrew unified search").borders(Borders::ALL);
            let items: Vec<ListItem> = results.iter().enumerate().map(|(i, r)| {
                let color = match r.source.label() {
                    "[AUR]" => Color::Yellow,
                    "[ChaoticAUR]" => Color::Magenta,
                    "[Flatpak]" => Color::Cyan,
                    _ => Color::White,
                };
                let style = if i == selected { Style::default().fg(Color::Black).bg(Color::White) } else { Style::default().fg(color) };
                let prefix = if selected_pkgs.contains(&i) { "[*] " } else { "    " };
                ListItem::new(format!("{}{} {} {} - {}", prefix, r.source.label(), r.name, r.version, r.description)).style(style)
            }).collect();
            let list = List::new(items).block(block).highlight_symbol("▶ ");
            f.render_widget(list, chunks[0]);
            let mut details = String::new();
            if show_details {
                if let Some(pkg) = results.get(selected) {
                    if pkg.source == core::Source::Aur {
                        details = pkgb_preview.clone();
                        details.push_str("\n[Deps]: ");
                        details.push_str(&format!("{:?}", crate::aur::get_deps(&pkg.name)));
                    } else {
                        details = format!("No PKGBUILD for {}", pkg.name);
                    }
                }
            }
            let log_lines = log_pane.get().join("\n");
            let status_p = Paragraph::new(
                if show_help { help_text.into() }
                else if show_log { log_lines }
                else { status.clone() + "\n" + &details }
            ).block(Block::default().borders(Borders::ALL).title(if show_log { "Log" } else { "Status/Details" }));
            f.render_widget(status_p, chunks[1]);
        }).unwrap();

        if event::poll(std::time::Duration::from_millis(200)).unwrap() {
            if let Event::Key(key) = event::read().unwrap() {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('/') => {
                        query = prompt_for_query();
                        results = rt.block_on(async { core::unified_search(&query) });
                        selected = 0;
                    }
                    KeyCode::Down => {
                        if selected + 1 < results.len() { selected += 1; }
                    }
                    KeyCode::Up => {
                        if selected > 0 { selected -= 1; }
                    }
                    KeyCode::Char(' ') => {
                        if selected_pkgs.contains(&selected) {
                            selected_pkgs.retain(|&x| x != selected);
                        } else {
                            selected_pkgs.push(selected);
                        }
                    }
                    KeyCode::Char('d') => {
                        show_details = !show_details;
                        if show_details {
                            if let Some(pkg) = results.get(selected) {
                                if pkg.source == core::Source::Aur {
                                    pkgb_preview = rt.block_on(async_get_pkgbuild_cached(&pkg.name));
                                }
                            }
                        }
                    }
                    KeyCode::Char('h') => {
                        show_help = !show_help;
                    }
                    KeyCode::Char('l') => {
                        show_log = !show_log;
                    }
                    KeyCode::Char('c') => {
                        log_pane.clear();
                    }
                    KeyCode::Enter => {
                        let to_install: Vec<String> = if selected_pkgs.is_empty() {
                            results.get(selected).map(|p| p.name.clone()).into_iter().collect()
                        } else {
                            selected_pkgs.iter().filter_map(|&i| results.get(i).map(|p| p.name.clone())).collect()
                        };
                        for pkg in &to_install {
                            run_lua_hook("pre_install", pkg);
                            status = format!("Installing {}...", pkg);
                            log_pane.push(&status);
                            core::install_with_priority(pkg, &config);
                            run_lua_hook("post_install", pkg);
                            let done = format!("[ghostbrew] {} install complete.", pkg);
                            log_pane.push(&done);
                            status = done;
                        }
                        selected_pkgs.clear();
                    }
                    _ => {}
                }
            }
        }
    }
    restore_terminal();
}

fn prompt_for_query() -> String {
    use std::io::{self, Write};
    print!("Search: ");
    io::stdout().flush().unwrap();
    let mut query = String::new();
    io::stdin().read_line(&mut query).unwrap();
    query.trim().to_string()
}

fn setup_terminal() -> ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>> {
    use crossterm::terminal::{enable_raw_mode, EnterAlternateScreen};
    use std::io::stdout;
    use crossterm::execute;
    enable_raw_mode().unwrap();
    execute!(stdout(), EnterAlternateScreen).unwrap();
    let backend = ratatui::backend::CrosstermBackend::new(stdout());
    ratatui::Terminal::new(backend).unwrap()
}

fn restore_terminal() {
    use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
    use std::io::stdout;
    use crossterm::execute;
    disable_raw_mode().unwrap();
    execute!(stdout(), LeaveAlternateScreen).unwrap();
}
