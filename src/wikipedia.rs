use scraper::{ElementRef, Html, Selector, node::Node};
use serde::Deserialize;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone, Default)]
pub struct Article {
    pub title: String,
    pub lines: Vec<Vec<Span>>,
    pub links: Vec<Link>,
    pub headings: Vec<Heading>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanKind {
    Normal,
    H1,
    H2,
    H3,
    Bold,
    Italic,
    Code,
    CodeBlock,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub text: String,
    pub link_index: Option<usize>,
    pub kind: SpanKind,
}

#[derive(Debug, Clone)]
pub struct Link {
    pub title: String,
    /// Every logical line on which this link appears (ascending).
    pub lines: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Heading {
    pub title: String,
    pub level: u8,
    pub line: usize,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    query: Option<SearchQuery>,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    search: Vec<SearchResult>,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    title: String,
}

#[derive(Debug, Deserialize)]
struct ParseResponse {
    parse: Option<ParseResult>,
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: String,
    info: String,
}

#[derive(Debug, Deserialize)]
struct ParseResult {
    title: String,
    text: String,
}

pub fn build_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!(
            "twiki/",
            env!("CARGO_PKG_VERSION"),
            " (",
            env!("CARGO_PKG_REPOSITORY"),
            ")"
        ))
        .timeout(std::time::Duration::from_secs(20))
        .build()
}

async fn fetch_parse(
    client: &reqwest::Client,
    base: &str,
    page: &str,
) -> anyhow::Result<Option<Article>> {
    let url = format!(
        "{base}?action=parse&page={}&prop=text&format=json&formatversion=2&redirects=1&disablelimitreport=1",
        urlencoding::encode(page)
    );
    let resp: ParseResponse = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    if let Some(parsed) = resp.parse {
        let html = parsed.text;
        let title = parsed.title;
        let article = tokio::task::spawn_blocking(move || parse_html(&title, &html)).await?;
        return Ok(Some(article));
    }
    match resp.error {
        Some(e) if matches!(e.code.as_str(), "missingtitle" | "invalidtitle") => Ok(None),
        Some(e) => anyhow::bail!("{}", e.info),
        None => Ok(None),
    }
}

/// Tries the query as an exact title first, then falls back to full-text search.
pub async fn fetch_article(
    client: &reqwest::Client,
    query: &str,
    lang: &str,
) -> anyhow::Result<Article> {
    let base = format!("https://{lang}.wikipedia.org/w/api.php");

    if let Some(article) = fetch_parse(client, &base, query).await? {
        return Ok(article);
    }

    let search_url = format!(
        "{base}?action=query&list=search&srsearch={}&format=json&srlimit=1",
        urlencoding::encode(query)
    );
    let resp: SearchResponse = client
        .get(&search_url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let title = resp
        .query
        .and_then(|q| q.search.into_iter().next())
        .map(|r| r.title)
        .ok_or_else(|| anyhow::anyhow!("No article found for \"{query}\""))?;

    fetch_parse(client, &base, &title)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Article not found: \"{title}\""))
}

#[derive(Clone, Default)]
struct TextCtx {
    link: Option<Rc<str>>,
    heading: u8,
    bold: bool,
    italic: bool,
    code: bool,
    in_pre: bool,
}

impl TextCtx {
    fn kind(&self) -> SpanKind {
        if self.in_pre {
            return SpanKind::CodeBlock;
        }
        if self.code {
            return SpanKind::Code;
        }
        match self.heading {
            1 => SpanKind::H1,
            2 | 3 => SpanKind::H2,
            4..=6 => SpanKind::H3,
            _ if self.bold => SpanKind::Bold,
            _ if self.italic => SpanKind::Italic,
            _ => SpanKind::Normal,
        }
    }
}

#[derive(Default)]
struct Builder {
    lines: Vec<Vec<Span>>,
    links: Vec<Link>,
    link_map: HashMap<Rc<str>, usize>,
    current: Vec<Span>,
    blank_run: usize,
}

