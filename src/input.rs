use crate::app::{Action, App, Mode};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

impl App {
    pub fn handle_mouse(&mut self, ev: MouseEvent) {
        if self.mode != Mode::Reading {
            return;
        }
        match ev.kind {
            MouseEventKind::ScrollDown => self.scroll_down(3),
            MouseEventKind::ScrollUp => self.scroll_up(3),
            _ => {}
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            return Some(Action::Quit);
        }
        if !self.loading {
            self.status = None;
        }
        if key.code == KeyCode::F(1) && self.mode != Mode::Help {
            self.open_overlay(Mode::Help);
            return None;
        }

        match self.mode {
            Mode::Help => self.close_overlay(),
            Mode::LangPicker => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.close_overlay(),
                KeyCode::Up | KeyCode::Char('k') => self.lang_picker_move(-1),
                KeyCode::Down | KeyCode::Char('j') => self.lang_picker_move(1),
                KeyCode::Enter => self.confirm_lang(),
                _ => {}
            },
            Mode::ThemePicker => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => self.cancel_theme_picker(),
                KeyCode::Up | KeyCode::Char('k') => self.theme_picker_move(-1),
                KeyCode::Down | KeyCode::Char('j') => self.theme_picker_move(1),
                KeyCode::Enter => self.confirm_theme(),
                _ => {}
            },
            Mode::Toc => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('o') => self.close_overlay(),
                KeyCode::Up | KeyCode::Char('k') => self.toc_move(-1),
                KeyCode::Down | KeyCode::Char('j') => self.toc_move(1),
                KeyCode::Enter => self.toc_confirm(),
                _ => {}
            },
            Mode::Search => return self.handle_search_key(key),
            Mode::Find => match key.code {
                KeyCode::Esc => self.mode = Mode::Reading,
                KeyCode::Enter => {
                    self.mode = Mode::Reading;
                    self.run_find();
                }
                KeyCode::Backspace => {
                    self.find_input.pop();
                }
                KeyCode::Char(c) if !ctrl && !key.modifiers.contains(KeyModifiers::ALT) => {
                    self.find_input.push(c);
                }
                _ => {}
            },
            Mode::Reading => return self.handle_reading_key(key),
        }
        None
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Option<Action> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let len = self.search_input.chars().count();
        match key.code {
            KeyCode::Esc => {
                if self.article.is_some() {
                    self.mode = Mode::Reading;
                }
            }
            KeyCode::Enter => return self.begin_fetch(self.search_input.clone()),
            KeyCode::Backspace => {
                if self.search_cursor > 0 {
                    self.search_cursor -= 1;
                    let i = self.search_byte_index();
                    self.search_input.remove(i);
                }
            }
            KeyCode::Delete => {
                if self.search_cursor < len {
                    let i = self.search_byte_index();
                    self.search_input.remove(i);
                }
            }
            KeyCode::Left => self.search_cursor = self.search_cursor.saturating_sub(1),
            KeyCode::Right => self.search_cursor = (self.search_cursor + 1).min(len),
            KeyCode::Home => self.search_cursor = 0,
            KeyCode::End => self.search_cursor = len,
            KeyCode::Char('a') if ctrl => self.search_cursor = 0,
            KeyCode::Char('e') if ctrl => self.search_cursor = len,
            KeyCode::Char('u') if ctrl => {
                let i = self.search_byte_index();
                self.search_input.drain(..i);
                self.search_cursor = 0;
            }
            KeyCode::Char('l') if ctrl => self.open_overlay(Mode::LangPicker),
            KeyCode::Char('t') if ctrl => self.open_overlay(Mode::ThemePicker),
            KeyCode::Char(c) if !ctrl && !key.modifiers.contains(KeyModifiers::ALT) => {
                let i = self.search_byte_index();
                self.search_input.insert(i, c);
                self.search_cursor += 1;
            }
            _ => {}
        }
        None
    }

    fn handle_reading_key(&mut self, key: KeyEvent) -> Option<Action> {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let page = self.viewport_height.saturating_sub(1).max(1);
        let half = (self.viewport_height / 2).max(1);
        match key.code {
            KeyCode::Char('q') => return Some(Action::Quit),
            KeyCode::Char('/') | KeyCode::Char('s') => self.mode = Mode::Search,
            KeyCode::Char('f') => {
                self.find_input.clear();
                self.mode = Mode::Find;
            }
            KeyCode::Char('n') => self.step_match(1),
            KeyCode::Char('N') => self.step_match(-1),
            KeyCode::Char('o') => {
                if self
                    .article
                    .as_ref()
                    .is_some_and(|a| !a.headings.is_empty())
                {
                    self.open_overlay(Mode::Toc);
                }
            }
            KeyCode::Char('L') => self.open_overlay(Mode::LangPicker),
            KeyCode::Char('T') => self.open_overlay(Mode::ThemePicker),
            KeyCode::Char('?') => self.open_overlay(Mode::Help),
            KeyCode::Backspace | KeyCode::Char('h') => self.go_back(),
            KeyCode::Down | KeyCode::Char('j') => self.scroll_down(1),
            KeyCode::Up | KeyCode::Char('k') => self.scroll_up(1),
            KeyCode::PageDown | KeyCode::Char(' ') => self.scroll_down(page),
            KeyCode::PageUp => self.scroll_up(page),
            KeyCode::Char('d') if ctrl => self.scroll_down(half),
            KeyCode::Char('u') if ctrl => self.scroll_up(half),
            KeyCode::Home | KeyCode::Char('g') => self.scroll = 0,
            KeyCode::End | KeyCode::Char('G') => self.scroll_to_end(),
            KeyCode::Tab => self.next_link(),
            KeyCode::BackTab => self.prev_link(),
            KeyCode::Enter => {
                if let Some(title) = self.selected_link_title().map(str::to_string) {
                    return self.begin_fetch(title);
                }
            }
            _ => {}
        }
        None
    }
}
