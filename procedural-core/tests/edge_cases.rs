//! Edge case & boundary tests (TD-003)
//!
//! Tests behavior at system boundaries:
//! - Null pointer inputs → should return null or default, never crash
//! - Empty / malformed JSON → graceful error handling
//! - Maximum values (u64::MAX, u32::MAX, very large floats)
//! - Zero / minimum boundary values
//! - Double-free safety (free_string on null)
//! - Invalid IDs, unknown domains, out-of-range enums

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use tower_core::bridge::*;

// ============================================================
// Helpers
// ============================================================

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn ptr_to_string(ptr: *mut c_char) -> String {
    assert!(!ptr.is_null(), "FFI returned null pointer");
    let s = unsafe { CStr::from_ptr(ptr).to_str().unwrap().to_owned() };
    free_string(ptr);
    s
}

fn is_valid_json(ptr: *mut c_char) -> bool {
    if ptr.is_null() {
        return false;
    }
    let s = unsafe { CStr::from_ptr(ptr).to_str().unwrap().to_owned() };
    free_string(ptr);
    serde_json::from_str::<serde_json::Value>(&s).is_ok()
}

// ============================================================
// 1. Null pointer safety — functions accepting *const c_char
// ============================================================

#[test]
fn null_input_mastery_gain_xp() {
    let result = mastery_gain_xp(std::ptr::null(), 0, 100);
    assert!(
        result.is_null(),
        "mastery_gain_xp(null, ...) should return null"
    );
}

#[test]
fn null_input_mastery_get_tier() {
    let tier = mastery_get_tier(std::ptr::null(), 0);
    assert_eq!(tier, -1, "mastery_get_tier(null, ...) should return -1");
}

#[test]
fn null_input_mastery_xp_for_action() {
    let xp = mastery_xp_for_action(std::ptr::null());
    assert_eq!(xp, 0, "mastery_xp_for_action(null) should return 0");
}

#[test]
fn null_input_calculate_combat() {
    let result = calculate_combat(std::ptr::null());
    assert!(
        result.is_null(),
        "calculate_combat(null) should return null"
    );
}

