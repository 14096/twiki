use crate::app::{App, LANGUAGES, Mode};
use crate::theme::{THEMES, Theme};
use crate::wikipedia::{Span as WikiSpan, SpanKind};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::Stylize,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Padding, Paragraph,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
};
use unicode_width::UnicodeWidthStr;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    let t = app.theme();

    f.render_widget(Paragraph::new("").style(Style::default().bg(t.bg)), area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    draw_search(f, app, t, chunks[0]);
    draw_article(f, app, t, chunks[1]);
    draw_statusbar(f, app, t, chunks[2]);

    match app.mode {
        Mode::LangPicker => draw_lang_picker(f, app, t, area),
        Mode::ThemePicker => draw_theme_picker(f, app, t, area),
        Mode::Toc => draw_toc(f, app, t, area),
        Mode::Help => draw_help(f, t, area),
        _ => {}
    }
}

fn s(fg: Color) -> Style {
    Style::default().fg(fg)
}
fn sb(fg: Color) -> Style {
    Style::default().fg(fg).add_modifier(Modifier::BOLD)
}

fn centered_popup(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(w) / 2,
        y: area.y + area.height.saturating_sub(h) / 2,
        width: w,
        height: h,
    }
}

fn render_popup_bg(f: &mut Frame, popup: Rect) {
    let shadow = Rect {
        x: popup.x + 1,
        y: popup.y + 1,
        width: popup.width,
        height: popup.height,
    };
    f.render_widget(
        Paragraph::new("").style(Style::default().bg(Color::Rgb(5, 5, 10))),
        shadow,
    );
    f.render_widget(Clear, popup);
}

fn draw_search(f: &mut Frame, app: &App, t: &Theme, area: Rect) {
    let focused = app.mode == Mode::Search;
    let (border_col, title_col) = if focused {
        (t.border_focused, t.accent)
    } else {
        (t.border, t.dim)
    };

    let lang = &LANGUAGES[app.lang_index];
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(s(border_col))
        .bg(t.surface)
        .title(Line::from(Span::styled(
            " Search Wikipedia ",
            sb(title_col),
        )))
        .title_alignment(Alignment::Left)
        .title_bottom(
            Line::from(Span::styled(
                format!(" {} ", lang.name),
                Style::default()
                    .fg(t.key_fg)
                    .bg(t.accent)
                    .add_modifier(Modifier::BOLD),
            ))
            .alignment(Alignment::Right),
        )
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text_col = if focused { t.text } else { t.dim };
    f.render_widget(
        Paragraph::new(Span::styled(app.search_input.as_str(), s(text_col))),
        inner,
    );
    if focused {
        let before: String = app.search_input.chars().take(app.search_cursor).collect();
        let x = inner.x + before.width() as u16;
        f.set_cursor_position((x.min(inner.right().saturating_sub(1)), inner.y));
    }
}

