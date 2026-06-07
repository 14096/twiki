# twiki

A terminal Wikipedia reader.

## Features

- Full article text with headings, bold, italic, inline code, and **syntax-highlighted code blocks**
- Clickable internal links — navigate Wikipedia without leaving the terminal
- 10 built-in colour themes with live preview

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

On launch the search bar is focused. Type a topic and press `Enter`.

Press `?` at any time to open the full keybindings reference.

### Search mode

| Key         | Action               |
| ----------- | -------------------- |
| `Enter`     | Fetch article        |
| `Backspace` | Delete character     |
| `Ctrl-L`    | Open language picker |
| `Ctrl-T`    | Open theme picker    |
| `Esc`       | Back to article      |

### Reading mode

| Key              | Action                    |
| ---------------- | ------------------------- |
| `/` or `s`       | Open search               |
| `j` / `k`        | Scroll one line down / up |
| `Space` / `PgDn` | Scroll one page down      |
| `PgUp`           | Scroll one page up        |
| `Ctrl-D`         | Scroll half page down     |
| `Ctrl-U`         | Scroll half page up       |
| `g`              | Jump to top               |
| `G`              | Jump to bottom            |
| `Tab`            | Select next link          |
| `Shift-Tab`      | Select previous link      |
| `Enter`          | Follow selected link      |
| `L`              | Open language picker      |
| `T`              | Open theme picker         |
| `?`              | Open help popup           |
| `q`              | Quit                      |