#[test]
fn null_input_semantic_similarity() {
    let result = semantic_similarity(std::ptr::null(), std::ptr::null());
    assert_eq!(
        result, 0.0,
        "semantic_similarity(null, null) should return 0.0"
    );

    let tags = cstr(r#"[["fire", 0.5]]"#);
    let result2 = semantic_similarity(tags.as_ptr(), std::ptr::null());
    assert_eq!(
        result2, 0.0,
        "semantic_similarity(valid, null) should return 0.0"
    );

    let result3 = semantic_similarity(std::ptr::null(), tags.as_ptr());
    assert_eq!(
        result3, 0.0,
        "semantic_similarity(null, valid) should return 0.0"
    );
}

#[test]
fn null_input_generate_loot() {
    let result = generate_loot(std::ptr::null(), 10, 42);
    assert!(
        result.is_null(),
        "generate_loot(null, ...) should return null"
    );
}

#[test]
fn null_input_record_delta() {
    // null player_id — uses unwrap_or_default(), returns valid result with empty player
    let payload = cstr(r#"{"action":"test"}"#);
    let result = record_delta(0, 1, 42, std::ptr::null(), payload.as_ptr(), 1);
    assert!(
        !result.is_null(),
        "record_delta gracefully handles null player (defaults to empty)"
    );
    assert!(is_valid_json(result));

    // null payload — uses unwrap_or_default(), returns valid result with empty payload
    let player = cstr("player1");
    let result2 = record_delta(0, 1, 42, player.as_ptr(), std::ptr::null(), 1);
    assert!(
        !result2.is_null(),
        "record_delta gracefully handles null payload (defaults to empty)"
    );
    assert!(is_valid_json(result2));
}

#[test]
fn null_input_create_floor_snapshot() {
    let result = create_floor_snapshot(42, 1, std::ptr::null());
    assert!(
        result.is_null(),
        "create_floor_snapshot(null deltas) should return null"
    );
}

#[test]
fn null_input_evaluate_event_trigger() {
    let result = evaluate_event_trigger(0, std::ptr::null());
    assert!(
        result.is_null(),
        "evaluate_event_trigger(null context) should return null"
    );
}

#[test]
fn null_input_spec_choose_branch() {
    let result = spec_choose_branch(std::ptr::null(), std::ptr::null(), std::ptr::null());
    assert!(
        result.is_null(),
        "spec_choose_branch(null, null, null) should return null"
    );
}

#[test]
fn null_input_spec_find_synergies() {
    let result = spec_find_synergies(std::ptr::null());
    assert!(
        result.is_null(),
        "spec_find_synergies(null) should return null"
    );
}

#[test]
fn null_input_ability_learn() {
    let result = ability_learn(std::ptr::null(), std::ptr::null());
    assert!(
        result.is_null(),
        "ability_learn(null, null) should return null"
    );
}

#[test]
fn null_input_ability_equip() {
    let result = ability_equip(std::ptr::null(), 0, std::ptr::null());
    assert!(
        result.is_null(),
        "ability_equip(null, 0, null) should return null"
    );
}

#[test]
fn null_input_socket_create_equipment() {
    let result = socket_create_equipment(std::ptr::null(), std::ptr::null());
    assert!(
        result.is_null(),
        "socket_create_equipment(null, null) should return null"
    );
}

#[test]
fn null_input_socket_insert_gem() {
    let result = socket_insert_gem(std::ptr::null(), 0, std::ptr::null());
    assert!(
        result.is_null(),
        "socket_insert_gem(null, 0, null) should return null"
    );
}

#[test]
fn null_input_socket_insert_rune() {
    let result = socket_insert_rune(std::ptr::null(), 0, std::ptr::null());
    assert!(
        result.is_null(),
        "socket_insert_rune(null, 0, null) should return null"
    );
}

#[test]
fn null_input_socket_combine_gems() {
    let result = socket_combine_gems(std::ptr::null());
    assert!(
        result.is_null(),
        "socket_combine_gems(null) should return null"
    );
}

#[test]
fn null_input_cosmetic_unlock() {
    let result = cosmetic_unlock(std::ptr::null(), std::ptr::null());
    assert!(
        result.is_null(),
        "cosmetic_unlock(null, null) should return null"
    );
}

#[test]
fn null_input_cosmetic_apply_transmog() {
    let result = cosmetic_apply_transmog(std::ptr::null(), 0, std::ptr::null());
    assert!(
        result.is_null(),
        "cosmetic_apply_transmog(null, 0, null) should return null"
    );
}

#[test]
fn null_input_cosmetic_apply_dye() {
    let result = cosmetic_apply_dye(std::ptr::null(), 0, 0, std::ptr::null());
    assert!(
        result.is_null(),
        "cosmetic_apply_dye(null, 0, 0, null) should return null"
    );
}

#[test]
fn null_input_tutorial_complete_step() {
    let result = tutorial_complete_step(std::ptr::null(), std::ptr::null());
    assert!(
        result.is_null(),
        "tutorial_complete_step(null, null) should return null"
    );
}

#[test]
fn null_input_tutorial_completion_percent() {
    let pct = tutorial_completion_percent(std::ptr::null());
    assert_eq!(
        pct, 0.0,
        "tutorial_completion_percent(null) should return 0.0"
    );
}

#[test]
fn null_input_achievement_increment() {
    let result = achievement_increment(std::ptr::null(), std::ptr::null(), 10);
    assert!(
        result.is_null(),
        "achievement_increment(null, null, 10) should return null"
    );
}

#[test]
fn null_input_achievement_check_all() {
    let result = achievement_check_all(std::ptr::null(), 0);
    assert!(
        result.is_null(),
        "achievement_check_all(null, 0) should return null"
    );
}

#[test]
fn null_input_achievement_completion_percent() {
    let pct = achievement_completion_percent(std::ptr::null());
    assert_eq!(
        pct, 0.0,
        "achievement_completion_percent(null) should return 0.0"
    );
}

#[test]
fn null_input_season_create_pass() {
    let result = season_create_pass(1, std::ptr::null());
    assert!(
        result.is_null(),
        "season_create_pass(1, null) should return null"
    );
}

#[test]
fn null_input_season_add_xp() {
    let result = season_add_xp(std::ptr::null(), 100);
    assert!(
        result.is_null(),
        "season_add_xp(null, 100) should return null"
    );
}

#[test]
fn null_input_social_create_guild() {
    let result = social_create_guild(
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
    );
    assert!(
        result.is_null(),
        "social_create_guild(nulls) should return null"
    );
}

#[test]
fn null_input_social_guild_add_member() {
    let result = social_guild_add_member(std::ptr::null(), std::ptr::null(), std::ptr::null());
    assert!(
        result.is_null(),
        "social_guild_add_member(nulls) should return null"
    );
}

#[test]
fn null_input_social_create_party() {
    let result = social_create_party(std::ptr::null(), std::ptr::null());
    assert!(
        result.is_null(),
        "social_create_party(nulls) should return null"
    );
}

#[test]
fn null_input_social_party_add_member() {
    let result = social_party_add_member(std::ptr::null(), std::ptr::null(), std::ptr::null(), 0);
    assert!(
        result.is_null(),
        "social_party_add_member(nulls) should return null"
    );
}

#[test]
fn null_input_social_create_trade() {
    let result = social_create_trade(std::ptr::null(), std::ptr::null());
    assert!(
        result.is_null(),
        "social_create_trade(nulls) should return null"
    );
}

#[test]
fn null_input_social_trade_add_item() {
    let result = social_trade_add_item(
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        1,
        std::ptr::null(),
    );
    assert!(
        result.is_null(),
        "social_trade_add_item(nulls) should return null"
    );
}

#[test]
fn null_input_social_trade_lock() {
    let result = social_trade_lock(std::ptr::null(), std::ptr::null());
    assert!(
        result.is_null(),
        "social_trade_lock(nulls) should return null"
    );
}

#[test]
fn null_input_social_trade_confirm() {
    let result = social_trade_confirm(std::ptr::null(), std::ptr::null());
    assert!(
        result.is_null(),
        "social_trade_confirm(nulls) should return null"
    );
}

#[test]
fn null_input_social_trade_execute() {
    let result = social_trade_execute(std::ptr::null());
    assert!(
        result.is_null(),
        "social_trade_execute(null) should return null"
    );
}

// ============================================================
// 2. free_string safety
// ============================================================

#[test]
fn free_string_null_is_safe() {
    // Should not crash
    free_string(std::ptr::null_mut());
    free_string(std::ptr::null_mut());
    free_string(std::ptr::null_mut());
}

// ============================================================
// 3. Malformed / empty JSON inputs
// ============================================================

#[test]
fn malformed_json_mastery_gain_xp() {
    let bad = cstr("not valid json{{{");
    let result = mastery_gain_xp(bad.as_ptr(), 0, 100);
    assert!(
        result.is_null(),
        "mastery_gain_xp with malformed JSON should return null"
    );
}

#[test]
fn empty_string_mastery_gain_xp() {
    let empty = cstr("");
    let result = mastery_gain_xp(empty.as_ptr(), 0, 100);
    assert!(
        result.is_null(),
        "mastery_gain_xp with empty string should return null"
    );
}

#[test]
fn malformed_json_calculate_combat() {
    let bad = cstr("{broken json");
    let result = calculate_combat(bad.as_ptr());
    assert!(
        result.is_null(),
        "calculate_combat with malformed JSON should return null"
    );
}

#[test]
fn empty_json_object_calculate_combat() {
    let empty = cstr("{}");
    let result = calculate_combat(empty.as_ptr());
    // Should return null since required fields are missing
    assert!(
        result.is_null(),
        "calculate_combat with empty object should return null"
    );
}

#[test]
fn malformed_json_semantic_similarity() {
    let bad = cstr("not json");
    let valid = cstr(r#"[["fire", 0.5]]"#);
    let result = semantic_similarity(bad.as_ptr(), valid.as_ptr());
    assert_eq!(
        result, 0.0,
        "semantic_similarity with bad JSON should return 0.0"
    );
}

#[test]
fn empty_tags_semantic_similarity() {
    let empty = cstr("[]");
    let valid = cstr(r#"[["fire", 0.5]]"#);
    let result = semantic_similarity(empty.as_ptr(), valid.as_ptr());
    // Empty tags = 0 similarity or valid value, but should not crash
    assert!(result.is_finite(), "Result should be finite");
}

#[test]
fn malformed_json_generate_loot() {
    // generate_loot uses unwrap_or_default() for tags — bad JSON gives empty tags, still generates loot
    let bad = cstr("broken");
    let result = generate_loot(bad.as_ptr(), 10, 42);
    assert!(
        !result.is_null(),
        "generate_loot gracefully handles bad JSON (defaults to empty tags)"
    );
    assert!(is_valid_json(result));
}

#[test]
fn malformed_json_socket_create_equipment() {
    let name = cstr("Test Sword");
    let bad_colors = cstr("not json");
    let result = socket_create_equipment(name.as_ptr(), bad_colors.as_ptr());
    assert!(
        result.is_null(),
        "socket_create_equipment with bad colors JSON should return null"
    );
}

#[test]
fn malformed_json_season_add_xp() {
    let bad = cstr("not json");
    let result = season_add_xp(bad.as_ptr(), 100);
    assert!(
        result.is_null(),
        "season_add_xp with bad JSON should return null"
    );
}

#[test]
fn malformed_json_achievement_increment() {
    let bad = cstr("{{{{");
    let aid = cstr("monster_slayer_1");
    let result = achievement_increment(bad.as_ptr(), aid.as_ptr(), 10);
    assert!(
        result.is_null(),
        "achievement_increment with bad tracker JSON should return null"
    );
}

#[test]
fn malformed_json_social_guild_add_member() {
    let bad = cstr("not a guild json");
    let uid = cstr("user1");
    let uname = cstr("User");
    let result = social_guild_add_member(bad.as_ptr(), uid.as_ptr(), uname.as_ptr());
    assert!(
        result.is_null(),
        "social_guild_add_member with bad guild JSON should return null"
    );
}

#[test]
fn malformed_json_social_trade_add_item() {
    let bad = cstr("broken");
    let pid = cstr("player1");
    let item = cstr("Sword");
    let rarity = cstr("Common");
    let result = social_trade_add_item(
        bad.as_ptr(),
        pid.as_ptr(),
        item.as_ptr(),
        1,
        rarity.as_ptr(),
    );
    assert!(
        result.is_null(),
        "social_trade_add_item with bad JSON should return null"
    );
}

#[test]
fn malformed_json_create_floor_snapshot() {
    // create_floor_snapshot uses unwrap_or_default() — bad JSON gives empty deltas, still creates snapshot
    let bad = cstr("not an array");
    let result = create_floor_snapshot(42, 1, bad.as_ptr());
    assert!(
        !result.is_null(),
        "create_floor_snapshot gracefully handles bad JSON (defaults to empty deltas)"
    );
    assert!(is_valid_json(result));
}

#[test]
fn malformed_json_evaluate_event_trigger() {
    let bad = cstr("{}broken");
    let result = evaluate_event_trigger(0, bad.as_ptr());
    assert!(
        result.is_null(),
        "evaluate_event_trigger with bad JSON should return null"
    );
}

#[test]
fn malformed_json_spec_find_synergies() {
    let bad = cstr("not array");
    let result = spec_find_synergies(bad.as_ptr());
    assert!(
        result.is_null(),
        "spec_find_synergies with bad JSON should return null"
    );
}

#[test]
fn malformed_json_ability_learn() {
    let bad = cstr("not json");
    let aid = cstr("basic_slash");
    let result = ability_learn(bad.as_ptr(), aid.as_ptr());
    assert!(
        result.is_null(),
        "ability_learn with bad loadout JSON should return null"
    );
}

#[test]
fn malformed_json_tutorial_complete_step() {
    let bad = cstr("bad json");
    let step = cstr("step_1");
    let result = tutorial_complete_step(bad.as_ptr(), step.as_ptr());
    assert!(
        result.is_null(),
        "tutorial_complete_step with bad JSON should return null"
    );
}

#[test]
fn malformed_json_cosmetic_unlock() {
    let bad = cstr("{{bad");
    let cid = cstr("helmet_01");
    let result = cosmetic_unlock(bad.as_ptr(), cid.as_ptr());
    assert!(
        result.is_null(),
        "cosmetic_unlock with bad JSON should return null"
    );
}

// ============================================================
// 4. Boundary values — u64::MAX, u32::MAX, extreme floats
// ============================================================

#[test]
fn max_seed_floor_generation() {
    let ptr = generate_floor(u64::MAX, 1);
    assert!(!ptr.is_null(), "generate_floor should handle u64::MAX seed");
    assert!(is_valid_json(ptr));
}

#[test]
fn max_floor_id_floor_generation() {
    let ptr = generate_floor(42, u32::MAX);
    assert!(
        !ptr.is_null(),
        "generate_floor should handle u32::MAX floor_id"
    );
    assert!(is_valid_json(ptr));
}

#[test]
fn max_seed_floor_layout() {
    let ptr = generate_floor_layout(u64::MAX, 1);
    assert!(
        !ptr.is_null(),
        "generate_floor_layout should handle u64::MAX seed"
    );
    assert!(is_valid_json(ptr));
}

#[test]
fn max_seed_floor_hash() {
    // Should not crash, return some u64
    let hash = get_floor_hash(u64::MAX, u32::MAX);
    let _ = hash; // just verify no crash
}

#[test]
fn max_floor_id_tier() {
    let tier = get_floor_tier(u32::MAX);
    let _ = tier; // verify no crash
}

#[test]
fn zero_floor_id_generation() {
    // floor_id=0 — edge case (most floors start at 1)
    let ptr = generate_floor(42, 0);
    assert!(!ptr.is_null(), "generate_floor should handle floor_id=0");
    assert!(is_valid_json(ptr));
}

#[test]
fn max_seed_monster_generation() {
    let ptr = generate_monster(u64::MAX, 1);
    assert!(
        !ptr.is_null(),
        "generate_monster should handle u64::MAX hash"
    );
    assert!(is_valid_json(ptr));
}

#[test]
fn max_floor_level_monster_generation() {
    let ptr = generate_monster(42, u32::MAX);
    assert!(
        !ptr.is_null(),
        "generate_monster should handle u32::MAX floor_level"
    );
    assert!(is_valid_json(ptr));
}

#[test]
fn zero_count_floor_monsters() {
    let ptr = generate_floor_monsters(42, 1, 0);
    assert!(
        !ptr.is_null(),
        "generate_floor_monsters with count=0 should return valid JSON"
    );
    let json = ptr_to_string(ptr);
    let monsters: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert!(monsters.is_empty(), "0 count should produce empty array");
}

#[test]
fn large_count_floor_monsters() {
    // 100 monsters — shouldn't crash
    let ptr = generate_floor_monsters(42, 1, 100);
    assert!(!ptr.is_null());
    let json = ptr_to_string(ptr);
    let monsters: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(monsters.len(), 100, "Should generate exactly 100 monsters");
}

#[test]
fn max_angle_id_multiplier() {
    // angle_id beyond defined range should return default 1.0
    let mult = get_angle_multiplier(u32::MAX);
    assert!(
        mult > 0.0 && mult.is_finite(),
        "Invalid angle should return valid default"
    );
}

#[test]
fn combat_extreme_damage() {
    let request = serde_json::json!({
        "base_damage": f32::MAX as f64,
        "angle_id": 0,
        "combo_step": 0,
        "attacker_tags_json": "[]",
        "defender_tags_json": "[]"
    });
    let req_str = CString::new(serde_json::to_string(&request).unwrap()).unwrap();
    let result = calculate_combat(req_str.as_ptr());
    if !result.is_null() {
        let json = ptr_to_string(result);
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        let dmg = val["final_damage"].as_f64().unwrap();
        // May be infinite due to f32 overflow, but should not crash
        assert!(!dmg.is_nan(), "Extreme damage should not produce NaN");
    }
}

#[test]
fn combat_zero_damage() {
    let request = serde_json::json!({
        "base_damage": 0.0,
        "angle_id": 0,
        "combo_step": 0,
        "attacker_tags_json": "[]",
        "defender_tags_json": "[]"
    });
    let req_str = CString::new(serde_json::to_string(&request).unwrap()).unwrap();
    let result = calculate_combat(req_str.as_ptr());
    if !result.is_null() {
        let json = ptr_to_string(result);
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        let dmg = val["final_damage"].as_f64().unwrap();
        assert!(
            dmg.is_finite(),
            "Zero base damage should produce finite result"
        );
    }
}

#[test]
fn combat_max_combo_step() {
    let request = serde_json::json!({
        "base_damage": 100.0,
        "angle_id": 0,
        "combo_step": u32::MAX,
        "attacker_tags_json": "[[\"fire\", 0.5]]",
        "defender_tags_json": "[[\"water\", 0.3]]"
    });
    let req_str = CString::new(serde_json::to_string(&request).unwrap()).unwrap();
    let result = calculate_combat(req_str.as_ptr());
    // Should not crash regardless of combo_step value
    if !result.is_null() {
        assert!(is_valid_json(result));
    }
}

#[test]
fn breath_state_zero_elapsed() {
    let ptr = get_breath_state(0.0);
    assert!(!ptr.is_null());
    assert!(is_valid_json(ptr));
}

#[test]
fn breath_state_very_large_elapsed() {
    let ptr = get_breath_state(f32::MAX);
    assert!(!ptr.is_null());
    let json = ptr_to_string(ptr);
    let state: serde_json::Value = serde_json::from_str(&json).unwrap();
    let phase = state["phase"].as_str().unwrap();
    assert!(
        ["Inhale", "Hold", "Exhale", "Pause"].contains(&phase),
        "Very large elapsed should still produce valid phase"
    );
}

#[test]
fn breath_state_negative_elapsed() {
    // Negative time — edge case
    let ptr = get_breath_state(-1.0);
    assert!(!ptr.is_null());
    assert!(is_valid_json(ptr));
}

#[test]
fn loot_max_floor_level() {
    let tags = cstr(r#"[["fire", 0.5]]"#);
    let ptr = generate_loot(tags.as_ptr(), u32::MAX, 42);
    if !ptr.is_null() {
        assert!(is_valid_json(ptr));
    }
}

#[test]
fn loot_floor_level_zero() {
    let tags = cstr(r#"[["fire", 0.5]]"#);
    let ptr = generate_loot(tags.as_ptr(), 0, 42);
    if !ptr.is_null() {
        assert!(is_valid_json(ptr));
    }
}

// ============================================================
// 5. Invalid domain / ID values
// ============================================================

#[test]
fn mastery_invalid_domain_high() {
    let profile = ptr_to_string(mastery_create_profile());
    let pc = cstr(&profile);
    let result = mastery_gain_xp(pc.as_ptr(), 255, 100);
    assert!(result.is_null(), "Invalid domain 255 should return null");
}

#[test]
fn mastery_zero_xp_gain() {
    let profile = ptr_to_string(mastery_create_profile());
    let pc = cstr(&profile);
    let result = mastery_gain_xp(pc.as_ptr(), 0, 0);
    // Gaining 0 XP should still work (no-op)
    assert!(!result.is_null(), "Gaining 0 XP should be valid");
    assert!(is_valid_json(result));
}

#[test]
fn mastery_max_xp_gain() {
    let profile = ptr_to_string(mastery_create_profile());
    let pc = cstr(&profile);
    let result = mastery_gain_xp(pc.as_ptr(), 0, u64::MAX);
    // Should not overflow or crash
    assert!(!result.is_null(), "Gaining u64::MAX XP should be valid");
    assert!(is_valid_json(result));
}

#[test]
fn mastery_unknown_action_name() {
    // Unknown actions return 1 XP (base reward for any action)
    let unknown = cstr("this_action_does_not_exist_at_all");
    let xp = mastery_xp_for_action(unknown.as_ptr());
    assert_eq!(xp, 1, "Unknown action should return 1 XP (base reward)");
}

#[test]
fn ability_learn_unknown_id() {
    let loadout = ptr_to_string(ability_create_loadout());
    let lc = cstr(&loadout);
    let unknown = cstr("nonexistent_ability_xyz_999");
    let result = ability_learn(lc.as_ptr(), unknown.as_ptr());
    assert!(
        result.is_null(),
        "Learning unknown ability should return null"
    );
}

#[test]
fn ability_equip_out_of_range_slot() {
    // Learn a real ability first
    let defaults_json = ptr_to_string(ability_get_defaults());
    let defaults: Vec<serde_json::Value> = serde_json::from_str(&defaults_json).unwrap();
    if defaults.is_empty() {
        return;
    }
    let aid = defaults[0]["id"].as_str().unwrap();

    let loadout_json = ptr_to_string(ability_create_loadout());
    let lc = cstr(&loadout_json);
    let ac = cstr(aid);
    let learned = ptr_to_string(ability_learn(lc.as_ptr(), ac.as_ptr()));

    // Try equipping to slot u32::MAX — equip silently ignores out-of-range slots
    let lc2 = cstr(&learned);
    let ac2 = cstr(aid);
    let result = ability_equip(lc2.as_ptr(), u32::MAX, ac2.as_ptr());
    // The function doesn't validate slot bounds — it returns the loadout unchanged
    assert!(
        !result.is_null(),
        "ability_equip returns loadout unchanged for out-of-range slot"
    );
    assert!(is_valid_json(result));
}

#[test]
fn socket_insert_gem_out_of_range_slot() {
    let name = cstr("Test Sword");
    let colors = cstr("[0]"); // 1 Red socket
    let equip_json = ptr_to_string(socket_create_equipment(name.as_ptr(), colors.as_ptr()));

    let gems_json = ptr_to_string(socket_get_starter_gems());
    let gems: Vec<serde_json::Value> = serde_json::from_str(&gems_json).unwrap();
    if gems.is_empty() {
        return;
    }
    let gem_str = serde_json::to_string(&gems[0]).unwrap();
    let gem_c = cstr(&gem_str);
    let equip_c = cstr(&equip_json);

    // slot 999 doesn't exist
    let result = socket_insert_gem(equip_c.as_ptr(), 999, gem_c.as_ptr());
    assert!(
        result.is_null(),
        "Inserting gem into slot 999 should return null"
    );
}

#[test]
fn cosmetic_unlock_unknown_id() {
    let profile = ptr_to_string(cosmetic_create_profile());
    let pc = cstr(&profile);
    let unknown = cstr("nonexistent_cosmetic_xyz_999");
    let result = cosmetic_unlock(pc.as_ptr(), unknown.as_ptr());
    // May return null or unchanged profile depending on implementation
    if !result.is_null() {
        assert!(is_valid_json(result));
    }
}

#[test]
fn tutorial_complete_unknown_step() {
    let progress = ptr_to_string(tutorial_create_progress());
    let pc = cstr(&progress);
    let unknown = cstr("nonexistent_step_999");
    let result = tutorial_complete_step(pc.as_ptr(), unknown.as_ptr());
    // Should handle gracefully — null or unchanged progress
    if !result.is_null() {
        assert!(is_valid_json(result));
    }
}

#[test]
fn achievement_increment_unknown_id() {
    let tracker = ptr_to_string(achievement_create_tracker());
    let tc = cstr(&tracker);
    let unknown = cstr("achievement_that_does_not_exist");
    let result = achievement_increment(tc.as_ptr(), unknown.as_ptr(), 100);
    // Should handle gracefully
    if !result.is_null() {
        assert!(is_valid_json(result));
    }
}

#[test]
fn achievement_increment_zero_amount() {
    let tracker = ptr_to_string(achievement_create_tracker());
    let tc = cstr(&tracker);
    let aid = cstr("monster_slayer_1");
    let result = achievement_increment(tc.as_ptr(), aid.as_ptr(), 0);
    assert!(!result.is_null(), "Incrementing by 0 should be valid");
    assert!(is_valid_json(result));
}

#[test]
fn achievement_increment_max_amount() {
    let tracker = ptr_to_string(achievement_create_tracker());
    let tc = cstr(&tracker);
    let aid = cstr("monster_slayer_1");
    let result = achievement_increment(tc.as_ptr(), aid.as_ptr(), u64::MAX);
    // Should not overflow
    assert!(
        !result.is_null(),
        "Incrementing by u64::MAX should be valid"
    );
    assert!(is_valid_json(result));
}

// ============================================================
// 6. Season pass edge cases
// ============================================================

#[test]
fn season_zero_season_number() {
    let name = cstr("Season Zero");
    let ptr = season_create_pass(0, name.as_ptr());
    assert!(!ptr.is_null(), "Season 0 should be valid");
    assert!(is_valid_json(ptr));
}

#[test]
fn season_max_season_number() {
    let name = cstr("Max Season");
    let ptr = season_create_pass(u32::MAX, name.as_ptr());
    assert!(!ptr.is_null(), "Season u32::MAX should be valid");
    assert!(is_valid_json(ptr));
}

#[test]
fn season_add_zero_xp() {
    let name = cstr("Test Season");
    let pass = ptr_to_string(season_create_pass(1, name.as_ptr()));
    let pc = cstr(&pass);
    let result = season_add_xp(pc.as_ptr(), 0);
    assert!(!result.is_null(), "Adding 0 XP should be valid");
    assert!(is_valid_json(result));
}

#[test]
fn season_add_max_xp() {
    let name = cstr("Test Season");
    let pass = ptr_to_string(season_create_pass(1, name.as_ptr()));
    let pc = cstr(&pass);
    let result = season_add_xp(pc.as_ptr(), u64::MAX);
    // Should not overflow
    assert!(!result.is_null(), "Adding u64::MAX XP should be valid");
    assert!(is_valid_json(result));
}

#[test]
fn season_rewards_max_number() {
    let ptr = season_get_rewards(u32::MAX);
    assert!(
        !ptr.is_null(),
        "season_get_rewards(u32::MAX) should be valid"
    );
    assert!(is_valid_json(ptr));
}

// ============================================================
// 7. Social system edge cases
// ============================================================

#[test]
fn guild_empty_name() {
    let name = cstr("");
    let tag = cstr("TG");
    let lid = cstr("leader");
    let lname = cstr("Leader");
    let faction = cstr("AscendingOrder");
    let result = social_create_guild(
        name.as_ptr(),
        tag.as_ptr(),
        lid.as_ptr(),
        lname.as_ptr(),
        faction.as_ptr(),
    );
    // Might return null or create guild with empty name
    if !result.is_null() {
        assert!(is_valid_json(result));
    }
}

#[test]
fn guild_unknown_faction() {
    let name = cstr("Test Guild");
    let tag = cstr("TG");
    let lid = cstr("leader");
    let lname = cstr("Leader");
    let faction = cstr("NonexistentFaction999");
    let result = social_create_guild(
        name.as_ptr(),
        tag.as_ptr(),
        lid.as_ptr(),
        lname.as_ptr(),
        faction.as_ptr(),
    );
    // Should handle gracefully
    if !result.is_null() {
        assert!(is_valid_json(result));
    }
}

#[test]
fn trade_lock_wrong_player() {
    let pa = cstr("player_a");
    let pb = cstr("player_b");
    let trade = ptr_to_string(social_create_trade(pa.as_ptr(), pb.as_ptr()));

    // Try locking with a player not in the trade
    let tc = cstr(&trade);
    let wrong = cstr("player_c_not_in_trade");
    let result = social_trade_lock(tc.as_ptr(), wrong.as_ptr());
    // Should return null or unchanged trade
    if !result.is_null() {
        assert!(is_valid_json(result));
    }
}

#[test]
fn trade_execute_without_both_confirmed() {
    let pa = cstr("player_a");
    let pb = cstr("player_b");
    let trade = ptr_to_string(social_create_trade(pa.as_ptr(), pb.as_ptr()));

    // Try executing without locking/confirming — execute() runs regardless of state
    let tc = cstr(&trade);
    let result = social_trade_execute(tc.as_ptr());
    // trade.execute() doesn't validate state, returns the trade object as-is
    assert!(
        !result.is_null(),
        "trade_execute returns trade regardless of state"
    );
    assert!(is_valid_json(result));
}

#[test]
fn party_add_member_high_role_id() {
    let lid = cstr("leader");
    let lname = cstr("Leader");
    let party = ptr_to_string(social_create_party(lid.as_ptr(), lname.as_ptr()));

    let pc = cstr(&party);
    let uid = cstr("member1");
    let uname = cstr("Member");
    // role_id 255 — likely out of enum range
    let result = social_party_add_member(pc.as_ptr(), uid.as_ptr(), uname.as_ptr(), 255);
    // Should handle gracefully
    if !result.is_null() {
        assert!(is_valid_json(result));
    }
}

// ============================================================
// 8. Replication edge cases
// ============================================================

#[test]
fn record_delta_max_tick() {
    let player = cstr("player1");
    let payload = cstr(r#"{"action":"test"}"#);
    let result = record_delta(0, 1, 42, player.as_ptr(), payload.as_ptr(), u64::MAX);
    assert!(!result.is_null(), "record_delta with max tick should work");
    assert!(is_valid_json(result));
}

#[test]
fn record_delta_max_entity_hash() {
    let player = cstr("player1");
    let payload = cstr(r#"{"action":"test"}"#);
    let result = record_delta(0, 1, u64::MAX, player.as_ptr(), payload.as_ptr(), 1);
    assert!(
        !result.is_null(),
        "record_delta with max entity_hash should work"
    );
    assert!(is_valid_json(result));
}

#[test]
fn create_floor_snapshot_empty_deltas() {
    let deltas = cstr("[]");
    let result = create_floor_snapshot(42, 1, deltas.as_ptr());
    assert!(!result.is_null(), "Snapshot with empty deltas should work");
    assert!(is_valid_json(result));
}

// ============================================================
// 9. Event trigger edge cases
// ============================================================

#[test]
fn event_trigger_unknown_type_id() {
    let ctx = serde_json::json!({
        "breath_phase": "Hold",
        "floor_tags": [],
        "floor_hash": 0,
        "corruption_level": 0.0,
        "player_actions": [],
        "active_factions": []
    });
    let ctx_str = CString::new(serde_json::to_string(&ctx).unwrap()).unwrap();
    // type_id 255 — likely out of enum range
    let result = evaluate_event_trigger(255, ctx_str.as_ptr());
    // Should return null for unknown trigger type
    assert!(result.is_null(), "Unknown trigger type should return null");
}

// ============================================================
// 10. Determinism verification with extreme seeds
// ============================================================

#[test]
fn determinism_floor_extreme_seeds() {
    let seeds = [0u64, 1, u64::MAX, u64::MAX - 1];
    for seed in seeds {
        let a = ptr_to_string(generate_floor(seed, 1));
        let b = ptr_to_string(generate_floor(seed, 1));
        assert_eq!(
            a, b,
            "Floor generation must be deterministic for seed={seed}"
        );
    }
}

#[test]
fn determinism_monster_extreme_hashes() {
    let hashes = [0u64, 1, u64::MAX, u64::MAX / 2];
    for hash in hashes {
        let a = ptr_to_string(generate_monster(hash, 5));
        let b = ptr_to_string(generate_monster(hash, 5));
        assert_eq!(
            a, b,
            "Monster generation must be deterministic for hash={hash}"
        );
    }
}

// ============================================================
// 11. Version & static getters validation
// ============================================================

#[test]
fn version_string_is_valid() {
    let version = ptr_to_string(get_version());
    assert!(!version.is_empty(), "Version should not be empty");
    // Should follow semver-like format
    assert!(
        version.contains('.'),
        "Version should contain dots (semver)"
    );
}

#[test]
fn static_getters_always_valid() {
    // These take no parameters and should always return valid JSON
    assert!(is_valid_json(mastery_get_all_domains()));
    assert!(is_valid_json(mastery_create_profile()));
    assert!(is_valid_json(spec_get_all_branches()));
    assert!(is_valid_json(spec_create_profile()));
    assert!(is_valid_json(ability_get_defaults()));
    assert!(is_valid_json(ability_create_loadout()));
    assert!(is_valid_json(socket_get_starter_gems()));
    assert!(is_valid_json(socket_get_starter_runes()));
    assert!(is_valid_json(cosmetic_get_all()));
    assert!(is_valid_json(cosmetic_get_all_dyes()));
    assert!(is_valid_json(cosmetic_create_profile()));
    assert!(is_valid_json(tutorial_get_steps()));
    assert!(is_valid_json(tutorial_get_hints()));
    assert!(is_valid_json(tutorial_create_progress()));
    assert!(is_valid_json(achievement_create_tracker()));
}

// ============================================================
// 12. Socket system color matching edge cases
// ============================================================

#[test]
fn socket_all_color_values() {
    // Test all defined SocketColor values: Red=0, Blue=1, Green=2, Prismatic=3
    for color_val in 0..=3 {
        let name = cstr(&format!("Color{color_val} Sword"));
        let colors = cstr(&format!("[{color_val}]"));
        let result = socket_create_equipment(name.as_ptr(), colors.as_ptr());
        assert!(!result.is_null(), "Color {color_val} should be valid");
        assert!(is_valid_json(result));
    }
}

#[test]
fn socket_invalid_color_value() {
    let name = cstr("Bad Color Sword");
    let colors = cstr("[99]"); // No such SocketColor
    let result = socket_create_equipment(name.as_ptr(), colors.as_ptr());
    // Should return null or handle gracefully
    if !result.is_null() {
        assert!(is_valid_json(result));
    }
}

#[test]
fn socket_empty_colors_array() {
    let name = cstr("No Socket Sword");
    let colors = cstr("[]"); // Zero sockets
    let result = socket_create_equipment(name.as_ptr(), colors.as_ptr());
    if !result.is_null() {
        let json = ptr_to_string(result);
        let equip: serde_json::Value = serde_json::from_str(&json).unwrap();
        let sockets = equip["sockets"].as_array().unwrap();
        assert!(sockets.is_empty(), "Empty colors should produce 0 sockets");
    }
}

// ============================================================
// 13. Semantic tags edge cases
// ============================================================

#[test]
fn semantic_identical_tags() {
    let tags = cstr(r#"[["fire", 0.8], ["water", 0.5]]"#);
    let similarity = semantic_similarity(tags.as_ptr(), tags.as_ptr());
    // Identical tags should have similarity close to 1.0
    assert!(
        similarity >= 0.9,
        "Identical tags should have similarity >= 0.9, got {similarity}"
    );
}

#[test]
fn semantic_single_tag() {
    let a = cstr(r#"[["fire", 1.0]]"#);
    let b = cstr(r#"[["ice", 1.0]]"#);
    let similarity = semantic_similarity(a.as_ptr(), b.as_ptr());
    assert!(
        (0.0..=1.0).contains(&similarity),
        "Similarity should be in [0, 1], got {similarity}"
    );
}

#[test]
fn semantic_many_tags() {
    let a = cstr(
        r#"[["fire", 0.8], ["water", 0.5], ["earth", 0.3], ["wind", 0.2], ["void", 0.1], ["arcane", 0.9]]"#,
    );
    let b = cstr(r#"[["fire", 0.1], ["corruption", 0.9], ["shadow", 0.8]]"#);
    let similarity = semantic_similarity(a.as_ptr(), b.as_ptr());
    assert!(
        (0.0..=1.0).contains(&similarity),
        "Similarity should be in [0, 1], got {similarity}"
    );
}
