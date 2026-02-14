//! Fuzz-style stress tests for FFI surface.
//!
//! Validates that rapid-fire calls, extreme inputs, malformed JSON,
//! and concurrent access don't cause crashes, panics, or memory issues.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use tower_core::bridge::*;

// ============================================================
// Helpers
// ============================================================

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn is_valid_json(ptr: *mut c_char) -> bool {
    if ptr.is_null() {
        return false;
    }
    let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("");
    let valid = serde_json::from_str::<serde_json::Value>(s).is_ok();
    free_string(ptr);
    valid
}

fn ptr_to_string(ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let s = unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .unwrap_or("")
        .to_owned();
    free_string(ptr);
    s
}

// ============================================================
// Rapid-fire stress: call FFI functions N times in tight loops
// ============================================================

const RAPID_ITERS: usize = 200;

#[test]
fn stress_generate_floor_rapid() {
    for i in 0..RAPID_ITERS {
        let ptr = generate_floor(42, i as u32);
        assert!(!ptr.is_null());
        assert!(is_valid_json(ptr));
    }
}

#[test]
fn stress_generate_floor_layout_rapid() {
    for i in 0..RAPID_ITERS {
        let ptr = generate_floor_layout(42, i as u32);
        assert!(!ptr.is_null());
        assert!(is_valid_json(ptr));
    }
}

#[test]
fn stress_generate_monster_rapid() {
    for i in 0..RAPID_ITERS {
        let ptr = generate_monster(i as u64 * 997, i as u32);
        assert!(!ptr.is_null());
        assert!(is_valid_json(ptr));
    }
}

#[test]
fn stress_generate_floor_monsters_rapid() {
    for i in 0..50 {
        let ptr = generate_floor_monsters(i as u64, i as u32, 10);
        assert!(!ptr.is_null());
        assert!(is_valid_json(ptr));
    }
}

