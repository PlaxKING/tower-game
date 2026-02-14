//! Save File Migration System (IMP-016)
//!
//! Handles versioned save files with forward migration:
//! - Each save has a `version` field
//! - Migration functions transform v(N) → v(N+1) → ... → v(current)
//! - Old saves are never lost — always migrated forward
//! - Unknown future versions produce an error (no downgrade)

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current save format version
pub const CURRENT_SAVE_VERSION: u32 = 3;

/// Minimum supported version (anything below cannot be migrated)
pub const MIN_SUPPORTED_VERSION: u32 = 1;

/// Error types for migration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MigrationError {
    /// Save version is newer than what we support (can't downgrade)
    FutureVersion {
        save_version: u32,
        max_supported: u32,
    },
    /// Save version is too old (below minimum)
    TooOldVersion {
        save_version: u32,
        min_supported: u32,
    },
    /// JSON parsing failed
    InvalidFormat { detail: String },
    /// A specific migration step failed
    MigrationStepFailed { from_version: u32, detail: String },
}

/// Result of a migration attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub success: bool,
    pub original_version: u32,
    pub final_version: u32,
    pub steps_applied: Vec<String>,
    pub error: Option<MigrationError>,
    pub data: Option<Value>,
}

/// Header present in all save files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveHeader {
    pub version: u32,
    pub created_at: String,
    pub last_modified: String,
    pub player_name: String,
}

/// Migrate a save file from its current version to CURRENT_SAVE_VERSION
pub fn migrate_save(json_str: &str) -> MigrationResult {
    // Parse JSON
    let mut data: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            return MigrationResult {
                success: false,
                original_version: 0,
                final_version: 0,
                steps_applied: vec![],
                error: Some(MigrationError::InvalidFormat {
                    detail: e.to_string(),
                }),
                data: None,
            };
        }
    };

    // Extract version
    let version = data.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    if version == 0 {
        return MigrationResult {
            success: false,
            original_version: 0,
            final_version: 0,
            steps_applied: vec![],
            error: Some(MigrationError::InvalidFormat {
                detail: "Missing or invalid 'version' field".to_string(),
            }),
            data: None,
        };
    }

    // Check for future version
    if version > CURRENT_SAVE_VERSION {
        return MigrationResult {
            success: false,
            original_version: version,
            final_version: version,
            steps_applied: vec![],
            error: Some(MigrationError::FutureVersion {
                save_version: version,
                max_supported: CURRENT_SAVE_VERSION,
            }),
            data: None,
        };
    }

    // Check for too-old version
    if version < MIN_SUPPORTED_VERSION {
        return MigrationResult {
            success: false,
            original_version: version,
            final_version: version,
            steps_applied: vec![],
            error: Some(MigrationError::TooOldVersion {
                save_version: version,
                min_supported: MIN_SUPPORTED_VERSION,
            }),
            data: None,
        };
    }

    // Already current
    if version == CURRENT_SAVE_VERSION {
        return MigrationResult {
            success: true,
            original_version: version,
            final_version: version,
            steps_applied: vec!["No migration needed".to_string()],
            error: None,
            data: Some(data),
        };
    }

    // Apply migrations sequentially
    let mut current_version = version;
    let mut steps = Vec::new();

    while current_version < CURRENT_SAVE_VERSION {
        let result = apply_migration_step(&mut data, current_version);
        match result {
            Ok(description) => {
                steps.push(description);
                current_version += 1;
                data["version"] = serde_json::json!(current_version);
            }
            Err(detail) => {
                return MigrationResult {
                    success: false,
                    original_version: version,
                    final_version: current_version,
                    steps_applied: steps,
                    error: Some(MigrationError::MigrationStepFailed {
                        from_version: current_version,
                        detail,
                    }),
                    data: None,
                };
            }
        }
    }

    MigrationResult {
        success: true,
        original_version: version,
        final_version: current_version,
        steps_applied: steps,
        error: None,
        data: Some(data),
    }
}

/// Apply a single migration step from `from_version` to `from_version + 1`
fn apply_migration_step(data: &mut Value, from_version: u32) -> Result<String, String> {
    match from_version {
        1 => migrate_v1_to_v2(data),
        2 => migrate_v2_to_v3(data),
        _ => Err(format!("No migration path from version {}", from_version)),
    }
}

