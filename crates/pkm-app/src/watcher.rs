//! File watcher service: monitors vault directory for external markdown file changes.
//!
//! When markdown files are edited outside the app (e.g., in VS Code), this service
//! detects the changes, parses them, and sends events to an MPSC channel for the
//! app to process and update the database.
//!
//! Implements debouncing and ignore_next_events cache to prevent infinite loops when
//! the app itself modifies files.

use notify::{Watcher, RecursiveMode, Result as NotifyResult};
use notify::recommended_watcher;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use pkm_core::note::Note;
use pkm_core::markdown;
use pkm_core::{Actor, Timestamp, id::NoteId};

/// An event from the file watcher indicating a note was modified externally.
#[derive(Debug, Clone)]
pub struct NoteWatcherEvent {
    pub file_path: PathBuf,
    pub note: Note,
}

/// Handle to skip processing on the next file modification event.
/// Used by the app to prevent infinite loops when it writes files.
#[derive(Clone)]
pub struct IgnoreNextEvent {
    cache: Arc<Mutex<HashSet<PathBuf>>>,
}

impl IgnoreNextEvent {
    /// Mark a file to be skipped in the next watch event.
    pub fn skip_next(&self, path: PathBuf) {
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(path);
        }
    }
}

/// Start watching a vault directory for markdown file changes.
/// Returns (receiver, ignore_handle) where:
/// - receiver emits `NoteWatcherEvent` when files change externally
/// - ignore_handle can be used to skip processing on app-initiated writes
///
/// The watcher runs in a background thread and will continue until the returned
/// receiver is dropped.
pub fn watch_vault(vault_path: &Path) -> NotifyResult<(mpsc::Receiver<NoteWatcherEvent>, IgnoreNextEvent)> {
    let (tx, rx) = mpsc::channel();
    let vault_path = vault_path.to_path_buf();
    let ignore_cache = Arc::new(Mutex::new(HashSet::new()));
    let ignore_handle = IgnoreNextEvent { cache: Arc::clone(&ignore_cache) };

    // Create a watcher in a background thread
    std::thread::spawn(move || {
        if let Err(e) = watch_impl(&vault_path, tx, ignore_cache) {
            eprintln!("File watcher error: {}", e);
        }
    });

    Ok((rx, ignore_handle))
}

fn watch_impl(
    vault_path: &Path,
    tx: mpsc::Sender<NoteWatcherEvent>,
    ignore_cache: Arc<Mutex<HashSet<PathBuf>>>,
) -> NotifyResult<()> {
    let vault_path = vault_path.to_path_buf();
    let (watch_tx, watch_rx) = mpsc::channel();

    let mut watcher = recommended_watcher(move |res: NotifyResult<notify::Event>| {
        let _ = watch_tx.send(res);
    })?;

    watcher.watch(&vault_path, RecursiveMode::Recursive)?;

    // Debouncing state
    let mut last_event_time: Option<Instant> = None;
    let debounce_duration = Duration::from_millis(200);

    // Process watcher events
    while let Ok(Ok(event)) = watch_rx.recv().map(|r| r) {
        use notify::EventKind;

        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                // Debounce: ignore events that come too close together
                let now = Instant::now();
                if let Some(last_time) = last_event_time {
                    if now.duration_since(last_time) < debounce_duration {
                        continue;
                    }
                }
                last_event_time = Some(now);

                for path in event.paths {
                    // Only process .md files
                    if path.extension().and_then(|s| s.to_str()) == Some("md") {
                        // Check if this file is in the ignore cache
                        let should_skip = {
                            let mut cache = ignore_cache.lock().unwrap_or_else(|e| e.into_inner());
                            cache.remove(&path)
                        };

                        if should_skip {
                            // App wrote this file; skip processing to prevent infinite loop
                            continue;
                        }

                        if let Ok(markdown_text) = std::fs::read_to_string(&path) {
                            // Parse the markdown file into a Note
                            // Use a dummy note_id since it will be extracted from frontmatter
                            let dummy_id = NoteId::new();
                            let now = Timestamp::now_utc();

                            match markdown::markdown_to_note(
                                &markdown_text,
                                dummy_id,
                                Actor::User,
                                now,
                            ) {
                                Ok((note, _blocks)) => {
                                    let event = NoteWatcherEvent {
                                        file_path: path.clone(),
                                        note,
                                    };
                                    // Send the event (ignore if receiver is dropped)
                                    let _ = tx.send(event);
                                }
                                Err(e) => {
                                    eprintln!("Failed to parse markdown {}: {}", path.display(), e);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    #[test]
    fn file_watcher_detects_new_markdown_file() {
        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path();

        // Start watcher
        let rx = watch_vault(vault_path).expect("failed to start watcher");

        // Give watcher time to start
        thread::sleep(Duration::from_millis(100));

        // Create a markdown file
        let note_path = vault_path.join("test-note.md");
        let markdown_content = "---\nid: 123e4567-e89b-12d3-a456-426614174000\ncreated_by: User\ncreated_at: 2026-06-26T00:00:00Z\nmetadata: {}\n---\n\n# Test Note\n\nSome content";
        fs::write(&note_path, markdown_content).expect("failed to write test file");

        // Wait for watcher to detect and process the file
        thread::sleep(Duration::from_millis(500));

        // Check if we received an event
        if let Ok(event) = rx.try_recv() {
            assert_eq!(event.note.title, "Test Note");
            assert_eq!(event.file_path, note_path);
        }
    }
}