impl Builder {
    fn push_line(&mut self, line: Vec<Span>) {
        let is_blank = line.iter().all(|s| s.text.trim().is_empty());
        let is_code = line.iter().any(|s| s.kind == SpanKind::CodeBlock);
        if is_blank && !is_code {
            self.blank_run += 1;
            if self.lines.is_empty() || self.blank_run > 1 {
                return;
            }
        } else {
            self.blank_run = 0;
        }
        self.lines.push(line);
    }

    fn newline(&mut self) {
        let line = if self.current.is_empty() {
            vec![Span {
                text: String::new(),
                link_index: None,
                kind: SpanKind::Normal,
            }]
        } else {
            std::mem::take(&mut self.current)
        };
        self.push_line(line);
    }

    fn text(&mut self, text: &str, ctx: &TextCtx) {
        for (i, part) in text.split('\n').enumerate() {
            if i > 0 {
                self.newline();
            }
            if part.is_empty() && !ctx.in_pre {
                continue;
            }
            let link_index = ctx.link.as_ref().map(|title| {
                let line_no = self.lines.len();
                let idx = *self.link_map.entry(title.clone()).or_insert_with(|| {
                    self.links.push(Link {
                        title: title.to_string(),
                        lines: Vec::new(),
                    });
                    self.links.len() - 1
                });
                let lines = &mut self.links[idx].lines;
                if lines.last() != Some(&line_no) {
                    lines.push(line_no);
                }
                idx
            });
            self.current.push(Span {
                text: part.to_string(),
                link_index,
                kind: ctx.kind(),
            });
        }
    }

    fn finish(mut self, title: &str) -> Article {
        if !self.current.is_empty() {
            let line = std::mem::take(&mut self.current);
            self.push_line(line);
        }
        let headings = self
            .lines
            .iter()
            .enumerate()
            .filter_map(|(line, spans)| {
                let level = match spans.iter().find(|s| !s.text.trim().is_empty())?.kind {
                    SpanKind::H1 => 1,
                    SpanKind::H2 => 2,
                    SpanKind::H3 => 3,
                    _ => return None,
                };
                let all_heading = spans.iter().all(|s| {
                    s.text.trim().is_empty()
                        || matches!(s.kind, SpanKind::H1 | SpanKind::H2 | SpanKind::H3)
                });
                if !all_heading {
                    return None;
                }
                let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                Some(Heading {
                    title: text.trim().to_string(),
                    level,
                    line,
                })
            })
            .collect();
        Article {
            title: title.to_string(),
            lines: self.lines,
            links: self.links,
            headings,
        }
    }
}

struct Selectors {
    skip: Selector,
    stop: Selector,
}

fn walk_el(el: ElementRef, b: &mut Builder, sels: &Selectors, ctx: &TextCtx) -> bool {
    let tag = el.value().name();

    if sels.skip.matches_with_scope(&el, None) {
        return false;
    }
    if sels.stop.matches_with_scope(&el, None) {
        return true;
    }

    let heading_level: u8 = match tag {
        "h1" => 1,
        "h2" => 2,
        "h3" => 3,
        "h4" => 4,
        "h5" => 5,
        "h6" => 6,
        _ => 0,
    };

    let is_block =
        heading_level > 0 || matches!(tag, "p" | "li" | "div" | "blockquote" | "pre" | "dd" | "dt");

    if tag == "br" {
        b.newline();
        return false;
    }
    if is_block {
        b.newline();
    }

    let link = if tag == "a" {
        let href = el.value().attr("href").unwrap_or("");
        match href.strip_prefix("/wiki/") {
            Some(rest) if !rest.contains(':') => {
                let page = rest.split('#').next().unwrap_or(rest);
                let page = decode_title(page);
                if page.is_empty() {
                    ctx.link.clone()
                } else {
                    Some(Rc::from(page))
                }
            }
            _ => ctx.link.clone(),
        }
    } else {
        ctx.link.clone()
    };

    let child_ctx = TextCtx {
        link,
        heading: if heading_level > 0 {
            heading_level
        } else {
            ctx.heading
        },
        bold: ctx.bold || matches!(tag, "b" | "strong"),
        italic: ctx.italic || matches!(tag, "i" | "em"),
        code: ctx.code || matches!(tag, "code" | "tt" | "kbd" | "samp"),
        in_pre: ctx.in_pre || tag == "pre",
    };

    if tag == "li" {
        let depth = el
            .ancestors()
            .filter_map(ElementRef::wrap)
            .filter(|a| a.value().name() == "li")
            .count();
        let ordered = el
            .parent()
            .and_then(ElementRef::wrap)
            .is_some_and(|p| p.value().name() == "ol");
        let marker = if ordered {
            let n = el
                .prev_siblings()
                .filter_map(ElementRef::wrap)
                .filter(|s| s.value().name() == "li")
                .count()
                + 1;
            format!("{n}. ")
        } else {
            "• ".to_string()
        };
        let plain = TextCtx {
            link: None,
            ..child_ctx.clone()
        };
        b.text(&format!("{}{marker}", "  ".repeat(depth)), &plain);
    }

    for child in el.children() {
        match child.value() {
            Node::Text(t) => {
                let text: &str = t.as_ref();
                if child_ctx.in_pre || !text.trim().is_empty() {
                    b.text(text, &child_ctx);
                }
            }
            Node::Element(_) => {
                if let Some(child_el) = ElementRef::wrap(child)
                    && walk_el(child_el, b, sels, &child_ctx)
                {
                    return true;
                }
            }
            _ => {}
        }
    }

    if is_block {
        b.newline();
    }
    false
}

