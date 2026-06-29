//! File watcher service: monitors vault directory for external markdown file changes.
//!
//! When markdown files are edited outside the app (e.g., in VS Code), this service
//! detects the changes, parses them, and sends events to a tokio MPSC channel for the
//! app to process and update the database.
//!
//! Implements debouncing and ignore_next_events cache to prevent infinite loops when
//! the app itself modifies files.

use notify::{Watcher, RecursiveMode, Result as NotifyResult};
use notify::recommended_watcher;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use pkm_fs::{FsNoteRepo, SharedVault};
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use pkm_core::note::Note;
use pkm_core::markdown;
use pkm_core::{Actor, Timestamp, id::NoteId};

/// An event from the file watcher indicating a note was modified or deleted externally.
#[derive(Debug, Clone)]
pub enum NoteWatcherEvent {
    Modified {
        file_path: PathBuf,
        note: Note,
        blocks: Vec<pkm_core::block::Block>,
    },
    Deleted {
        file_path: PathBuf,
    },
}

fn normalize_path(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    let s = s.strip_prefix(r"\\?\").unwrap_or(&s);
    PathBuf::from(s.to_string())
}

/// Tracks file metadata (size + modified time) for files the app has written,
/// so the watcher can distinguish app-initiated writes from external edits.
#[derive(Debug, Clone)]
struct FileMeta {
    size: u64,
    modified: std::time::SystemTime,
}

/// Handle to skip processing on file modification events.
/// Combines a TTL-based cache with file metadata tracking to handle multiple OS events
/// for a single write and to survive delayed OS notifications on slow disks.
#[derive(Clone)]
pub struct IgnoreNextEvent {
    cache: Arc<Mutex<HashMap<PathBuf, (Instant, Option<FileMeta>)>>>,
}

impl IgnoreNextEvent {
    /// Mark a file to be skipped until the TTL expires (default 1 second).
    /// Also records the file's current metadata (size + mtime) so the watcher can
    /// still match events that arrive after the TTL window on a slow disk.
    pub fn skip_next(&self, path: PathBuf) {
        let path = normalize_path(&path);
        let meta = std::fs::metadata(&path).ok().and_then(|m| {
            m.modified().ok().map(|modified| FileMeta { size: m.len(), modified })
        });
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(path, (Instant::now() + Duration::from_secs(1), meta));
        }
    }

    /// Returns true if the event should be skipped because it matches an app-initiated write.
    fn should_skip(&self, path: &Path) -> bool {
        let normalized = normalize_path(path);
        if let Ok(mut cache) = self.cache.lock() {
            let now = Instant::now();
            if let Some((expiry, meta)) = cache.get(&normalized) {
                // Fast path: TTL still valid
                if now < *expiry {
                    return true;
                }
                // Slow path: TTL expired but metadata still matches
                if let Some(ref recorded) = meta {
                    if let Ok(current_meta) = std::fs::metadata(path) {
                        if current_meta.len() == recorded.size
                            && current_meta.modified().ok().as_ref() == Some(&recorded.modified)
                        {
                            return true;
                        }
                    }
                }
            }
            // Clean up expired entries
            cache.retain(|_, &mut (exp, _)| now < exp);
        }
        false
    }
}

/// Start watching a vault directory for markdown file changes.
/// Returns (receiver, ignore_handle) where:
/// - receiver emits `NoteWatcherEvent` when files change externally
/// - ignore_handle can be used to skip processing on app-initiated writes
///
/// The watcher runs in a background tokio task and will continue until the returned
/// receiver is dropped.
pub fn watch_vault(vault_path: &Path, vault_state: SharedVault) -> NotifyResult<(mpsc::UnboundedReceiver<NoteWatcherEvent>, IgnoreNextEvent)> {
    let (tx, rx) = mpsc::unbounded_channel();
    let vault_path = vault_path.to_path_buf();
    let ignore_state = IgnoreNextEvent { cache: Arc::new(Mutex::new(HashMap::new())) };
    let ignore_handle = ignore_state.clone();

    // Create a watcher in a background tokio task
    tokio::spawn(async move {
        if let Err(e) = watch_impl(&vault_path, vault_state, tx, ignore_state).await {
            eprintln!("File watcher error: {}", e);
        }
    });

    Ok((rx, ignore_handle))
}