fn draw_article(f: &mut Frame, app: &mut App, t: &Theme, area: Rect) {
    let (border_col, title_style) = if app.mode == Mode::Reading {
        (t.border_focused, sb(t.title))
    } else {
        (t.border, s(t.dim))
    };

    let title_text = app
        .article
        .as_ref()
        .map(|a| format!("  {}  ", a.title))
        .unwrap_or_else(|| "  twiki  ".to_string());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(s(border_col))
        .bg(t.surface)
        .title(Span::styled(title_text, title_style))
        .title_alignment(Alignment::Center)
        .padding(Padding::horizontal(2));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let text_area = Rect {
        width: inner.width.saturating_sub(1),
        ..inner
    };
    let scrollbar_area = Rect {
        x: inner.x + inner.width.saturating_sub(1),
        width: 1,
        ..inner
    };

    app.viewport_height = text_area.height as usize;

    if app.article.is_none() {
        let welcome = Text::from(vec![
            Line::from(""),
            Line::from(Span::styled("twiki", sb(t.accent))),
            Line::from(""),
            Line::from(Span::styled("A terminal Wikipedia reader", s(t.dim))),
            Line::from(""),
            Line::from(vec![
                Span::styled(" /  ", sb(t.key_bg)),
                Span::styled("search   ", s(t.dim)),
                Span::styled(" L  ", sb(t.key_bg)),
                Span::styled("language   ", s(t.dim)),
                Span::styled(" T  ", sb(t.key_bg)),
                Span::styled("theme   ", s(t.dim)),
                Span::styled(" q  ", sb(t.key_bg)),
                Span::styled("quit", s(t.dim)),
            ]),
        ]);
        f.render_widget(
            Paragraph::new(welcome).alignment(Alignment::Center),
            text_area,
        );
        return;
    }

    let article = app.article.as_ref().unwrap();
    let selected_link = app.selected_link;
    let col_width = text_area.width.max(1);

    if app.layout_dirty || app.layout_width != col_width {
        let mut offsets = Vec::with_capacity(article.lines.len());
        let mut total = 0usize;
        for wiki_line in &article.lines {
            offsets.push(total);
            let line = Line::from(
                wiki_line
                    .iter()
                    .map(|ws| Span::raw(ws.text.as_str()))
                    .collect::<Vec<_>>(),
            );
            total += Paragraph::new(line)
                .wrap(Wrap { trim: false })
                .line_count(col_width)
                .max(1);
        }
        app.line_offsets = offsets;
        app.total_lines = total;
        app.layout_width = col_width;
        app.layout_dirty = false;
        app.clamp_scroll();
    }

    let article = app.article.as_ref().unwrap();
    let offsets = &app.line_offsets;
    let first = offsets
        .partition_point(|&o| o <= app.scroll)
        .saturating_sub(1);
    let end = offsets.partition_point(|&o| o < app.scroll + app.viewport_height);
    let skip_rows = app.scroll - offsets.get(first).copied().unwrap_or(0);

    let visible: Vec<Line> = article.lines[first.min(article.lines.len())..end.max(first)]
        .iter()
        .map(|wiki_line| {
            Line::from(
                wiki_line
                    .iter()
                    .map(|ws| wiki_span_to_ratatui(ws, selected_link, t))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();

    f.render_widget(
        Paragraph::new(Text::from(visible))
            .scroll((skip_rows.min(u16::MAX as usize) as u16, 0))
            .wrap(Wrap { trim: false }),
        text_area,
    );

    let mut sb_state = ScrollbarState::new(app.total_lines.saturating_sub(app.viewport_height))
        .position(app.scroll);
    f.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_style(s(t.scrollbar_thumb))
            .track_style(s(t.scrollbar_track))
            .begin_symbol(None)
            .end_symbol(None),
        scrollbar_area,
        &mut sb_state,
    );
}

fn wiki_span_to_ratatui<'a>(ws: &'a WikiSpan, selected_link: Option<usize>, t: &Theme) -> Span<'a> {
    if let Some(link_idx) = ws.link_index {
        return if selected_link == Some(link_idx) {
            Span::styled(
                ws.text.as_str(),
                Style::default()
                    .fg(t.link_hi_fg)
                    .bg(t.link_hi_bg)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(
                ws.text.as_str(),
                Style::default()
                    .fg(t.link)
                    .add_modifier(Modifier::UNDERLINED),
            )
        };
    }

    let style = match ws.kind {
        SpanKind::H1 => sb(t.h1),
        SpanKind::H2 => sb(t.h2),
        SpanKind::H3 => sb(t.h3),
        SpanKind::Bold => Style::default().fg(t.bold).add_modifier(Modifier::BOLD),
        SpanKind::Italic => Style::default().fg(t.italic).add_modifier(Modifier::ITALIC),
        SpanKind::Code => s(t.code),
        SpanKind::CodeBlock => Style::default().fg(t.code).bg(t.code_bg),
        SpanKind::Normal => s(t.text),
    };
    Span::styled(ws.text.as_str(), style)
}

fn draw_statusbar(f: &mut Frame, app: &App, t: &Theme, area: Rect) {
    let line: Line = if app.mode == Mode::Find {
        Line::from(vec![
            Span::styled(" find: ", sb(t.accent)),
            Span::styled(format!("{}_", app.find_input), s(t.text)),
        ])
    } else if let Some(status) = &app.status {
        let col = if status.starts_with("Error") {
            t.err
        } else {
            t.spinner
        };
        Line::from(Span::styled(format!("  {status}"), sb(col)))
    } else {
        let hint = match app.mode {
            Mode::Help => " Esc  close",
            Mode::LangPicker | Mode::ThemePicker | Mode::Toc => {
                " j/k  move   Enter  select   Esc  cancel"
            }
            _ => " ?  help   q  quit",
        };
        Line::from(Span::styled(hint, s(t.dim)))
    };

    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(t.status_bg)),
        area,
    );
}

