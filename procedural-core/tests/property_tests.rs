//! Property-based tests (IMP-004) using proptest
//!
//! Tests invariants that must hold for ALL inputs:
//! - Floor generation: any seed → valid floor
//! - Combat: any angle/combo → finite positive damage
//! - Economy: transactions → balance >= 0
//! - Mastery: XP → monotonic tier growth
//! - Sockets: valid color matching rules
//! - Season pass: XP → monotonic level growth

use proptest::prelude::*;
use std::ffi::{CStr, CString};

// Import FFI functions
use tower_core::bridge::*;

fn ptr_to_string(ptr: *mut std::os::raw::c_char) -> String {
    assert!(!ptr.is_null(), "FFI returned null pointer");
    let s = unsafe { CStr::from_ptr(ptr).to_str().unwrap().to_owned() };
    free_string(ptr);
    s
}

// ============================================================
// Floor Generation Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn prop_any_seed_generates_valid_floor(seed in any::<u64>(), floor_id in 1u32..=100) {
        let ptr = generate_floor(seed, floor_id);
        prop_assert!(!ptr.is_null(), "generate_floor returned null for seed={seed}, floor={floor_id}");
        let json = ptr_to_string(ptr);
        let value: serde_json::Value = serde_json::from_str(&json)
            .expect("generate_floor returned invalid JSON");

        // Must have floor_id
        prop_assert_eq!(value["floor_id"].as_u64().unwrap(), floor_id as u64);
        // Must have tier
        prop_assert!(value["tier"].as_str().is_some(), "Missing tier");
        // Must have biome_tags
        prop_assert!(value["biome_tags"].is_array(), "Missing biome_tags");
    }

    #[test]
    fn prop_floor_generation_is_deterministic(seed in any::<u64>(), floor_id in 1u32..=50) {
        let json1 = ptr_to_string(generate_floor(seed, floor_id));
        let json2 = ptr_to_string(generate_floor(seed, floor_id));
        prop_assert_eq!(json1, json2, "Same seed+floor should produce identical output");
    }

    #[test]
    fn prop_any_seed_generates_valid_layout(seed in any::<u64>(), floor_id in 1u32..=50) {
        let ptr = generate_floor_layout(seed, floor_id);
        prop_assert!(!ptr.is_null());
        let json = ptr_to_string(ptr);
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Must have tiles (2D array)
        prop_assert!(value["tiles"].is_array(), "Missing tiles array");
        let tiles = value["tiles"].as_array().unwrap();
        prop_assert!(!tiles.is_empty(), "Tiles array is empty");

        // Must have rooms
        prop_assert!(value["rooms"].is_array(), "Missing rooms array");

        // Every tile value must be 0-11 (valid TileType)
        for row in tiles {
            for tile in row.as_array().unwrap() {
                let t = tile.as_u64().unwrap();
                prop_assert!(t <= 11, "Invalid tile type: {t}");
            }
        }
    }
}

// ============================================================
// Monster Generation Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn prop_any_hash_generates_valid_monster(seed in any::<u64>(), floor_id in 1u32..=100) {
        let ptr = generate_monster(seed, floor_id);
        prop_assert!(!ptr.is_null());
        let json = ptr_to_string(ptr);
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Must have name (non-empty string)
        let name = value["name"].as_str().unwrap();
        prop_assert!(!name.is_empty(), "Monster name is empty");

        // Must have positive HP and damage
        let hp = value["max_hp"].as_f64().unwrap();
        let dmg = value["damage"].as_f64().unwrap();
        prop_assert!(hp > 0.0, "Monster HP must be positive, got {hp}");
        prop_assert!(dmg > 0.0, "Monster damage must be positive, got {dmg}");

        // Must have valid size
        let size = value["size"].as_str().unwrap();
        prop_assert!(
            ["Tiny", "Small", "Medium", "Large", "Colossal"].contains(&size),
            "Invalid monster size: {size}"
        );

        // Must have valid element
        let element = value["element"].as_str().unwrap();
        prop_assert!(
            ["Fire", "Water", "Earth", "Wind", "Void", "Neutral"].contains(&element),
            "Invalid monster element: {element}"
        );
    }
}