async fn watch_impl(
    vault_path: &Path,
    vault_state: SharedVault,
    tx: mpsc::UnboundedSender<NoteWatcherEvent>,
    ignore_state: IgnoreNextEvent,
) -> NotifyResult<()> {
    let vault_path = vault_path.to_path_buf();
    let (watch_tx, mut watch_rx) = mpsc::unbounded_channel();

    let mut watcher = recommended_watcher(move |res: NotifyResult<notify::Event>| {
        let _ = watch_tx.send(res);
    })?;

    watcher.watch(&vault_path, RecursiveMode::Recursive)?;

    // Debouncing state: map of paths to their debounce deadlines
    let mut pending_files: HashMap<PathBuf, tokio::task::JoinHandle<()>> = HashMap::new();
    let debounce_duration = Duration::from_millis(200);

    // Periodic cleanup interval to reclaim finished JoinHandles even when
    // no new file modifications arrive (prevents memory leak from idle task).
    let mut last_cleanup = tokio::time::Instant::now();
    let cleanup_interval = Duration::from_secs(10);

    // Process watcher events
    while let Some(Ok(event)) = watch_rx.recv().await {
        use notify::EventKind;

        // Periodic cleanup of finished handles
        if last_cleanup.elapsed() >= cleanup_interval {
            pending_files.retain(|_, handle| !handle.is_finished());
            last_cleanup = tokio::time::Instant::now();
        }

        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                for path in event.paths {
                    // Only process .md files inside the notes/ directory (including subdirectories)
                    if path.extension().and_then(|s| s.to_str()) != Some("md") {
                        continue;
                    }
                    let normalized_path = normalize_path(&path);
                    let notes_path = normalize_path(&vault_path.join("notes"));
                    if !normalized_path.starts_with(&notes_path) {
                        continue;
                    }

                    // Check if this file was written by the app (TTL + metadata match)
                    if ignore_state.should_skip(&normalized_path) {
                        continue;
                    }

                    // Cancel any existing pending task for this file
                    if let Some(handle) = pending_files.remove(&normalized_path) {
                        handle.abort();
                    }

                    // Spawn a new debounced task for this file
                    let path_clone = path.clone();
                    let tx_clone = tx.clone();
                    let vault_state_clone = vault_state.clone();
                    let vault_path_clone = vault_path.clone();

                    let task = tokio::spawn(async move {
                        // Wait for debounce duration
                        sleep(debounce_duration).await;

                        // Process the file
                        if let Ok(markdown_text) = tokio::fs::read_to_string(&path_clone).await {
                            let file_name = path_clone.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
                            let note_repo = FsNoteRepo {
                                state: vault_state_clone.clone(),
                                vault_path: vault_path_clone.clone(),
                            };

                            // Find existing note ID by filename, otherwise mint a new one.
                            let fallback_id = note_repo.get_by_filename(&file_name)
                                .map(|n| n.id)
                                .unwrap_or_else(NoteId::new);

                            let now = Timestamp::now_utc();

                            match markdown::markdown_to_note(
                                &markdown_text,
                                fallback_id,
                                Actor::User,
                                now,
                            ) {
                                Ok((note, blocks)) => {
                                    let event = NoteWatcherEvent::Modified {
                                        file_path: path_clone.clone(),
                                        note,
                                        blocks,
                                    };
                                    let _ = tx_clone.send(event);
                                }
                                Err(e) => {
                                    eprintln!("Failed to parse markdown {}: {}", path_clone.display(), e);
                                }
                            }
                        }
                    });

                    pending_files.retain(|_, handle| !handle.is_finished());
                    pending_files.insert(normalized_path, task);
                }
            }
            EventKind::Remove(_) => {
                for path in event.paths {
                    if path.extension().and_then(|s| s.to_str()) != Some("md") {
                        continue;
                    }
                    let normalized_path = normalize_path(&path);
                    let notes_path = normalize_path(&vault_path.join("notes"));
                    if !normalized_path.starts_with(&notes_path) {
                        continue;
                    }
                    if ignore_state.should_skip(&normalized_path) {
                        continue;
                    }
                    let _ = tx.send(NoteWatcherEvent::Deleted { file_path: path });
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
    use tokio::time::timeout;
    use tempfile::TempDir;

    #[tokio::test]
    async fn file_watcher_detects_new_markdown_file() {
        use pkm_fs::VaultState;
        use std::sync::RwLock;

        let temp_dir = TempDir::new().unwrap();
        let vault_path = temp_dir.path();
        let vault_state = Arc::new(RwLock::new(VaultState::new()));

        // Start watcher
        let (mut rx, _ignore) = watch_vault(vault_path, vault_state).expect("failed to start watcher");

        // Give watcher time to start
        sleep(Duration::from_millis(100)).await;

        // Create a notes/ directory and a markdown file inside it
        let notes_dir = vault_path.join("notes");
        fs::create_dir_all(&notes_dir).expect("failed to create notes dir");
        let note_path = notes_dir.join("test-note.md");
        let markdown_content = "---\nid: 123e4567-e89b-12d3-a456-426614174000\ncreated_by:\n  kind: user\ncreated_at: 2026-06-26T00:00:00Z\nmetadata: {}\n---\n\n# Test Note\n\nSome content";
        fs::write(&note_path, markdown_content).expect("failed to write test file");

        // Wait for watcher to detect and process the file (with timeout)
        if let Ok(Some(NoteWatcherEvent::Modified { note, file_path, .. })) = timeout(Duration::from_secs(2), rx.recv()).await {
            assert_eq!(note.title, "Test Note");
            assert_eq!(file_path, note_path);
        } else {
            panic!("Expected NoteWatcherEvent::Modified");
        }
    }
}
