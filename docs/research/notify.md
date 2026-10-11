# notify Research & Best Practices

Access Date: 2026-10-11
Source URL: https://docs.rs/notify/latest/notify/

## 1. Features
- Cross-platform filesystem watcher using macOS FSEvents natively via `RecommendedWatcher`.
- Debouncing: `notify-debouncer-mini` or in-memory channel debounce to collapse rapid consecutive filesystem write events on SQLite / WAL files.

## 2. Recommended Usage Pattern
```rust
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;

let (tx, rx) = channel();
let mut watcher = RecommendedWatcher::new(move |res| {
    let _ = tx.send(res);
}, Config::default())?;
watcher.watch(Path::new("..."), RecursiveMode::Recursive)?;
```

## 3. Integration with Tokio
- Polling baseline: Combine filesystem event notifications with periodic background polling (e.g. 500ms or 1s intervals) to guarantee no events are missed on SQLite WAL sync or rename operations.