/// Migration v1 → v2:
/// - Added `mastery` section with empty profile
/// - Renamed `player_level` → removed (mastery replaces levels)
/// - Added `specialization` section
/// - Added `equipped_cosmetics` array
fn migrate_v1_to_v2(data: &mut Value) -> Result<String, String> {
    let obj = data.as_object_mut().ok_or("Save data is not an object")?;

    // Remove deprecated player_level field
    obj.remove("player_level");

    // Add mastery section if missing
    if !obj.contains_key("mastery") {
        obj.insert(
            "mastery".to_string(),
            serde_json::json!({
                "domains": {},
                "total_xp": 0
            }),
        );
    }

    // Add specialization section if missing
    if !obj.contains_key("specialization") {
        obj.insert(
            "specialization".to_string(),
            serde_json::json!({
                "chosen_branches": [],
                "active_synergies": []
            }),
        );
    }

    // Add cosmetics if missing
    if !obj.contains_key("equipped_cosmetics") {
        obj.insert("equipped_cosmetics".to_string(), serde_json::json!([]));
    }

    Ok("v1→v2: Added mastery, specialization, cosmetics; removed player_level".to_string())
}

/// Migration v2 → v3:
/// - Added `mutator_history` tracking completed mutator challenges
/// - Added `game_flow_state` field
/// - Added `achievements_v2` with new achievement format (categories)
/// - Renamed `inventory.items` entries to include `semantic_tags` field
/// - Added `socket_data` field to equipment entries
fn migrate_v2_to_v3(data: &mut Value) -> Result<String, String> {
    let obj = data.as_object_mut().ok_or("Save data is not an object")?;

    // Add mutator history
    if !obj.contains_key("mutator_history") {
        obj.insert(
            "mutator_history".to_string(),
            serde_json::json!({
                "completed_mutators": [],
                "highest_difficulty_cleared": 0,
                "total_mutator_floors_cleared": 0
            }),
        );
    }

    // Add game flow state
    if !obj.contains_key("game_flow_state") {
        obj.insert("game_flow_state".to_string(), serde_json::json!("MainMenu"));
    }

    // Migrate achievements to v2 format (add categories if missing)
    if let Some(achievements) = obj.get("achievements").cloned() {
        if achievements.is_array() {
            // Convert flat array to categorized format
            let categorized = serde_json::json!({
                "format": "v2",
                "entries": achievements,
                "categories": {}
            });
            obj.insert("achievements".to_string(), categorized);
        }
    } else {
        obj.insert(
            "achievements".to_string(),
            serde_json::json!({
                "format": "v2",
                "entries": [],
                "categories": {}
            }),
        );
    }

    // Add semantic_tags to inventory items if missing
    if let Some(inventory) = obj.get_mut("inventory") {
        if let Some(items) = inventory.get_mut("items") {
            if let Some(items_array) = items.as_array_mut() {
                for item in items_array.iter_mut() {
                    if let Some(item_obj) = item.as_object_mut() {
                        if !item_obj.contains_key("semantic_tags") {
                            item_obj.insert("semantic_tags".to_string(), serde_json::json!([]));
                        }
                        if !item_obj.contains_key("socket_data") {
                            item_obj.insert("socket_data".to_string(), serde_json::json!(null));
                        }
                    }
                }
            }
        }
    }

    Ok("v2→v3: Added mutator_history, game_flow_state, achievements_v2, item semantic_tags/socket_data".to_string())
}

/// Validate that a save file is at the current version
pub fn validate_save(json_str: &str) -> bool {
    let data: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => return false,
    };
    data.get("version")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32 == CURRENT_SAVE_VERSION)
        .unwrap_or(false)
}

