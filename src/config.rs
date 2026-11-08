use crate::app::LANGUAGES;
use crate::theme::THEMES;
use std::path::PathBuf;

pub struct Config {
    pub theme_index: usize,
    pub lang_index: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme_index: 1,
            lang_index: 0,
        }
    }
}

fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("twiki").join("config"))
}

pub fn load() -> Config {
    let mut cfg = Config::default();
    let Some(text) = path().and_then(|p| std::fs::read_to_string(p).ok()) else {
        return cfg;
    };
    for line in text.lines() {
        match line.split_once('=') {
            Some(("theme", v)) => {
                if let Some(i) = THEMES.iter().position(|t| t.name == v.trim()) {
                    cfg.theme_index = i;
                }
            }
            Some(("lang", v)) => {
                if let Some(i) = LANGUAGES.iter().position(|l| l.code == v.trim()) {
                    cfg.lang_index = i;
                }
            }
            _ => {}
        }
    }
    cfg
}

pub fn save(cfg: &Config) {
    let Some(p) = path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let body = format!(
        "theme={}\nlang={}\n",
        THEMES[cfg.theme_index].name, LANGUAGES[cfg.lang_index].code
    );
    let _ = std::fs::write(p, body);
}
