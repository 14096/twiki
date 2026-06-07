use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Article {
    pub title: String,
    pub lines: Vec<Vec<Span>>,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub line: usize,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    query: SearchQuery,
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
    parse: ParseResult,
}

#[derive(Debug, Deserialize)]
struct ParseResult {
    title: String,
    text: TextContent,
}

#[derive(Debug, Deserialize)]
struct TextContent {
    #[serde(rename = "*")]
    content: String,
}

pub async fn fetch_article(query: &str, lang: &str) -> anyhow::Result<Article> {
    let client = reqwest::Client::builder()
        .user_agent("twiki/0.1 (https://github.com/user/twiki)")
        .build()?;

    let base = format!("https://{}.wikipedia.org/w/api.php", lang);

    let search_url = format!(
        "{base}?action=query&list=search&srsearch={}&format=json&srlimit=1",
        urlencoding::encode(query)
    );
    let resp: SearchResponse = client.get(&search_url).send().await?.json().await?;
    let title = resp
        .query
        .search
        .into_iter()
        .next()
        .map(|r| r.title)
        .unwrap_or_else(|| query.to_string());

    let parse_url = format!(
        "{base}?action=parse&page={}&prop=text&format=json&disablelimitreport=1",
        urlencoding::encode(&title)
    );
    let parsed: ParseResponse = client.get(&parse_url).send().await?.json().await?;

    let article = parse_html(&parsed.parse.title, &parsed.parse.text.content);
    Ok(article)
}

#[derive(Clone)]
struct TextCtx {
    link: Option<String>,
    heading: u8,
    bold: bool,
    italic: bool,
    code: bool,
    in_pre: bool,
}

impl Default for TextCtx {
    fn default() -> Self {
        Self {
            link: None,
            heading: 0,
            bold: false,
            italic: false,
            code: false,
            in_pre: false,
        }
    }
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
            4 | 5 | 6 => SpanKind::H3,
            _ => {
                if self.bold {
                    SpanKind::Bold
                } else if self.italic {
                    SpanKind::Italic
                } else {
                    SpanKind::Normal
                }
            }
        }
    }
}

enum Token {
    Text { text: String, ctx: TextCtx },
    Newline,
}

fn parse_html(title: &str, html: &str) -> Article {
    use scraper::{ElementRef, Html, Selector};

    let document = Html::parse_fragment(html);
    let mut tokens: Vec<Token> = Vec::new();

    let skip_sel = Selector::parse(
        "table, figure, figcaption, \
         .infobox, .navbox, .sidebar, .reflist, \
         .references, sup.reference, .mw-editsection, \
         .thumb, .gallery, .hatnote, style, script",
    )
    .unwrap();

    let stop_sel = Selector::parse("#Notes, #References, #Book_sources, #External_links").unwrap();

    fn walk_el(
        el: ElementRef,
        tokens: &mut Vec<Token>,
        skip_sel: &Selector,
        stop_sel: &Selector,
        ctx: &TextCtx,
    ) -> bool {
        let tag = el.value().name();

        if skip_sel.matches_with_scope(&el, None) {
            return false;
        }

        if stop_sel.matches_with_scope(&el, None) {
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

        let is_block = heading_level > 0
            || matches!(tag, "p" | "li" | "div" | "blockquote" | "pre" | "dd" | "dt");

        if is_block {
            tokens.push(Token::Newline);
        } else if tag == "br" {
            tokens.push(Token::Newline);
            return false;
        }

        let child_ctx = TextCtx {
            link: if tag == "a" {
                let href = el.value().attr("href").unwrap_or("");
                if href.starts_with("/wiki/") && !href.contains(':') {
                    let page = href.trim_start_matches("/wiki/").replace('_', " ");
                    Some(percent_decode(&page))
                } else {
                    ctx.link.clone()
                }
            } else {
                ctx.link.clone()
            },
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

        for child in el.children() {
            use scraper::node::Node;
            match child.value() {
                Node::Text(t) => {
                    let text: &str = t.as_ref();
                    if child_ctx.in_pre || !text.trim().is_empty() {
                        tokens.push(Token::Text {
                            text: text.to_string(),
                            ctx: child_ctx.clone(),
                        });
                    }
                }
                Node::Element(_) => {
                    if let Some(child_el) = ElementRef::wrap(child) {
                        if walk_el(child_el, tokens, skip_sel, stop_sel, &child_ctx) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }

        if is_block {
            tokens.push(Token::Newline);
        }
        false
    }

    let root = document.root_element();
    walk_el(root, &mut tokens, &skip_sel, &stop_sel, &TextCtx::default());

    let mut lines: Vec<Vec<Span>> = Vec::new();
    let mut links: Vec<Link> = Vec::new();
    let mut current_line: Vec<Span> = Vec::new();

    let flush = |lines: &mut Vec<Vec<Span>>, line: &mut Vec<Span>| {
        if line.is_empty() {
            lines.push(vec![Span {
                text: String::new(),
                link_index: None,
                kind: SpanKind::Normal,
            }]);
        } else {
            lines.push(std::mem::take(line));
        }
    };

    for token in tokens {
        match token {
            Token::Newline => {
                flush(&mut lines, &mut current_line);
            }
            Token::Text { text, ctx } => {
                for (i, part) in text.split('\n').enumerate() {
                    if i > 0 {
                        flush(&mut lines, &mut current_line);
                    }
                    if part.is_empty() && !ctx.in_pre {
                        continue;
                    }
                    let kind = ctx.kind();
                    let link_index = if let Some(ref lt) = ctx.link {
                        let line_no = lines.len();
                        let idx = links.iter().position(|l| &l.title == lt);
                        Some(idx.unwrap_or_else(|| {
                            links.push(Link {
                                title: lt.clone(),
                                line: line_no,
                            });
                            links.len() - 1
                        }))
                    } else {
                        None
                    };
                    current_line.push(Span {
                        text: part.to_string(),
                        link_index,
                        kind,
                    });
                }
            }
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    let mut collapsed: Vec<Vec<Span>> = Vec::with_capacity(lines.len());
    let mut blank_run = 0usize;
    for line in lines {
        let is_blank = line.iter().all(|s| s.text.trim().is_empty());
        let is_code = line.iter().any(|s| s.kind == SpanKind::CodeBlock);
        if is_blank && !is_code {
            blank_run += 1;
            if blank_run <= 1 {
                collapsed.push(line);
            }
        } else {
            blank_run = 0;
            collapsed.push(line);
        }
    }

    Article {
        title: title.to_string(),
        lines: collapsed,
        links,
    }
}

fn percent_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(h), Some(l)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2])) {
                out.push(char::from(h * 16 + l));
                i += 3;
                continue;
            }
        }
        out.push(char::from(bytes[i]));
        i += 1;
    }
    out
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
