# twiki

A terminal Wikipedia reader.

## Features

- Full article text with headings, bold, italic, inline code, code blocks, and bulleted/numbered lists
- Clickable internal links, so you can navigate Wikipedia without leaving the terminal
- Back history, find-in-article, and a table of contents
- 20 Wikipedia languages
- 10 built-in colour themes with live preview
- Mouse wheel scrolling

## Install

Requires [Rust](https://rustup.rs) (edition 2024, stable).

```
cargo install --path .
```

Or run directly:

```
cargo run --release
```

## Usage

On launch the search bar is focused. Type a topic and press `Enter`. The query is tried as an exact article title first, then falls back to Wikipedia search.

Press `?` (or `F1` from anywhere) to open the full keybindings reference.

### Search mode

| Key                 | Action               |
| ------------------- | -------------------- |
| `Enter`             | Fetch article        |
| `Backspace`         | Delete character     |
| `←` / `→`           | Move cursor          |
| `Ctrl-A` / `Ctrl-E` | Start / end of line  |
| `Ctrl-U`            | Clear to start       |
| `Ctrl-L`            | Open language picker |
| `Ctrl-T`            | Open theme picker    |
| `F1`                | Open help            |
| `Esc`               | Back to article      |

### Reading mode

| Key               | Action                    |
| ----------------- | ------------------------- |
| `/` or `s`        | Open search               |
| `j` / `k`         | Scroll one line down / up |
| `Space` / `PgDn`  | Scroll one page down      |
| `PgUp`            | Scroll one page up        |
| `Ctrl-D`          | Scroll half page down     |
| `Ctrl-U`          | Scroll half page up       |
| `g`               | Jump to top               |
| `G`               | Jump to bottom            |
| `Tab`             | Select next link          |
| `Shift-Tab`       | Select previous link      |
| `Enter`           | Follow selected link      |
| `Backspace` / `h` | Go back                   |
| `f`               | Find in article           |
| `n` / `N`         | Next / previous match     |
| `o`               | Table of contents         |
| `L`               | Open language picker      |
| `T`               | Open theme picker         |
| `?`               | Open help popup           |
| `q`               | Quit                      |

## Configuration

Theme and language are saved automatically to `~/.config/twiki/config` (or `$XDG_CONFIG_HOME/twiki/config`).
