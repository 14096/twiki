# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-11-08

### Added

- Back history (`Backspace` / `h`) that restores the previous scroll position.
- Find in article (`f`, then `n` / `N`).
- Table of contents popup (`o`).
- Mouse wheel scrolling.
- Search box cursor editing (Left/Right/Home/End/Delete, Ctrl-A/E/U).
- `F1` opens help from any mode.
- Bullet and numbered list rendering.
- Theme and language persisted to `~/.config/twiki/config`.
- Clear "article not found" and API error messages.
- Unit tests and a CI workflow.

### Changed

- Articles load asynchronously; the UI no longer freezes during fetches.
- The terminal redraws on events instead of polling every 100ms.
- A single HTTP client is reused, and exact-title lookup is tried before search, reducing requests.
- Only the visible part of an article is rendered.
- `?` can be typed in the search box.
- Release profile uses LTO, stripping and `panic = "abort"`; tokio features trimmed and reqwest uses rustls.

### Fixed

- Non-ASCII link titles were decoded incorrectly.
- Scrollbar, `G` and link scrolling used an inaccurate wrapped-line count.
- Link positions drifted after blank lines were collapsed.
- `Ctrl-D` / `Ctrl-U` scrolled a full page instead of half.
- Ctrl/Alt key chords were inserted as text in the search box.
- Key release events could trigger actions twice on some platforms.
- The terminal was not restored after a panic.
- Hidden short-description text appeared at the top of articles.

## [0.1.0]

- Initial release.