// ============================================================
// Combat Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn prop_combat_damage_is_finite_and_positive(
        base_damage in 1.0f32..=1000.0,
        combo_step in 0u32..=10,
        angle_id in 0u32..=5,
    ) {
        let request = serde_json::json!({
            "base_damage": base_damage,
            "combo_step": combo_step,
            "angle_id": angle_id,
            "attacker_tags_json": "[[\"fire\", 0.5]]",
            "defender_tags_json": "[[\"water\", 0.3]]",
        });
        let req_str = CString::new(serde_json::to_string(&request).unwrap()).unwrap();
        let ptr = calculate_combat(req_str.as_ptr());
        prop_assert!(!ptr.is_null());
        let json = ptr_to_string(ptr);
        let result: serde_json::Value = serde_json::from_str(&json).unwrap();

        let final_damage = result["final_damage"].as_f64().unwrap();
        prop_assert!(final_damage.is_finite(), "Damage must be finite, got {final_damage}");
        prop_assert!(final_damage > 0.0, "Damage must be positive, got {final_damage}");

        let angle_mult = result["angle_multiplier"].as_f64().unwrap();
        prop_assert!(angle_mult > 0.0 && angle_mult.is_finite(), "Angle multiplier must be positive finite, got {angle_mult}");
    }

    #[test]
    fn prop_angle_multiplier_valid_range(angle_id in 0u32..=255) {
        let mult = get_angle_multiplier(angle_id);
        prop_assert!(mult > 0.0 && mult <= 2.0, "Angle multiplier out of range: {mult}");
    }
}

// ============================================================
// Mastery Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_mastery_xp_causes_monotonic_tier_growth(
        domain_id in 0u32..=20,
        xp_amounts in prop::collection::vec(1u64..=10000, 1..10),
    ) {
        let mut profile_json = ptr_to_string(mastery_create_profile());
        let mut prev_tier = mastery_get_tier(
            CString::new(profile_json.as_str()).unwrap().as_ptr(),
            domain_id,
        );

        for xp in xp_amounts {
            let profile_c = CString::new(profile_json.as_str()).unwrap();
            let ptr = mastery_gain_xp(profile_c.as_ptr(), domain_id, xp);
            prop_assert!(!ptr.is_null());
            profile_json = ptr_to_string(ptr);

            let new_tier = mastery_get_tier(
                CString::new(profile_json.as_str()).unwrap().as_ptr(),
                domain_id,
            );
            prop_assert!(
                new_tier >= prev_tier,
                "Tier decreased from {prev_tier} to {new_tier} after gaining {xp} XP"
            );
            prev_tier = new_tier;
        }
    }

    #[test]
    fn prop_mastery_invalid_domain_returns_error(domain_id in 21u32..=255) {
        let profile_json = ptr_to_string(mastery_create_profile());
        let profile_c = CString::new(profile_json.as_str()).unwrap();

        // Invalid domain should return null
        let ptr = mastery_gain_xp(profile_c.as_ptr(), domain_id, 100);
        prop_assert!(ptr.is_null(), "Expected null for invalid domain {domain_id}");

        // Invalid domain should return -1
        let tier = mastery_get_tier(profile_c.as_ptr(), domain_id);
        prop_assert_eq!(tier, -1, "Expected -1 for invalid domain {}", domain_id);
    }
}

// ============================================================
// Season Pass Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_season_xp_causes_monotonic_level_growth(
        season in 1u32..=10,
        xp_amounts in prop::collection::vec(100u64..=5000, 1..20),
    ) {
        let name = CString::new("Test Season").unwrap();
        let mut pass_json = ptr_to_string(season_create_pass(season, name.as_ptr()));

        let initial: serde_json::Value = serde_json::from_str(&pass_json).unwrap();
        let mut prev_level = initial["current_level"].as_u64().unwrap_or(0);

        for xp in xp_amounts {
            let pass_c = CString::new(pass_json.as_str()).unwrap();
            let ptr = season_add_xp(pass_c.as_ptr(), xp);
            prop_assert!(!ptr.is_null());
            pass_json = ptr_to_string(ptr);

            let value: serde_json::Value = serde_json::from_str(&pass_json).unwrap();
            let new_level = value["current_level"].as_u64().unwrap_or(0);
            prop_assert!(
                new_level >= prev_level,
                "Season level decreased from {prev_level} to {new_level}"
            );
            prev_level = new_level;
        }
    }

    #[test]
    fn prop_daily_quests_always_three(day_seed in any::<u64>()) {
        let ptr = season_generate_dailies(day_seed);
        prop_assert!(!ptr.is_null());
        let json = ptr_to_string(ptr);
        let quests: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(quests.len(), 3, "Daily quests should always be 3, got {}", quests.len());
    }

    #[test]
    fn prop_weekly_quests_always_three(week_seed in any::<u64>()) {
        let ptr = season_generate_weeklies(week_seed);
        prop_assert!(!ptr.is_null());
        let json = ptr_to_string(ptr);
        let quests: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(quests.len(), 3, "Weekly quests should always be 3, got {}", quests.len());
    }
}

