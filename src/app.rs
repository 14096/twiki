use crate::theme::{THEMES, Theme};
use crate::wikipedia::Article;

#[derive(Debug, PartialEq, Eq)]
pub enum Mode {
    Search,
    Reading,
    LangPicker,
    ThemePicker,
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

pub struct App {
    pub mode: Mode,
    pub prev_mode: Mode,
    pub search_input: String,
    pub article: Option<Article>,
    pub scroll: usize,
    pub total_lines: usize,
    pub viewport_height: usize,
    pub selected_link: Option<usize>,
    pub status: Option<String>,
    pub lang_index: usize,
    pub lang_cursor: usize,
    pub theme_index: usize,
    pub theme_cursor: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: Mode::Search,
            prev_mode: Mode::Search,
            search_input: String::new(),
            article: None,
            scroll: 0,
            total_lines: 0,
            viewport_height: 0,
            selected_link: None,
            status: None,
            lang_index: 0,
            lang_cursor: 0,
            theme_index: 1,
            theme_cursor: 0,
        }
    }

    pub fn current_lang(&self) -> &'static str {
        LANGUAGES[self.lang_index].code
    }

    pub fn theme(&self) -> &'static Theme {
        &THEMES[self.theme_index]
    }

    pub fn open_lang_picker(&mut self) {
        if self.mode == Mode::LangPicker {
            return;
        }
        self.prev_mode = match self.mode {
            Mode::Search => Mode::Search,
            _ => Mode::Reading,
        };
        self.lang_cursor = self.lang_index;
        self.mode = Mode::LangPicker;
    }

    pub fn confirm_lang(&mut self) {
        self.lang_index = self.lang_cursor;
        self.close_picker();
    }

    pub fn open_help(&mut self) {
        if self.mode == Mode::Help {
            return;
        }
        self.prev_mode = match self.mode {
            Mode::Search => Mode::Search,
            _ => Mode::Reading,
        };
        self.mode = Mode::Help;
    }

    pub fn close_help(&mut self) {
        self.close_picker();
    }

    pub fn open_theme_picker(&mut self) {
        if self.mode == Mode::ThemePicker {
            return;
        }
        self.prev_mode = match self.mode {
            Mode::Search => Mode::Search,
            _ => Mode::Reading,
        };
        self.theme_cursor = self.theme_index;
        self.mode = Mode::ThemePicker;
    }

    pub fn confirm_theme(&mut self) {
        self.theme_index = self.theme_cursor;
        self.close_picker();
    }

    pub fn preview_theme(&mut self) {
        self.theme_index = self.theme_cursor;
    }

    pub fn theme_picker_up(&mut self) {
        if self.theme_cursor > 0 {
            self.theme_cursor -= 1;
        } else {
            self.theme_cursor = THEMES.len() - 1;
        }
        self.preview_theme();
    }

    pub fn theme_picker_down(&mut self) {
        self.theme_cursor = (self.theme_cursor + 1) % THEMES.len();
        self.preview_theme();
    }

    pub fn close_picker(&mut self) {
        self.mode = match self.prev_mode {
            Mode::Search => Mode::Search,
            _ => {
                if self.article.is_some() {
                    Mode::Reading
                } else {
                    Mode::Search
                }
            }
        };
    }

    pub fn cancel_theme_picker(&mut self, saved_index: usize) {
        self.theme_index = saved_index;
        self.close_picker();
    }

    pub fn lang_picker_up(&mut self) {
        if self.lang_cursor > 0 {
            self.lang_cursor -= 1;
        } else {
            self.lang_cursor = LANGUAGES.len() - 1;
        }
    }

    pub fn lang_picker_down(&mut self) {
        self.lang_cursor = (self.lang_cursor + 1) % LANGUAGES.len();
    }

    pub fn close_lang_picker(&mut self) {
        self.close_picker();
    }

    pub fn set_article(&mut self, article: Article) {
        self.article = Some(article);
        self.scroll = 0;
        self.selected_link = None;
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max = self.total_lines.saturating_sub(self.viewport_height);
        self.scroll = (self.scroll + n).min(max);
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll = self.scroll.saturating_sub(n);
    }

    pub fn scroll_to_end(&mut self) {
        let max = self.total_lines.saturating_sub(self.viewport_height);
        self.scroll = max;
    }

    pub fn next_link(&mut self) {
        if let Some(article) = &self.article {
            let count = article.links.len();
            if count == 0 {
                return;
            }
            self.selected_link = Some(match self.selected_link {
                None => 0,
                Some(i) => (i + 1) % count,
            });
            self.scroll_to_link();
        }
    }

    pub fn prev_link(&mut self) {
        if let Some(article) = &self.article {
            let count = article.links.len();
            if count == 0 {
                return;
            }
            self.selected_link = Some(match self.selected_link {
                None => count.saturating_sub(1),
                Some(0) => count.saturating_sub(1),
                Some(i) => i - 1,
            });
            self.scroll_to_link();
        }
    }

    fn scroll_to_link(&mut self) {
        if let (Some(article), Some(idx)) = (&self.article, self.selected_link) {
            if let Some(link) = article.links.get(idx) {
                let line = link.line;
                let half = self.viewport_height / 2;
                self.scroll = line.saturating_sub(half);
                let max = self.total_lines.saturating_sub(self.viewport_height);
                self.scroll = self.scroll.min(max);
            }
        }
    }

    pub fn selected_link_title(&self) -> Option<&str> {
        let article = self.article.as_ref()?;
        let idx = self.selected_link?;
        article.links.get(idx).map(|l| l.title.as_str())
    }
}