/// Create a new empty save file at the current version
pub fn create_new_save(player_name: &str) -> Value {
    serde_json::json!({
        "version": CURRENT_SAVE_VERSION,
        "created_at": "2026-02-14T00:00:00Z",
        "last_modified": "2026-02-14T00:00:00Z",
        "player_name": player_name,
        "mastery": {
            "domains": {},
            "total_xp": 0
        },
        "specialization": {
            "chosen_branches": [],
            "active_synergies": []
        },
        "equipped_cosmetics": [],
        "inventory": {
            "items": [],
            "shards": 0,
            "echo_fragments": 0
        },
        "mutator_history": {
            "completed_mutators": [],
            "highest_difficulty_cleared": 0,
            "total_mutator_floors_cleared": 0
        },
        "game_flow_state": "MainMenu",
        "achievements": {
            "format": "v2",
            "entries": [],
            "categories": {}
        },
        "stats": {
            "highest_floor": 0,
            "total_monsters_slain": 0,
            "total_deaths": 0,
            "total_play_time_secs": 0.0,
            "total_damage_dealt": 0.0,
            "total_shards_earned": 0
        },
        "settings": {
            "master_volume": 1.0,
            "sfx_volume": 1.0,
            "music_volume": 0.7,
            "mouse_sensitivity": 1.0,
            "invert_y": false,
            "show_damage_numbers": true,
            "minimap_rotation": true
        }
    })
}