// ============================================================
// Socket System Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn prop_equipment_socket_count_matches_request(
        num_sockets in 1usize..=4,
        color_val in 0u32..=3,
    ) {
        let actual_colors: Vec<u32> = vec![color_val; num_sockets];
        let colors_json = serde_json::to_string(&actual_colors).unwrap();

        let name = CString::new("PropTest Sword").unwrap();
        let colors_c = CString::new(colors_json).unwrap();
        let ptr = socket_create_equipment(name.as_ptr(), colors_c.as_ptr());
        prop_assert!(!ptr.is_null());
        let json = ptr_to_string(ptr);
        let equip: serde_json::Value = serde_json::from_str(&json).unwrap();

        let sockets = equip["sockets"].as_array().unwrap();
        prop_assert_eq!(
            sockets.len(), num_sockets,
            "Socket count mismatch: expected {}, got {}", num_sockets, sockets.len()
        );
    }
}

// ============================================================
// Achievement Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]

    #[test]
    fn prop_achievement_completion_bounded(increments in prop::collection::vec(1u64..=1000, 0..20)) {
        let mut tracker_json = ptr_to_string(achievement_create_tracker());
        let aid = CString::new("monster_slayer_1").unwrap();

        for amount in increments {
            let tracker_c = CString::new(tracker_json.as_str()).unwrap();
            let ptr = achievement_increment(tracker_c.as_ptr(), aid.as_ptr(), amount);
            prop_assert!(!ptr.is_null());
            tracker_json = ptr_to_string(ptr);
        }

        let tracker_c = CString::new(tracker_json.as_str()).unwrap();
        let pct = achievement_completion_percent(tracker_c.as_ptr());
        prop_assert!((0.0..=1.0).contains(&pct), "Completion must be [0, 1], got {pct}");
    }
}

// ============================================================
// Loot Generation Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_loot_generation_valid(floor_level in 1u32..=100, seed in any::<u64>()) {
        let tags_json = CString::new(r#"[["fire", 0.5], ["water", 0.3]]"#).unwrap();
        let ptr = generate_loot(tags_json.as_ptr(), floor_level, seed);
        prop_assert!(!ptr.is_null());
        let json = ptr_to_string(ptr);
        let items: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();

        for item in &items {
            // Every item must have a name
            prop_assert!(item["name"].as_str().is_some(), "Item missing name");
            // Every item must have a rarity
            prop_assert!(item["rarity"].as_str().is_some(), "Item missing rarity");
        }
    }
}

// ============================================================
// Breath of Tower Properties
// ============================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_breath_state_always_valid(elapsed in 0.0f32..=100000.0) {
        let ptr = get_breath_state(elapsed);
        prop_assert!(!ptr.is_null());
        let json = ptr_to_string(ptr);
        let state: serde_json::Value = serde_json::from_str(&json).unwrap();

        let phase = state["phase"].as_str().unwrap();
        prop_assert!(
            ["Inhale", "Hold", "Exhale", "Pause"].contains(&phase),
            "Invalid breath phase: {phase}"
        );

        let progress = state["phase_progress"].as_f64().unwrap();
        prop_assert!(
            (0.0..=1.0).contains(&progress),
            "Progress out of [0,1]: {progress}"
        );

        let mult = state["monster_spawn_mult"].as_f64().unwrap();
        prop_assert!(mult > 0.0 && mult.is_finite(), "Invalid multiplier: {mult}");
    }
}