pub fn parse_html(title: &str, html: &str) -> Article {
    let document = Html::parse_fragment(html);
    let sels = Selectors {
        skip: Selector::parse(
            "table, figure, figcaption, \
             .infobox, .navbox, .sidebar, .reflist, \
             .references, sup.reference, .mw-editsection, \
             .thumb, .gallery, .hatnote, .shortdescription, style, script",
        )
        .unwrap(),
        stop: Selector::parse("#Notes, #References, #Book_sources, #External_links").unwrap(),
    };

    let mut b = Builder::default();
    walk_el(document.root_element(), &mut b, &sels, &TextCtx::default());
    b.finish(title)
}

fn decode_title(s: &str) -> String {
    let decoded = urlencoding::decode(s)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| s.to_string());
    decoded.replace('_', " ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_utf8_titles() {
        assert_eq!(decode_title("Caf%C3%A9_au_lait"), "Café au lait");
        assert_eq!(decode_title("A_%26_B%21"), "A & B!");
    }

    #[test]
    fn collects_links_and_dedupes() {
        let a = parse_html(
            "T",
            r#"<p><a href="/wiki/Rust_(language)">Rust</a> and <a href="/wiki/Rust_(language)#History">again</a></p>"#,
        );
        assert_eq!(a.links.len(), 1);
        assert_eq!(a.links[0].title, "Rust (language)");
    }

    #[test]
    fn ignores_special_links() {
        let a = parse_html("T", r#"<p><a href="/wiki/File:X.png">x</a></p>"#);
        assert!(a.links.is_empty());
    }

    #[test]
    fn link_lines_match_collapsed_lines() {
        let a = parse_html(
            "T",
            r#"<p>one</p><p></p><p></p><p></p><p><a href="/wiki/B">b</a></p>"#,
        );
        let line = a.links[0].lines[0];
        assert!(a.lines[line].iter().any(|s| s.link_index == Some(0)));
    }

    #[test]
    fn extracts_headings_and_lists() {
        let a = parse_html(
            "T",
            "<h2>Intro</h2><ul><li>x</li></ul><ol><li>a</li><li>b</li></ol>",
        );
        assert_eq!(a.headings.len(), 1);
        assert_eq!(a.headings[0].title, "Intro");
        let text: Vec<String> = a
            .lines
            .iter()
            .map(|l| l.iter().map(|s| s.text.as_str()).collect())
            .collect();
        assert!(text.contains(&"• x".to_string()));
        assert!(text.contains(&"2. b".to_string()));
    }

    #[test]
    fn stops_at_references() {
        let a = parse_html(
            "T",
            r#"<p>keep</p><h2 id="References">Refs</h2><p>drop</p>"#,
        );
        assert!(a.lines.iter().flatten().all(|s| s.text != "drop"));
    }
}
