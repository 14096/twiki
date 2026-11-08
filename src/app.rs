use crate::config::{self, Config};
use crate::theme::{THEMES, Theme};
use crate::wikipedia::Article;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Search,
    Reading,
    Find,
    LangPicker,
    ThemePicker,
    Toc,
    Help,
}

#[derive(Debug, Clone)]
pub struct Language {
    pub code: &'static str,
    pub name: &'static str,
}

pub const LANGUAGES: &[Language] = &[
    Language {
        code: "en",
        name: "English",
    },
    Language {
        code: "es",
        name: "Español",
    },
    Language {
        code: "fr",
        name: "Français",
    },
    Language {
        code: "de",
        name: "Deutsch",
    },
    Language {
        code: "it",
        name: "Italiano",
    },
    Language {
        code: "pt",
        name: "Português",
    },
    Language {
        code: "ru",
        name: "Русский",
    },
    Language {
        code: "ja",
        name: "日本語",
    },
    Language {
        code: "zh",
        name: "中文",
    },
    Language {
        code: "ar",
        name: "العربية",
    },
    Language {
        code: "pl",
        name: "Polski",
    },
    Language {
        code: "nl",
        name: "Nederlands",
    },
    Language {
        code: "sv",
        name: "Svenska",
    },
    Language {
        code: "uk",
        name: "Українська",
    },
    Language {
        code: "ko",
        name: "한국어",
    },
    Language {
        code: "tr",
        name: "Türkçe",
    },
    Language {
        code: "vi",
        name: "Tiếng Việt",
    },
    Language {
        code: "fa",
        name: "فارسی",
    },
    Language {
        code: "he",
        name: "עברית",
    },
    Language {
        code: "id",
        name: "Bahasa Indonesia",
    },
];

pub enum Action {
    Quit,
    Fetch { query: String, lang: &'static str },
}

pub struct App {
    pub mode: Mode,
    prev_mode: Mode,
    pub search_input: String,
    /// Cursor position in `search_input`, counted in chars.
    pub search_cursor: usize,
    pub find_input: String,
    pub find_matches: Vec<usize>,
    pub find_pos: usize,
    pub article: Option<Article>,
    history: Vec<(Article, usize)>,
    pub scroll: usize,
    pub total_lines: usize,
    pub viewport_height: usize,
    /// Visual row at which each logical article line starts, for the cached width.
    pub line_offsets: Vec<usize>,
    pub layout_width: u16,
    pub layout_dirty: bool,
    pub selected_link: Option<usize>,
    pub status: Option<String>,
    pub loading: bool,
    pub lang_index: usize,
    pub lang_cursor: usize,
    pub theme_index: usize,
    pub theme_cursor: usize,
    pub theme_saved: usize,
    pub toc_cursor: usize,
}

impl App {
    pub fn new(cfg: Config) -> Self {
        Self {
            mode: Mode::Search,
            prev_mode: Mode::Search,
            search_input: String::new(),
            search_cursor: 0,
            find_input: String::new(),
            find_matches: Vec::new(),
            find_pos: 0,
            article: None,
            history: Vec::new(),
            scroll: 0,
            total_lines: 0,
            viewport_height: 0,
            line_offsets: Vec::new(),
            layout_width: 0,
            layout_dirty: true,
            selected_link: None,
            status: None,
            loading: false,
            lang_index: cfg.lang_index,
            lang_cursor: 0,
            theme_index: cfg.theme_index,
            theme_cursor: 0,
            theme_saved: cfg.theme_index,
            toc_cursor: 0,
        }
    }

    pub fn current_lang(&self) -> &'static str {
        LANGUAGES[self.lang_index].code
    }