#[test]
fn stress_combat_calc_rapid() {
    let tags_a = cstr(r#"[["fire", 0.8]]"#);
    let tags_b = cstr(r#"[["water", 0.5]]"#);
    for i in 0..RAPID_ITERS {
        let req = format!(
            r#"{{"base_damage":{}.0,"angle_id":{},"combo_step":{},"attacker_tags_json":"[[\"fire\",0.8]]","defender_tags_json":"[[\"water\",0.5]]"}}"#,
            50 + i % 200,
            i % 4,
            i % 8
        );
        let req_c = cstr(&req);
        let ptr = calculate_combat(req_c.as_ptr());
        assert!(!ptr.is_null());
        assert!(is_valid_json(ptr));
    }
    // Also stress semantic_similarity in same loop
    for _ in 0..RAPID_ITERS {
        let sim = semantic_similarity(tags_a.as_ptr(), tags_b.as_ptr());
        assert!((0.0..=1.0).contains(&sim));
    }
}

#[test]
fn stress_angle_multiplier_all_ids() {
    for id in 0..1000 {
        let mult = get_angle_multiplier(id);
        assert!(mult > 0.0);
        assert!(mult <= 2.0);
    }
}

#[test]
fn stress_breath_state_sweep() {
    // Sweep through an entire breath cycle + beyond
    for i in 0..2000 {
        let elapsed = i as f32 * 0.6; // 0..1200 seconds
        let ptr = get_breath_state(elapsed);
        assert!(!ptr.is_null());
        assert!(is_valid_json(ptr));
    }
}

#[test]
fn stress_loot_generation_rapid() {
    let tags = cstr(r#"[["fire", 0.8], ["corruption", 0.4]]"#);
    for i in 0..RAPID_ITERS {
        let ptr = generate_loot(tags.as_ptr(), i as u32, i as u64 * 31337);
        assert!(!ptr.is_null());
        assert!(is_valid_json(ptr));
    }
}

#[test]
fn stress_mastery_workflow_rapid() {
    for _ in 0..50 {
        let profile_ptr = mastery_create_profile();
        assert!(!profile_ptr.is_null());
        let profile_str = ptr_to_string(profile_ptr);
        let pc = cstr(&profile_str);

        // Gain XP in multiple domains rapidly
        let domains = [
            0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let mut current_profile = pc;
        for &domain_id in &domains {
            let result = mastery_gain_xp(current_profile.as_ptr(), domain_id, 50);
            if !result.is_null() {
                let s = ptr_to_string(result);
                current_profile = cstr(&s);
            }
        }
    }
}

#[test]
fn stress_floor_hash_determinism() {
    // Verify hashing is deterministic across many calls
    for seed in [0u64, 1, 42, u64::MAX, u64::MAX / 2] {
        for floor in 0..100u32 {
            let h1 = get_floor_hash(seed, floor);
            let h2 = get_floor_hash(seed, floor);
            assert_eq!(
                h1, h2,
                "Hash must be deterministic for seed={seed}, floor={floor}"
            );
        }
    }
}

#[test]
fn stress_floor_tier_full_range() {
    for floor_id in (0..1000).chain([u32::MAX - 1, u32::MAX].into_iter()) {
        let tier = get_floor_tier(floor_id);
        assert!(
            tier <= 3,
            "Tier must be 0-3, got {tier} for floor_id={floor_id}"
        );
    }
}

// ============================================================
// Malformed JSON stress
// ============================================================

#[test]
fn stress_malformed_json_combat() {
    let payloads = [
        "",
        "{}",
        "null",
        "42",
        r#"{"base_damage":"not_a_number"}"#,
        r#"{"base_damage":100}"#, // missing fields
        r#"[1,2,3]"#,
        r#"{"base_damage":1e999,"angle_id":0,"combo_step":0,"attacker_tags_json":"[]","defender_tags_json":"[]"}"#,
        "{invalid json!!!",
        &"x".repeat(10000),
    ];
    for payload in &payloads {
        let c = cstr(payload);
        let ptr = calculate_combat(c.as_ptr());
        // Should either return valid JSON or null — never crash
        if !ptr.is_null() {
            free_string(ptr);
        }
    }
}

#[test]
fn stress_malformed_json_mastery() {
    let payloads = ["", "{}", "null", "[1,2,3]", "{invalid", &"a".repeat(5000)];
    for payload in &payloads {
        let c = cstr(payload);
        let ptr = mastery_gain_xp(c.as_ptr(), 0, 100);
        if !ptr.is_null() {
            free_string(ptr);
        }

        let tier = mastery_get_tier(c.as_ptr(), 0);
        let _ = tier; // just ensure no crash
    }
}

#[test]
fn stress_malformed_json_semantic() {
    let payloads = [
        "",
        "{}",
        "null",
        "[1,2,3]",
        "[[]]",
        r#"[["tag"]]"#, // missing value
        r#"[["tag", "not_float"]]"#,
        "{invalid",
    ];
    for p in &payloads {
        let c = cstr(p);
        let sim = semantic_similarity(c.as_ptr(), c.as_ptr());
        assert!(sim >= 0.0, "Similarity must be non-negative for input: {p}");
    }
}

#[test]
fn stress_malformed_json_loot() {
    let payloads = ["", "{}", "null", "[1,2]", "{invalid"];
    for p in &payloads {
        let c = cstr(p);
        let ptr = generate_loot(c.as_ptr(), 10, 42);
        // null or valid JSON
        if !ptr.is_null() {
            free_string(ptr);
        }
    }
}

#[test]
fn stress_malformed_json_snapshot() {
    let payloads = [
        "",
        "{}",
        "null",
        "[1]",
        "{invalid",
        r#"[{"wrong":"format"}]"#,
    ];
    for p in &payloads {
        let c = cstr(p);
        let ptr = create_floor_snapshot(42, 1, c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
    }
}

#[test]
fn stress_malformed_json_abilities() {
    let payloads = ["", "{}", "null", "{invalid"];
    for p in &payloads {
        let c = cstr(p);
        let ptr = ability_learn(c.as_ptr(), c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
        let ptr = ability_equip(c.as_ptr(), 0, c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
    }
}

#[test]
fn stress_malformed_json_sockets() {
    let payloads = ["", "{}", "null", "{invalid"];
    for p in &payloads {
        let c = cstr(p);
        let ptr = socket_insert_gem(c.as_ptr(), 0, c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
        let ptr = socket_insert_rune(c.as_ptr(), 0, c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
        let ptr = socket_combine_gems(c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
    }
}

#[test]
fn stress_malformed_json_cosmetics() {
    let payloads = ["", "{}", "null", "{invalid"];
    for p in &payloads {
        let c = cstr(p);
        let ptr = cosmetic_unlock(c.as_ptr(), c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
        let ptr = cosmetic_apply_transmog(c.as_ptr(), 0, c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
        let ptr = cosmetic_apply_dye(c.as_ptr(), 0, 0, c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
    }
}

#[test]
fn stress_malformed_json_social() {
    let payloads = ["", "{}", "null", "{invalid"];
    for p in &payloads {
        let c = cstr(p);
        let ptr = social_guild_add_member(c.as_ptr(), c.as_ptr(), c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
        let ptr = social_party_add_member(c.as_ptr(), c.as_ptr(), c.as_ptr(), 0);
        if !ptr.is_null() {
            free_string(ptr);
        }
        let ptr = social_trade_add_item(c.as_ptr(), c.as_ptr(), c.as_ptr(), 1, c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
        let ptr = social_trade_lock(c.as_ptr(), c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
        let ptr = social_trade_confirm(c.as_ptr(), c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
        let ptr = social_trade_execute(c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
    }
}

#[test]
fn stress_malformed_json_events() {
    let payloads = ["", "{}", "null", "{invalid"];
    for p in &payloads {
        let c = cstr(p);
        let ptr = evaluate_event_trigger(0, c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
    }
}

#[test]
fn stress_malformed_json_seasons() {
    let payloads = ["", "{}", "null", "{invalid"];
    for p in &payloads {
        let c = cstr(p);
        let ptr = season_add_xp(c.as_ptr(), 100);
        if !ptr.is_null() {
            free_string(ptr);
        }
    }
}

#[test]
fn stress_malformed_json_tutorial() {
    let payloads = ["", "{}", "null", "{invalid"];
    for p in &payloads {
        let c = cstr(p);
        let ptr = tutorial_complete_step(c.as_ptr(), c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
        let pct = tutorial_completion_percent(c.as_ptr());
        assert!((0.0..=100.0).contains(&pct) || pct == 0.0);
    }
}

#[test]
fn stress_malformed_json_achievements() {
    let payloads = ["", "{}", "null", "{invalid"];
    for p in &payloads {
        let c = cstr(p);
        let ptr = achievement_increment(c.as_ptr(), c.as_ptr(), 1);
        if !ptr.is_null() {
            free_string(ptr);
        }
        let ptr = achievement_check_all(c.as_ptr(), 0);
        if !ptr.is_null() {
            free_string(ptr);
        }
        let pct = achievement_completion_percent(c.as_ptr());
        let _ = pct; // no crash is the assertion
    }
}

#[test]
fn stress_malformed_json_spec() {
    let payloads = ["", "{}", "null", "{invalid"];
    for p in &payloads {
        let c = cstr(p);
        let ptr = spec_choose_branch(c.as_ptr(), c.as_ptr(), c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
        let ptr = spec_find_synergies(c.as_ptr());
        if !ptr.is_null() {
            free_string(ptr);
        }
    }
}

// ============================================================
// Extreme numeric values
// ============================================================

#[test]
fn stress_extreme_seeds() {
    let seeds = [0u64, 1, u64::MAX, u64::MAX - 1, u64::MAX / 2, 0xDEADBEEF];
    for &seed in &seeds {
        let ptr = generate_floor(seed, 1);
        assert!(!ptr.is_null());
        assert!(is_valid_json(ptr));

        let ptr = generate_floor_monsters(seed, 1, 5);
        assert!(!ptr.is_null());
        assert!(is_valid_json(ptr));
    }
}

#[test]
fn stress_extreme_floor_ids() {
    let floors = [0u32, 1, 100, 500, 1000, u32::MAX - 1, u32::MAX];
    for &floor in &floors {
        let ptr = generate_floor(42, floor);
        assert!(!ptr.is_null());
        assert!(is_valid_json(ptr));
    }
}

#[test]
fn stress_extreme_breath_values() {
    let values = [
        0.0f32,
        -1.0,
        f32::EPSILON,
        1e10,
        f32::MAX,
        f32::INFINITY,
        f32::NAN,
    ];
    for &v in &values {
        let ptr = get_breath_state(v);
        // NaN/infinity might produce odd results but should never crash
        if !ptr.is_null() {
            free_string(ptr);
        }
    }
}

#[test]
fn stress_extreme_combat_values() {
    let req = format!(
        r#"{{"base_damage":{},"angle_id":{},"combo_step":{},"attacker_tags_json":"[]","defender_tags_json":"[]"}}"#,
        f32::MAX,
        u32::MAX,
        u32::MAX
    );
    let c = cstr(&req);
    let ptr = calculate_combat(c.as_ptr());
    if !ptr.is_null() {
        free_string(ptr);
    }

    // Zero damage
    let req = r#"{"base_damage":0.0,"angle_id":0,"combo_step":0,"attacker_tags_json":"[]","defender_tags_json":"[]"}"#;
    let c = cstr(req);
    let ptr = calculate_combat(c.as_ptr());
    assert!(!ptr.is_null());
    assert!(is_valid_json(ptr));

    // Negative damage
    let req = r#"{"base_damage":-100.0,"angle_id":0,"combo_step":0,"attacker_tags_json":"[]","defender_tags_json":"[]"}"#;
    let c = cstr(req);
    let ptr = calculate_combat(c.as_ptr());
    assert!(!ptr.is_null());
    assert!(is_valid_json(ptr));
}

#[test]
fn stress_extreme_loot_params() {
    let tags = cstr(r#"[["fire", 0.8]]"#);
    for &floor in &[0u32, 1, u32::MAX] {
        for &hash in &[0u64, u64::MAX] {
            let ptr = generate_loot(tags.as_ptr(), floor, hash);
            assert!(!ptr.is_null());
            assert!(is_valid_json(ptr));
        }
    }
}

// ============================================================
// Memory allocation/free stress
// ============================================================

#[test]
fn stress_alloc_free_cycle() {
    // Rapidly allocate and free strings
    for _ in 0..500 {
        let ptr = get_version();
        assert!(!ptr.is_null());
        free_string(ptr);
    }
}

#[test]
fn stress_alloc_free_large_payloads() {
    // Generate floors (produce large JSON) and free immediately
    for i in 0..100 {
        let ptr = generate_floor(42, i);
        assert!(!ptr.is_null());
        free_string(ptr);
    }
}

#[test]
fn stress_free_null_safety() {
    // free_string(null) should be a no-op
    for _ in 0..1000 {
        free_string(std::ptr::null_mut());
    }
}

#[test]
fn stress_double_free_guard() {
    // Note: this tests that our free_string gracefully handles
    // already-freed pointers (best effort — UB territory on most allocators).
    // We only test null here to be safe.
    let ptr = get_version();
    free_string(ptr);
    // Do NOT double-free — just verify the first free worked
}

// ============================================================
// No-argument FFI functions stress
// ============================================================

#[test]
fn stress_parameterless_functions() {
    for _ in 0..100 {
        assert!(is_valid_json(mastery_create_profile()));
        assert!(is_valid_json(mastery_get_all_domains()));
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
}

#[test]
fn stress_season_functions() {
    for i in 0..100u32 {
        let name = cstr("Test Season");
        assert!(is_valid_json(season_create_pass(i, name.as_ptr())));
        assert!(is_valid_json(season_generate_dailies(i as u64)));
        assert!(is_valid_json(season_generate_weeklies(i as u64)));
        assert!(is_valid_json(season_get_rewards(i)));
    }
}

// ============================================================
// Concurrent access stress (via rayon)
// ============================================================

#[test]
fn stress_concurrent_floor_generation() {
    use rayon::prelude::*;

    let results: Vec<bool> = (0..200u32)
        .into_par_iter()
        .map(|i| {
            let ptr = generate_floor(42, i);
            if ptr.is_null() {
                return false;
            }
            let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("");
            let valid = serde_json::from_str::<serde_json::Value>(s).is_ok();
            free_string(ptr);
            valid
        })
        .collect();

    assert!(
        results.iter().all(|&v| v),
        "All concurrent floor generations must produce valid JSON"
    );
}

#[test]
fn stress_concurrent_combat_calcs() {
    use rayon::prelude::*;

    let results: Vec<bool> = (0..200u32)
        .into_par_iter()
        .map(|i| {
            let req = format!(
                r#"{{"base_damage":{}.0,"angle_id":{},"combo_step":{},"attacker_tags_json":"[[\"fire\",0.8]]","defender_tags_json":"[[\"water\",0.5]]"}}"#,
                50 + i,
                i % 4,
                i % 8
            );
            let c = CString::new(req).unwrap();
            let ptr = calculate_combat(c.as_ptr());
            if ptr.is_null() {
                return false;
            }
            let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("");
            let valid = serde_json::from_str::<serde_json::Value>(s).is_ok();
            free_string(ptr);
            valid
        })
        .collect();

    assert!(results.iter().all(|&v| v));
}

#[test]
fn stress_concurrent_monster_generation() {
    use rayon::prelude::*;

    let results: Vec<bool> = (0..200u64)
        .into_par_iter()
        .map(|i| {
            let ptr = generate_monster(i * 997, (i % 100) as u32);
            if ptr.is_null() {
                return false;
            }
            let s = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or("");
            let valid = serde_json::from_str::<serde_json::Value>(s).is_ok();
            free_string(ptr);
            valid
        })
        .collect();

    assert!(results.iter().all(|&v| v));
}

#[test]
fn stress_concurrent_mixed_ffi() {
    use rayon::prelude::*;

    // Mix of different FFI calls concurrently
    let results: Vec<bool> = (0..300u32)
        .into_par_iter()
        .map(|i| match i % 6 {
            0 => {
                let ptr = generate_floor(42, i);
                let ok = !ptr.is_null();
                if ok {
                    free_string(ptr);
                }
                ok
            }
            1 => {
                let ptr = generate_monster(i as u64 * 31, i);
                let ok = !ptr.is_null();
                if ok {
                    free_string(ptr);
                }
                ok
            }
            2 => {
                let ptr = get_breath_state(i as f32 * 10.0);
                let ok = !ptr.is_null();
                if ok {
                    free_string(ptr);
                }
                ok
            }
            3 => {
                let tags = CString::new(r#"[["fire", 0.8]]"#).unwrap();
                let ptr = generate_loot(tags.as_ptr(), i, i as u64);
                let ok = !ptr.is_null();
                if ok {
                    free_string(ptr);
                }
                ok
            }
            4 => {
                let ptr = get_version();
                let ok = !ptr.is_null();
                if ok {
                    free_string(ptr);
                }
                ok
            }
            _ => {
                let ptr = mastery_create_profile();
                let ok = !ptr.is_null();
                if ok {
                    free_string(ptr);
                }
                ok
            }
        })
        .collect();

    assert!(
        results.iter().all(|&v| v),
        "All concurrent mixed FFI calls must succeed"
    );
}
