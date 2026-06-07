mod app;
mod theme;
mod ui;
mod wikipedia;

use app::App;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, time::Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let result = run(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }
    Ok(())
}

async fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> anyhow::Result<()> {
    let mut saved_theme: usize = app.theme_index;

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.mode {
                    app::Mode::Help => match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        _ => {
                            app.close_help();
                        }
                    },
                    app::Mode::LangPicker => match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.close_lang_picker();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.lang_picker_up();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.lang_picker_down();
                        }
                        KeyCode::Enter => {
                            app.confirm_lang();
                        }
                        _ => {}
                    },
                    app::Mode::ThemePicker => match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.cancel_theme_picker(saved_theme);
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.theme_picker_up();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.theme_picker_down();
                        }
                        KeyCode::Enter => {
                            saved_theme = app.theme_cursor;
                            app.confirm_theme();
                        }
                        _ => {}
                    },
                    app::Mode::Search => match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        KeyCode::Esc => {
                            if app.article.is_some() {
                                app.mode = app::Mode::Reading;
                            }
                        }
                        KeyCode::Enter => {
                            let query = app.search_input.clone();
                            if !query.is_empty() {
                                let lang = app.current_lang().to_string();
                                app.status = Some("Searching...".to_string());
                                match wikipedia::fetch_article(&query, &lang).await {
                                    Ok(article) => {
                                        app.set_article(article);
                                        app.mode = app::Mode::Reading;
                                        app.status = None;
                                    }
                                    Err(e) => {
                                        app.status = Some(format!("Error: {e}"));
                                    }
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            app.search_input.pop();
                        }
                        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.open_lang_picker();
                        }
                        KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            saved_theme = app.theme_index;
                            app.open_theme_picker();
                        }
                        KeyCode::Char('?') => {
                            app.open_help();
                        }
                        KeyCode::Char(c) => {
                            app.search_input.push(c);
                        }
                        _ => {}
                    },
                    app::Mode::Reading => match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            return Ok(());
                        }
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        KeyCode::Char('/') | KeyCode::Char('s') => {
                            app.mode = app::Mode::Search;
                        }
                        KeyCode::Char('L') => {
                            app.open_lang_picker();
                        }
                        KeyCode::Char('T') => {
                            saved_theme = app.theme_index;
                            app.open_theme_picker();
                        }
                        KeyCode::Char('?') => {
                            app.open_help();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.scroll_down(1);
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.scroll_up(1);
                        }
                        KeyCode::PageDown => {
                            let n = app.viewport_height.saturating_sub(1).max(1);
                            app.scroll_down(n);
                        }
                        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            let n = app.viewport_height.saturating_sub(1).max(1);
                            app.scroll_down(n);
                        }
                        KeyCode::PageUp => {
                            let n = app.viewport_height.saturating_sub(1).max(1);
                            app.scroll_up(n);
                        }
                        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            let n = app.viewport_height.saturating_sub(1).max(1);
                            app.scroll_up(n);
                        }
                        KeyCode::Char(' ') => {
                            let n = app.viewport_height.saturating_sub(1).max(1);
                            app.scroll_down(n);
                        }
                        KeyCode::Home | KeyCode::Char('g') => {
                            app.scroll = 0;
                        }
                        KeyCode::End | KeyCode::Char('G') => {
                            app.scroll_to_end();
                        }
                        KeyCode::Tab => {
                            app.next_link();
                        }
                        KeyCode::BackTab => {
                            app.prev_link();
                        }
                        KeyCode::Enter => {
                            if let Some(title) = app.selected_link_title() {
                                let title = title.to_string();
                                let lang = app.current_lang().to_string();
                                app.status = Some(format!("Loading \"{}\"...", title));
                                match wikipedia::fetch_article(&title, &lang).await {
                                    Ok(article) => {
                                        app.set_article(article);
                                        app.status = None;
                                    }
                                    Err(e) => {
                                        app.status = Some(format!("Error: {e}"));
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}
