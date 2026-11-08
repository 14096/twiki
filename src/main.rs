mod app;
mod config;
mod input;
mod theme;
mod ui;
mod wikipedia;

use app::{Action, App};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyEventKind},
    execute,
};
use futures_util::StreamExt;
use tokio::sync::mpsc;

type FetchResult = Result<wikipedia::Article, String>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = wikipedia::build_client()?;

    let mut terminal = ratatui::init();
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
        default_hook(info);
    }));
    execute!(std::io::stdout(), EnableMouseCapture)?;

    let mut app = App::new(config::load());
    let result = run(&mut terminal, &mut app, client).await;

    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();

    if let Err(e) = &result {
        eprintln!("Error: {e}");
    }
    result
}

async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    client: reqwest::Client,
) -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<FetchResult>();
    let mut events = EventStream::new();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        tokio::select! {
            ev = events.next() => {
                let Some(ev) = ev else { return Ok(()) };
                match ev? {
                    Event::Key(key) if key.kind != KeyEventKind::Release => {
                        match app.handle_key(key) {
                            Some(Action::Quit) => return Ok(()),
                            Some(Action::Fetch { query, lang }) => {
                                let client = client.clone();
                                let tx = tx.clone();
                                tokio::spawn(async move {
                                    let res = wikipedia::fetch_article(&client, &query, lang)
                                        .await
                                        .map_err(|e| e.to_string());
                                    let _ = tx.send(res);
                                });
                            }
                            None => {}
                        }
                    }
                    Event::Mouse(m) => app.handle_mouse(m),
                    _ => {}
                }
            }
            Some(res) = rx.recv() => app.finish_fetch(res),
        }
    }
}