    pub fn theme(&self) -> &'static Theme {
        &THEMES[self.theme_index]
    }

    fn save_config(&self) {
        config::save(&Config {
            theme_index: self.theme_index,
            lang_index: self.lang_index,
        });
    }

    pub fn open_overlay(&mut self, mode: Mode) {
        if self.mode == mode {
            return;
        }
        if matches!(self.mode, Mode::Search | Mode::Reading | Mode::Find) {
            self.prev_mode = self.mode;
        }
        match mode {
            Mode::LangPicker => self.lang_cursor = self.lang_index,
            Mode::ThemePicker => {
                self.theme_cursor = self.theme_index;
                self.theme_saved = self.theme_index;
            }
            Mode::Toc => self.toc_cursor = 0,
            _ => {}
        }
        self.mode = mode;
    }

    pub fn close_overlay(&mut self) {
        self.mode = if self.prev_mode == Mode::Reading && self.article.is_none() {
            Mode::Search
        } else {
            self.prev_mode
        };
    }

    pub fn confirm_lang(&mut self) {
        self.lang_index = self.lang_cursor;
        self.save_config();
        self.close_overlay();
    }

    pub fn confirm_theme(&mut self) {
        self.theme_index = self.theme_cursor;
        self.theme_saved = self.theme_index;
        self.save_config();
        self.close_overlay();
    }

    pub fn cancel_theme_picker(&mut self) {
        self.theme_index = self.theme_saved;
        self.close_overlay();
    }

    pub fn theme_picker_move(&mut self, delta: isize) {
        self.theme_cursor = wrap_step(self.theme_cursor, delta, THEMES.len());
        self.theme_index = self.theme_cursor;
    }

    pub fn lang_picker_move(&mut self, delta: isize) {
        self.lang_cursor = wrap_step(self.lang_cursor, delta, LANGUAGES.len());
    }

    pub fn toc_move(&mut self, delta: isize) {
        let n = self.article.as_ref().map_or(0, |a| a.headings.len());
        if n > 0 {
            self.toc_cursor = wrap_step(self.toc_cursor, delta, n);
        }
    }

    pub fn toc_confirm(&mut self) {
        let line = self
            .article
            .as_ref()
            .and_then(|a| a.headings.get(self.toc_cursor))
            .map(|h| h.line);
        self.close_overlay();
        if let Some(line) = line {
            self.scroll_to_line(line, false);
        }
    }

    /// Replaces the displayed article. When `keep_history` is set the previous one can be returned to.
    pub fn set_article(&mut self, article: Article, keep_history: bool) {
        if keep_history && let Some(prev) = self.article.take() {
            self.history.push((prev, self.scroll));
        }
        self.article = Some(article);
        self.scroll = 0;
        self.selected_link = None;
        self.find_matches.clear();
        self.layout_dirty = true;
    }

    pub fn go_back(&mut self) {
        if let Some((article, scroll)) = self.history.pop() {
            self.article = Some(article);
            self.selected_link = None;
            self.find_matches.clear();
            self.layout_dirty = true;
            self.scroll = scroll;
            self.status = None;
        } else {
            self.status = Some("No previous article".to_string());
        }
    }

    pub fn finish_fetch(&mut self, result: Result<Article, String>) {
        self.loading = false;
        match result {
            Ok(article) => {
                self.set_article(article, true);
                self.mode = Mode::Reading;
                self.status = None;
            }
            Err(e) => self.status = Some(format!("Error: {e}")),
        }
    }

    fn max_scroll(&self) -> usize {
        self.total_lines.saturating_sub(self.viewport_height)
    }

    pub fn clamp_scroll(&mut self) {
        self.scroll = self.scroll.min(self.max_scroll());
    }

    pub fn scroll_down(&mut self, n: usize) {
        self.scroll = (self.scroll + n).min(self.max_scroll());
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub fn scroll_to_end(&mut self) {
        self.scroll = self.max_scroll();
    }

    fn row_of(&self, line: usize) -> usize {
        self.line_offsets.get(line).copied().unwrap_or(0)
    }

    fn scroll_to_line(&mut self, line: usize, center: bool) {
        let row = self.row_of(line);
        let offset = if center { self.viewport_height / 2 } else { 0 };
        self.scroll = row.saturating_sub(offset).min(self.max_scroll());
    }

    pub fn next_link(&mut self) {
        self.step_link(1);
    }

    pub fn prev_link(&mut self) {
        self.step_link(-1);
    }

    fn step_link(&mut self, delta: isize) {
        let Some(article) = &self.article else { return };
        let count = article.links.len();
        if count == 0 {
            return;
        }
        let next = match self.selected_link {
            None if delta > 0 => 0,
            None => count - 1,
            Some(i) => wrap_step(i, delta, count),
        };
        self.selected_link = Some(next);
        self.scroll_to_link();
    }

    /// Scrolls to the occurrence of the selected link nearest to the viewport.
    fn scroll_to_link(&mut self) {
        let Some(lines) = self
            .article
            .as_ref()
            .zip(self.selected_link)
            .and_then(|(a, i)| a.links.get(i))
            .map(|l| l.lines.clone())
        else {
            return;
        };
        let visible = self.scroll..self.scroll + self.viewport_height;
        if lines.iter().any(|&l| visible.contains(&self.row_of(l))) {
            return;
        }
        let target = lines
            .iter()
            .copied()
            .find(|&l| self.row_of(l) >= self.scroll)
            .or_else(|| lines.first().copied());
        if let Some(line) = target {
            self.scroll_to_line(line, true);
        }
    }

    pub fn selected_link_title(&self) -> Option<&str> {
        let article = self.article.as_ref()?;
        let idx = self.selected_link?;
        article.links.get(idx).map(|l| l.title.as_str())
    }

    pub fn run_find(&mut self) {
        let needle = self.find_input.to_lowercase();
        self.find_matches.clear();
        self.find_pos = 0;
        if let (Some(article), false) = (&self.article, needle.is_empty()) {
            self.find_matches = article
                .lines
                .iter()
                .enumerate()
                .filter(|(_, spans)| {
                    let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                    text.to_lowercase().contains(&needle)
                })
                .map(|(i, _)| i)
                .collect();
        }
        if self.find_matches.is_empty() {
            self.status = Some(format!("Error: no match for \"{}\"", self.find_input));
        } else {
            self.jump_to_match();
        }
    }

    pub fn step_match(&mut self, delta: isize) {
        if self.find_matches.is_empty() {
            return;
        }
        self.find_pos = wrap_step(self.find_pos, delta, self.find_matches.len());
        self.jump_to_match();
    }

    fn jump_to_match(&mut self) {
        let line = self.find_matches[self.find_pos];
        self.scroll_to_line(line, true);
        self.status = Some(format!(
            "Match {}/{} for \"{}\"",
            self.find_pos + 1,
            self.find_matches.len(),
            self.find_input
        ));
    }

    pub fn search_byte_index(&self) -> usize {
        self.search_input
            .char_indices()
            .nth(self.search_cursor)
            .map_or(self.search_input.len(), |(i, _)| i)
    }

    pub fn begin_fetch(&mut self, query: String) -> Option<Action> {
        if self.loading || query.trim().is_empty() {
            return None;
        }
        self.loading = true;
        self.status = Some(format!("Loading \"{query}\"..."));
        Some(Action::Fetch {
            query,
            lang: self.current_lang(),
        })
    }
}

fn wrap_step(i: usize, delta: isize, len: usize) -> usize {
    (i as isize + delta).rem_euclid(len as isize) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_step_wraps_both_ways() {
        assert_eq!(wrap_step(0, -1, 5), 4);
        assert_eq!(wrap_step(4, 1, 5), 0);
        assert_eq!(wrap_step(2, 1, 5), 3);
    }

    #[test]
    fn scroll_is_clamped() {
        let mut app = App::new(Config::default());
        app.total_lines = 50;
        app.viewport_height = 10;
        app.scroll_down(100);
        assert_eq!(app.scroll, 40);
        app.scroll_up(100);
        assert_eq!(app.scroll, 0);
    }
}