fn draw_picker(
    f: &mut Frame,
    t: &Theme,
    area: Rect,
    title: &str,
    width: u16,
    items: Vec<ListItem>,
    cursor: usize,
) {
    let popup = centered_popup(area, width, items.len() as u16 + 4);
    render_popup_bg(f, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(s(t.accent))
        .bg(t.surface)
        .title(Span::styled(format!(" {title} "), sb(t.title)))
        .title_alignment(Alignment::Center)
        .padding(Padding::horizontal(1));

    let mut state = ListState::default();
    state.select(Some(cursor));

    f.render_stateful_widget(
        List::new(items).block(block).highlight_style(
            Style::default()
                .fg(t.key_fg)
                .bg(t.accent)
                .add_modifier(Modifier::BOLD),
        ),
        popup,
        &mut state,
    );
}

fn item_style(t: &Theme, is_cursor: bool, is_current: bool) -> Style {
    if is_cursor {
        Style::default()
            .fg(t.key_fg)
            .bg(t.accent)
            .add_modifier(Modifier::BOLD)
    } else if is_current {
        sb(t.accent)
    } else {
        s(t.text)
    }
}

fn draw_lang_picker(f: &mut Frame, app: &App, t: &Theme, area: Rect) {
    let items: Vec<ListItem> = LANGUAGES
        .iter()
        .enumerate()
        .map(|(i, lang)| {
            let marker = if i == app.lang_index { "●" } else { " " };
            let label = format!("{marker} {:<5} {}", lang.code, lang.name);
            ListItem::new(Span::styled(
                label,
                item_style(t, i == app.lang_cursor, i == app.lang_index),
            ))
        })
        .collect();
    draw_picker(f, t, area, "Language", 36, items, app.lang_cursor);
}

fn draw_theme_picker(f: &mut Frame, app: &App, t: &Theme, area: Rect) {
    let items: Vec<ListItem> = THEMES
        .iter()
        .enumerate()
        .map(|(i, theme)| {
            let marker = if i == app.theme_index { "●" } else { " " };
            let label = format!("{marker} {}", theme.name);
            let style = item_style(t, i == app.theme_cursor, i == app.theme_index);
            let dot = Span::styled("  ", Style::default().fg(theme.accent));
            ListItem::new(Line::from(vec![Span::styled(label, style), dot]))
        })
        .collect();
    draw_picker(f, t, area, "Theme", 40, items, app.theme_cursor);
}

fn draw_toc(f: &mut Frame, app: &App, t: &Theme, area: Rect) {
    let Some(article) = &app.article else { return };
    let items: Vec<ListItem> = article
        .headings
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let indent = "  ".repeat(h.level.saturating_sub(1) as usize);
            ListItem::new(Span::styled(
                format!("{indent}{}", h.title),
                item_style(t, i == app.toc_cursor, false),
            ))
        })
        .collect();
    let width = (area.width * 2 / 3).clamp(30, 70);
    draw_picker(f, t, area, "Contents", width, items, app.toc_cursor);
}

fn draw_help(f: &mut Frame, t: &Theme, area: Rect) {
    let sections: &[(&str, &[(&str, &str)])] = &[
        (
            "Search",
            &[
                ("Enter", "fetch article"),
                ("Left / Right", "move cursor"),
                ("Ctrl-A / Ctrl-E", "start / end of line"),
                ("Ctrl-U", "clear to start"),
                ("F1", "help"),
                ("Ctrl-L", "open language picker"),
                ("Ctrl-T", "open theme picker"),
                ("Esc", "back to article"),
            ],
        ),
        (
            "Reading",
            &[
                ("/  or  s", "open search"),
                ("j / k", "scroll one line down / up"),
                ("Space / PgDn", "scroll one page down"),
                ("PgUp", "scroll one page up"),
                ("Mouse wheel", "scroll"),
                ("Ctrl-D", "scroll half page down"),
                ("Ctrl-U", "scroll half page up"),
                ("g", "jump to top"),
                ("G", "jump to bottom"),
                ("Tab", "select next link"),
                ("Shift-Tab", "select previous link"),
                ("Enter", "follow selected link"),
                ("Backspace / h", "go back"),
                ("f", "find in article"),
                ("n / N", "next / previous match"),
                ("o", "table of contents"),
                ("L", "open language picker"),
                ("T", "open theme picker"),
                ("q", "quit"),
            ],
        ),
        (
            "Pickers (language / theme)",
            &[
                ("j / k", "move cursor down / up"),
                ("Enter", "confirm selection"),
                ("Esc", "cancel"),
            ],
        ),
    ];

    let content_height: u16 = sections.iter().map(|(_, rows)| rows.len() as u16 + 2).sum();
    let popup_height = (content_height + 2).min(area.height.saturating_sub(2));
    let popup_width: u16 = 56;
    let popup = centered_popup(area, popup_width, popup_height);
    render_popup_bg(f, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(s(t.accent))
        .bg(t.surface)
        .title(Span::styled(" Help ", sb(t.title)))
        .title_alignment(Alignment::Center)
        .padding(Padding::horizontal(1));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let mut lines: Vec<Line> = Vec::new();
    for (i, (section, rows)) in sections.iter().enumerate() {
        if i > 0 {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(*section, sb(t.accent))));
        for (key, desc) in *rows {
            lines.push(Line::from(vec![
                Span::styled(format!("  {key:<18}"), sb(t.key_bg)),
                Span::styled(*desc, s(t.text)),
            ]));
        }
    }

    f.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}