/// Get the save version from a JSON string without fully parsing
pub fn get_save_version(json_str: &str) -> Option<u32> {
    let data: Value = serde_json::from_str(json_str).ok()?;
    data.get("version")?.as_u64().map(|v| v as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_v1_save() -> String {
        serde_json::json!({
            "version": 1,
            "player_name": "TestPlayer",
            "player_level": 15,
            "inventory": {
                "items": [
                    {"name": "Fire Sword", "rarity": "Rare", "quantity": 1},
                    {"name": "Health Potion", "rarity": "Common", "quantity": 5}
                ],
                "shards": 1000,
                "echo_fragments": 50
            },
            "achievements": [
                {"id": "monster_slayer_1", "progress": 50}
            ],
            "stats": {
                "highest_floor": 25,
                "total_monsters_slain": 500
            }
        })
        .to_string()
    }

    fn make_v2_save() -> String {
        serde_json::json!({
            "version": 2,
            "player_name": "TestPlayer",
            "mastery": {
                "domains": {"SwordMastery": 1500},
                "total_xp": 1500
            },
            "specialization": {
                "chosen_branches": ["sword_berserker"],
                "active_synergies": []
            },
            "equipped_cosmetics": ["flame_aura"],
            "inventory": {
                "items": [
                    {"name": "Fire Sword", "rarity": "Rare", "quantity": 1}
                ],
                "shards": 2000
            },
            "achievements": [
                {"id": "monster_slayer_1", "progress": 100, "unlocked": true}
            ]
        })
        .to_string()
    }

    fn make_v3_save() -> String {
        create_new_save("TestPlayer").to_string()
    }

    #[test]
    fn test_current_version_no_migration() {
        let save = make_v3_save();
        let result = migrate_save(&save);
        assert!(result.success);
        assert_eq!(result.original_version, 3);
        assert_eq!(result.final_version, 3);
        assert_eq!(result.steps_applied.len(), 1);
        assert!(result.steps_applied[0].contains("No migration"));
    }

    #[test]
    fn test_migrate_v1_to_v3() {
        let save = make_v1_save();
        let result = migrate_save(&save);
        assert!(result.success);
        assert_eq!(result.original_version, 1);
        assert_eq!(result.final_version, 3);
        assert_eq!(result.steps_applied.len(), 2);

        let data = result.data.unwrap();
        // v1→v2: player_level removed
        assert!(data.get("player_level").is_none());
        // v1→v2: mastery added
        assert!(data.get("mastery").is_some());
        // v1→v2: specialization added
        assert!(data.get("specialization").is_some());
        // v2→v3: mutator_history added
        assert!(data.get("mutator_history").is_some());
        // v2→v3: game_flow_state added
        assert!(data.get("game_flow_state").is_some());
        // v2→v3: items have semantic_tags
        let items = data["inventory"]["items"].as_array().unwrap();
        for item in items {
            assert!(item.get("semantic_tags").is_some());
            assert!(item.get("socket_data").is_some());
        }
        // Version updated
        assert_eq!(data["version"].as_u64().unwrap(), 3);
    }

    #[test]
    fn test_migrate_v2_to_v3() {
        let save = make_v2_save();
        let result = migrate_save(&save);
        assert!(result.success);
        assert_eq!(result.original_version, 2);
        assert_eq!(result.final_version, 3);
        assert_eq!(result.steps_applied.len(), 1);

        let data = result.data.unwrap();
        assert!(data.get("mutator_history").is_some());
        assert!(data.get("game_flow_state").is_some());
        // Achievements migrated to v2 format
        assert_eq!(data["achievements"]["format"], "v2");
    }

    #[test]
    fn test_future_version_rejected() {
        let save = serde_json::json!({"version": 999}).to_string();
        let result = migrate_save(&save);
        assert!(!result.success);
        assert!(matches!(
            result.error,
            Some(MigrationError::FutureVersion { .. })
        ));
    }

    #[test]
    fn test_invalid_json() {
        let result = migrate_save("not json at all");
        assert!(!result.success);
        assert!(matches!(
            result.error,
            Some(MigrationError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn test_missing_version() {
        let save = serde_json::json!({"player_name": "Test"}).to_string();
        let result = migrate_save(&save);
        assert!(!result.success);
        assert!(matches!(
            result.error,
            Some(MigrationError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn test_validate_save_current() {
        let save = make_v3_save();
        assert!(validate_save(&save));
    }

    #[test]
    fn test_validate_save_old() {
        let save = make_v1_save();
        assert!(!validate_save(&save));
    }

    #[test]
    fn test_validate_save_invalid() {
        assert!(!validate_save("garbage"));
    }

    #[test]
    fn test_create_new_save() {
        let save = create_new_save("HeroPlayer");
        assert_eq!(
            save["version"].as_u64().unwrap(),
            CURRENT_SAVE_VERSION as u64
        );
        assert_eq!(save["player_name"], "HeroPlayer");
        assert!(save["mastery"].is_object());
        assert!(save["inventory"]["items"].is_array());
        assert!(save["mutator_history"].is_object());
        assert!(save["settings"].is_object());
    }

    #[test]
    fn test_get_save_version() {
        assert_eq!(get_save_version(&make_v1_save()), Some(1));
        assert_eq!(get_save_version(&make_v2_save()), Some(2));
        assert_eq!(get_save_version(&make_v3_save()), Some(3));
        assert_eq!(get_save_version("garbage"), None);
    }

    #[test]
    fn test_migration_preserves_existing_data() {
        let save = make_v1_save();
        let result = migrate_save(&save);
        assert!(result.success);
        let data = result.data.unwrap();
        // Original data preserved
        assert_eq!(data["player_name"], "TestPlayer");
        assert_eq!(data["inventory"]["shards"].as_u64().unwrap(), 1000);
        assert_eq!(data["stats"]["highest_floor"].as_u64().unwrap(), 25);
    }

    #[test]
    fn test_migration_result_serialization() {
        let result = migrate_save(&make_v1_save());
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: MigrationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.success, result.success);
        assert_eq!(deserialized.original_version, result.original_version);
        assert_eq!(deserialized.final_version, result.final_version);
    }

    #[test]
    fn test_migration_error_serialization() {
        let err = MigrationError::FutureVersion {
            save_version: 99,
            max_supported: 3,
        };
        let json = serde_json::to_string(&err).unwrap();
        let deserialized: MigrationError = serde_json::from_str(&json).unwrap();
        assert_eq!(err, deserialized);
    }

    #[test]
    fn test_idempotent_migration() {
        // Migrating an already-current save should not change it
        let save = make_v3_save();
        let result1 = migrate_save(&save);
        let data1 = result1.data.unwrap();
        let result2 = migrate_save(&serde_json::to_string(&data1).unwrap());
        assert!(result2.success);
        assert_eq!(result2.original_version, 3);
    }

    #[test]
    fn test_empty_inventory_migration() {
        let save = serde_json::json!({
            "version": 2,
            "player_name": "EmptyPlayer",
            "mastery": {"domains": {}, "total_xp": 0},
            "specialization": {"chosen_branches": [], "active_synergies": []},
            "equipped_cosmetics": [],
            "inventory": {"items": []}
        })
        .to_string();
        let result = migrate_save(&save);
        assert!(result.success);
        assert_eq!(result.final_version, 3);
    }
}
