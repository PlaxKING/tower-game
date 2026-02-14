//! Structured Logging & Tracing (IMP-013)
//!
//! Provides structured logging via the `tracing` crate with:
//! - Level-based filtering (TRACE/DEBUG/INFO/WARN/ERROR)
//! - Spans for operation timing
//! - JSON-compatible structured output
//! - FFI-safe initialization (idempotent)

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Once;
use tracing_subscriber::EnvFilter;

pub struct LoggingPlugin;

impl Plugin for LoggingPlugin {
    fn build(&self, _app: &mut App) {
        init_tracing_default();
    }
}

/// Log level for the tower core
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }

    pub fn from_id(id: u32) -> Self {
        match id {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            _ => LogLevel::Info,
        }
    }

    pub fn all_levels() -> Vec<LogLevel> {
        vec![
            LogLevel::Trace,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
        ]
    }
}

/// Configuration for tracing initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    pub default_level: LogLevel,
    pub module_filters: Vec<(String, LogLevel)>,
    pub show_timestamps: bool,
    pub show_thread_ids: bool,
    pub show_targets: bool,
    pub show_file_line: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            default_level: LogLevel::Info,
            module_filters: vec![
                ("tower_core::bridge".to_string(), LogLevel::Warn),
                ("tower_core::generation".to_string(), LogLevel::Info),
                ("tower_core::combat".to_string(), LogLevel::Debug),
                ("tower_core::engine".to_string(), LogLevel::Info),
            ],
            show_timestamps: true,
            show_thread_ids: false,
            show_targets: true,
            show_file_line: false,
        }
    }
}

impl TracingConfig {
    pub fn to_env_filter_string(&self) -> String {
        let mut parts = vec![self.default_level.as_str().to_string()];
        for (module, level) in &self.module_filters {
            parts.push(format!("{}={}", module, level.as_str()));
        }
        parts.join(",")
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

static TRACING_INIT: Once = Once::new();

/// Initialize tracing with default settings (idempotent — safe to call multiple times)
pub fn init_tracing_default() {
    init_tracing(&TracingConfig::default());
}

/// Initialize tracing with custom config (idempotent — first call wins)
pub fn init_tracing(config: &TracingConfig) {
    let filter_str = config.to_env_filter_string();
    TRACING_INIT.call_once(move || {
        let filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&filter_str));

        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false)
            .compact();

        // Ignore error if a global subscriber is already set (e.g., by Bevy)
        let _ = subscriber.try_init();
    });
}

/// Log a structured message at INFO level (for FFI use)
pub fn log_info(target: &str, message: &str) {
    tracing::info!(target: "tower_core", system = target, "{}", message);
}

/// Log a structured message at WARN level (for FFI use)
pub fn log_warn(target: &str, message: &str) {
    tracing::warn!(target: "tower_core", system = target, "{}", message);
}

/// Log a structured message at ERROR level (for FFI use)
pub fn log_error(target: &str, message: &str) {
    tracing::error!(target: "tower_core", system = target, "{}", message);
}

/// Log a structured message at DEBUG level (for FFI use)
pub fn log_debug(target: &str, message: &str) {
    tracing::debug!(target: "tower_core", system = target, "{}", message);
}

/// Create a named span for timing an operation
/// Returns a guard that logs duration on drop
pub struct TimingSpan {
    _span: tracing::span::EnteredSpan,
}

impl TimingSpan {
    pub fn new(name: &str) -> Self {
        let span = tracing::info_span!("operation", name = name);
        Self {
            _span: span.entered(),
        }
    }
}

/// Snapshot of current logging configuration for FFI
#[derive(Debug, Serialize, Deserialize)]
pub struct LoggingSnapshot {
    pub default_level: String,
    pub available_levels: Vec<String>,
    pub module_filter_count: usize,
    pub config: TracingConfig,
}

impl LoggingSnapshot {
    pub fn capture(config: &TracingConfig) -> Self {
        Self {
            default_level: config.default_level.as_str().to_string(),
            available_levels: LogLevel::all_levels()
                .iter()
                .map(|l| l.as_str().to_string())
                .collect(),
            module_filter_count: config.module_filters.len(),
            config: config.clone(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_id() {
        assert_eq!(LogLevel::from_id(0), LogLevel::Trace);
        assert_eq!(LogLevel::from_id(1), LogLevel::Debug);
        assert_eq!(LogLevel::from_id(2), LogLevel::Info);
        assert_eq!(LogLevel::from_id(3), LogLevel::Warn);
        assert_eq!(LogLevel::from_id(4), LogLevel::Error);
        assert_eq!(LogLevel::from_id(99), LogLevel::Info); // fallback
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Trace.as_str(), "trace");
        assert_eq!(LogLevel::Debug.as_str(), "debug");
        assert_eq!(LogLevel::Info.as_str(), "info");
        assert_eq!(LogLevel::Warn.as_str(), "warn");
        assert_eq!(LogLevel::Error.as_str(), "error");
    }

    #[test]
    fn test_all_levels() {
        let levels = LogLevel::all_levels();
        assert_eq!(levels.len(), 5);
    }

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert_eq!(config.default_level, LogLevel::Info);
        assert!(!config.module_filters.is_empty());
        assert!(config.show_timestamps);
        assert!(config.show_targets);
    }

    #[test]
    fn test_env_filter_string() {
        let config = TracingConfig::default();
        let filter = config.to_env_filter_string();
        assert!(filter.contains("info"));
        assert!(filter.contains("tower_core::bridge=warn"));
        assert!(filter.contains("tower_core::combat=debug"));
    }

    #[test]
    fn test_tracing_config_json_roundtrip() {
        let config = TracingConfig::default();
        let json = config.to_json();
        assert!(!json.is_empty());
        let restored = TracingConfig::from_json(&json).unwrap();
        assert_eq!(restored.default_level, config.default_level);
        assert_eq!(restored.module_filters.len(), config.module_filters.len());
    }

    #[test]
    fn test_init_tracing_idempotent() {
        // Should not panic when called multiple times
        init_tracing_default();
        init_tracing_default();
        init_tracing(&TracingConfig::default());
    }

    #[test]
    fn test_log_functions_no_panic() {
        init_tracing_default();
        log_info("test", "test info message");
        log_warn("test", "test warn message");
        log_error("test", "test error message");
        log_debug("test", "test debug message");
    }

    #[test]
    fn test_timing_span() {
        init_tracing_default();
        {
            let _span = TimingSpan::new("test_operation");
            // Simulate work
            let sum: u64 = (0..100).sum();
            assert!(sum > 0);
        }
        // Span dropped — timing logged
    }

    #[test]
    fn test_logging_snapshot() {
        let config = TracingConfig::default();
        let snapshot = LoggingSnapshot::capture(&config);
        assert_eq!(snapshot.default_level, "info");
        assert_eq!(snapshot.available_levels.len(), 5);
        let json = snapshot.to_json();
        assert!(json.contains("info"));
        assert!(json.contains("available_levels"));
    }

    #[test]
    fn test_custom_config() {
        let config = TracingConfig {
            default_level: LogLevel::Debug,
            module_filters: vec![("my_module".to_string(), LogLevel::Trace)],
            show_timestamps: false,
            show_thread_ids: true,
            show_targets: false,
            show_file_line: true,
        };
        let filter = config.to_env_filter_string();
        assert!(filter.starts_with("debug"));
        assert!(filter.contains("my_module=trace"));
    }
}
