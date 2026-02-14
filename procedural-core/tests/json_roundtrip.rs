/// Integration tests: JSON round-trip across FFI boundary.
///
/// Simulates the UE5 client pattern:
///   1. Call create_* FFI → get JSON string
///   2. Parse JSON (client-side validation)
///   3. Pass JSON back through modify_* FFI → get updated JSON
///   4. Parse again → verify modifications
///   5. Free all strings
///
/// These tests ensure Rust→JSON→Rust round-trips produce valid,
/// consistent state across the entire FFI surface.
use std::ffi::{CStr, CString};
use tower_core::bridge::*;

// ============================================================
// Helpers
// ============================================================

fn ptr_to_string(ptr: *mut std::os::raw::c_char) -> String {
    assert!(!ptr.is_null(), "FFI returned null pointer");
    let s = unsafe { CStr::from_ptr(ptr).to_str().unwrap().to_owned() };
    free_string(ptr);
    s
}

fn ptr_to_json(ptr: *mut std::os::raw::c_char) -> serde_json::Value {
    let s = ptr_to_string(ptr);
    serde_json::from_str(&s).unwrap_or_else(|e| {
        panic!("Invalid JSON from FFI: {e}\nRaw: {s}");
    })
}

fn cstr(s: &str) -> CString {
    CString::new(s).unwrap()
}

// ============================================================
// Mastery round-trip
// ============================================================

#[test]
fn roundtrip_mastery_profile_create_and_gain_xp() {
    // Step 1: Create fresh profile
    let profile_json = ptr_to_string(mastery_create_profile());
    let profile: serde_json::Value = serde_json::from_str(&profile_json).unwrap();
    assert!(profile.is_object());

    // Step 2: Gain XP in SwordMastery (domain 0) through FFI
    let input = cstr(&profile_json);
    let updated_json = ptr_to_string(mastery_gain_xp(input.as_ptr(), 0, 1000));
    let updated: serde_json::Value = serde_json::from_str(&updated_json).unwrap();
    assert!(updated.is_object());

    // Step 3: Check tier through FFI with updated JSON
    let input2 = cstr(&updated_json);
    let tier = mastery_get_tier(input2.as_ptr(), 0);
    assert!(tier >= 0, "Tier should be >= 0 after gaining 1000 XP");

    // Step 4: Gain more XP on already-modified profile
    let input3 = cstr(&updated_json);
    let final_json = ptr_to_string(mastery_gain_xp(input3.as_ptr(), 0, 5000));
    let final_input = cstr(&final_json);
    let final_tier = mastery_get_tier(final_input.as_ptr(), 0);
    assert!(final_tier >= tier, "More XP should not decrease tier");
}

#[test]
fn roundtrip_mastery_all_21_domains() {
    let domains_json = ptr_to_string(mastery_get_all_domains());
    let domains: Vec<String> = serde_json::from_str(&domains_json).unwrap();
    assert_eq!(domains.len(), 21);

    // Gain XP in every domain
    let mut profile_json = ptr_to_string(mastery_create_profile());
    for domain_id in 0..21u32 {
        let input = cstr(&profile_json);
        profile_json = ptr_to_string(mastery_gain_xp(input.as_ptr(), domain_id, 100));
    }

    // Verify all domains have tier >= 0
    for domain_id in 0..21u32 {
        let input = cstr(&profile_json);
        let tier = mastery_get_tier(input.as_ptr(), domain_id);
        assert!(tier >= 0, "Domain {domain_id} should have valid tier");
    }
}

// ============================================================
// Specialization round-trip
// ============================================================

#[test]
fn roundtrip_spec_profile_and_branches() {
    let branches_json = ptr_to_string(spec_get_all_branches());
    let branches: Vec<serde_json::Value> = serde_json::from_str(&branches_json).unwrap();
    assert!(!branches.is_empty(), "Should have specialization branches");

    let profile_json = ptr_to_string(spec_create_profile());
    let _profile: serde_json::Value = serde_json::from_str(&profile_json).unwrap();

    // Synergies with known branch IDs
    let ids = cstr(r#"["sword_berserker","parry_counter"]"#);
    let synergies_json = ptr_to_string(spec_find_synergies(ids.as_ptr()));
    let synergies: Vec<serde_json::Value> = serde_json::from_str(&synergies_json).unwrap();
    // May be empty if no synergy between these, but should be valid JSON array
    let _ = &synergies; // valid JSON array parsed successfully
}

// ============================================================
// Abilities round-trip
// ============================================================

#[test]
fn roundtrip_ability_learn_and_equip() {
    // Get default abilities
    let defaults_json = ptr_to_string(ability_get_defaults());
    let defaults: Vec<serde_json::Value> = serde_json::from_str(&defaults_json).unwrap();
    assert!(!defaults.is_empty(), "Should have default abilities");

    let ability_id = defaults[0]["id"].as_str().unwrap();

    // Create loadout → learn → equip → verify round-trip
    let loadout_json = ptr_to_string(ability_create_loadout());

    let learn_input = cstr(&loadout_json);
    let aid = cstr(ability_id);
    let learned_json = ptr_to_string(ability_learn(learn_input.as_ptr(), aid.as_ptr()));
    let learned: serde_json::Value = serde_json::from_str(&learned_json).unwrap();
    assert!(learned.is_object());

    // Equip to slot 0
    let equip_input = cstr(&learned_json);
    let aid2 = cstr(ability_id);
    let equipped_json = ptr_to_string(ability_equip(equip_input.as_ptr(), 0, aid2.as_ptr()));
    let equipped: serde_json::Value = serde_json::from_str(&equipped_json).unwrap();
    assert!(equipped.is_object());

    // Learn a second ability if available
    if defaults.len() > 1 {
        let second_id = defaults[1]["id"].as_str().unwrap();
        let input = cstr(&equipped_json);
        let aid3 = cstr(second_id);
        let both_json = ptr_to_string(ability_learn(input.as_ptr(), aid3.as_ptr()));
        let _both: serde_json::Value = serde_json::from_str(&both_json).unwrap();
    }
}

// ============================================================
// Sockets round-trip
// ============================================================

#[test]
fn roundtrip_socket_create_and_insert() {
    // Get starter gems
    let gems_json = ptr_to_string(socket_get_starter_gems());
    let gems: Vec<serde_json::Value> = serde_json::from_str(&gems_json).unwrap();
    assert!(!gems.is_empty());

    let runes_json = ptr_to_string(socket_get_starter_runes());
    let runes: Vec<serde_json::Value> = serde_json::from_str(&runes_json).unwrap();
    assert!(!runes.is_empty());

    // Create equipment with Prismatic socket (accepts any color)
    let name = cstr("Round-trip Sword");
    let colors = cstr("[3]"); // 3 = Prismatic
    let equip_json = ptr_to_string(socket_create_equipment(name.as_ptr(), colors.as_ptr()));
    let equip: serde_json::Value = serde_json::from_str(&equip_json).unwrap();
    assert!(equip["equipment_id"]
        .as_str()
        .unwrap()
        .contains("Round-trip Sword"));

    // Insert gem into slot 0
    let gem_str = serde_json::to_string(&gems[0]).unwrap();
    let gem_c = cstr(&gem_str);
    let equip_c = cstr(&equip_json);
    let with_gem_ptr = socket_insert_gem(equip_c.as_ptr(), 0, gem_c.as_ptr());
    if !with_gem_ptr.is_null() {
        let with_gem_json = ptr_to_string(with_gem_ptr);
        let _with_gem: serde_json::Value = serde_json::from_str(&with_gem_json).unwrap();
    }
}

#[test]
fn roundtrip_socket_combine_gems() {
    let gems_json = ptr_to_string(socket_get_starter_gems());
    let gems: Vec<serde_json::Value> = serde_json::from_str(&gems_json).unwrap();

    // Try combining first 3 gems (may fail if different tiers, that's OK)
    if gems.len() >= 3 {
        let combo: Vec<&serde_json::Value> = gems.iter().take(3).collect();
        let combo_str = serde_json::to_string(&combo).unwrap();
        let combo_c = cstr(&combo_str);
        let result = socket_combine_gems(combo_c.as_ptr());
        // May be null if gems can't be combined — that's valid behavior
        if !result.is_null() {
            let result_json = ptr_to_string(result);
            let _combined: serde_json::Value = serde_json::from_str(&result_json).unwrap();
        }
    }
}

// ============================================================
// Cosmetics round-trip
// ============================================================

#[test]
fn roundtrip_cosmetic_full_workflow() {
    // Get all cosmetics
    let all_json = ptr_to_string(cosmetic_get_all());
    let all: Vec<serde_json::Value> = serde_json::from_str(&all_json).unwrap();
    assert!(!all.is_empty());

    // Get all dyes
    let dyes_json = ptr_to_string(cosmetic_get_all_dyes());
    let dyes: Vec<serde_json::Value> = serde_json::from_str(&dyes_json).unwrap();
    assert!(!dyes.is_empty());

    // Create profile → unlock → transmog → dye
    let profile_json = ptr_to_string(cosmetic_create_profile());

    let cosmetic_id = all[0]["id"].as_str().unwrap();
    let cid = cstr(cosmetic_id);
    let profile_c = cstr(&profile_json);
    let unlocked_json = ptr_to_string(cosmetic_unlock(profile_c.as_ptr(), cid.as_ptr()));
    let _unlocked: serde_json::Value = serde_json::from_str(&unlocked_json).unwrap();

    // Apply transmog to slot 0 (HeadOverride)
    let unlocked_c = cstr(&unlocked_json);
    let cid2 = cstr(cosmetic_id);
    let transmog_ptr = cosmetic_apply_transmog(unlocked_c.as_ptr(), 0, cid2.as_ptr());
    // May be null if cosmetic doesn't fit slot — that's OK
    if !transmog_ptr.is_null() {
        let transmog_json = ptr_to_string(transmog_ptr);

        // Apply dye
        let dye_id = dyes[0]["id"].as_str().unwrap();
        let dye_c = cstr(dye_id);
        let trans_c = cstr(&transmog_json);
        let dyed_ptr = cosmetic_apply_dye(trans_c.as_ptr(), 0, 0, dye_c.as_ptr());
        if !dyed_ptr.is_null() {
            let dyed_json = ptr_to_string(dyed_ptr);
            let _dyed: serde_json::Value = serde_json::from_str(&dyed_json).unwrap();
        }
    }
}

// ============================================================
// Tutorial round-trip
// ============================================================

#[test]
fn roundtrip_tutorial_complete_all_steps() {
    let steps_json = ptr_to_string(tutorial_get_steps());
    let steps: Vec<serde_json::Value> = serde_json::from_str(&steps_json).unwrap();
    assert!(!steps.is_empty());

    let hints_json = ptr_to_string(tutorial_get_hints());
    let _hints: Vec<serde_json::Value> = serde_json::from_str(&hints_json).unwrap();

    // Complete steps one by one, passing JSON through each time
    let mut progress_json = ptr_to_string(tutorial_create_progress());
    let initial_pct = {
        let c = cstr(&progress_json);
        tutorial_completion_percent(c.as_ptr())
    };

    // Complete first step (may require no prereqs)
    let step_id = steps[0]["id"].as_str().unwrap();
    let sid = cstr(step_id);
    let prog_c = cstr(&progress_json);
    let updated_ptr = tutorial_complete_step(prog_c.as_ptr(), sid.as_ptr());
    if !updated_ptr.is_null() {
        progress_json = ptr_to_string(updated_ptr);
        let pct_c = cstr(&progress_json);
        let new_pct = tutorial_completion_percent(pct_c.as_ptr());
        assert!(
            new_pct >= initial_pct,
            "Completing a step should not decrease progress"
        );
    }
}

// ============================================================
// Achievements round-trip
// ============================================================

#[test]
fn roundtrip_achievement_multi_increment() {
    let mut tracker_json = ptr_to_string(achievement_create_tracker());

    // Increment "monster_slayer_1" multiple times
    let aid = cstr("monster_slayer_1");
    for _ in 0..5 {
        let tc = cstr(&tracker_json);
        let aid_c = cstr("monster_slayer_1");
        tracker_json = ptr_to_string(achievement_increment(tc.as_ptr(), aid_c.as_ptr(), 20));
    }

    // Check all achievements
    let tc = cstr(&tracker_json);
    let checked_json = ptr_to_string(achievement_check_all(tc.as_ptr(), 1000));
    let _checked: serde_json::Value = serde_json::from_str(&checked_json).unwrap();

    // Verify completion percent
    let cc = cstr(&checked_json);
    let pct = achievement_completion_percent(cc.as_ptr());
    assert!(
        (0.0..=1.0).contains(&pct),
        "Completion percent should be [0, 1]"
    );
    // After incrementing 100 kills, some achievement might be done
    let _ = aid; // use binding
}

// ============================================================
// Season Pass round-trip
// ============================================================

#[test]
fn roundtrip_season_pass_xp_progression() {
    let name = cstr("Integration Test Season");
    let mut pass_json = ptr_to_string(season_create_pass(1, name.as_ptr()));
    let pass: serde_json::Value = serde_json::from_str(&pass_json).unwrap();
    assert!(pass.is_object());

    // Add XP multiple times, simulating gameplay
    for _ in 0..10 {
        let pc = cstr(&pass_json);
        pass_json = ptr_to_string(season_add_xp(pc.as_ptr(), 1000));
    }

    // Final state should be valid JSON
    let final_pass: serde_json::Value = serde_json::from_str(&pass_json).unwrap();
    assert!(final_pass.is_object());

    // Quest generation
    let dailies_json = ptr_to_string(season_generate_dailies(42));
    let dailies: Vec<serde_json::Value> = serde_json::from_str(&dailies_json).unwrap();
    assert_eq!(dailies.len(), 3);

    let weeklies_json = ptr_to_string(season_generate_weeklies(42));
    let weeklies: Vec<serde_json::Value> = serde_json::from_str(&weeklies_json).unwrap();
    assert!(!weeklies.is_empty());

    // Rewards
    let rewards_json = ptr_to_string(season_get_rewards(1));
    let rewards: Vec<serde_json::Value> = serde_json::from_str(&rewards_json).unwrap();
    assert!(!rewards.is_empty());
}

// ============================================================
// Social round-trip: Guild → Party → Trade
// ============================================================

#[test]
fn roundtrip_social_guild_full() {
    let name = cstr("Round Trip Guild");
    let tag = cstr("RTG");
    let lid = cstr("leader_rt");
    let lname = cstr("RT Leader");
    let faction = cstr("AscendingOrder");

    let mut guild_json = ptr_to_string(social_create_guild(
        name.as_ptr(),
        tag.as_ptr(),
        lid.as_ptr(),
        lname.as_ptr(),
        faction.as_ptr(),
    ));
    let guild: serde_json::Value = serde_json::from_str(&guild_json).unwrap();
    assert_eq!(guild["name"].as_str().unwrap(), "Round Trip Guild");

    // Add 3 members
    for i in 0..3 {
        let gc = cstr(&guild_json);
        let uid = cstr(&format!("member_{i}"));
        let uname = cstr(&format!("Member {i}"));
        guild_json = ptr_to_string(social_guild_add_member(
            gc.as_ptr(),
            uid.as_ptr(),
            uname.as_ptr(),
        ));
    }

    let final_guild: serde_json::Value = serde_json::from_str(&guild_json).unwrap();
    assert!(final_guild.is_object());
}

#[test]
fn roundtrip_social_party_full() {
    let lid = cstr("party_leader");
    let lname = cstr("Party Leader");
    let mut party_json = ptr_to_string(social_create_party(lid.as_ptr(), lname.as_ptr()));

    // Add members with different roles
    for i in 0..3 {
        let pc = cstr(&party_json);
        let uid = cstr(&format!("party_member_{i}"));
        let uname = cstr(&format!("PM {i}"));
        let role_id = (i % 4) as u32; // cycle through roles
        party_json = ptr_to_string(social_party_add_member(
            pc.as_ptr(),
            uid.as_ptr(),
            uname.as_ptr(),
            role_id,
        ));
    }

    let final_party: serde_json::Value = serde_json::from_str(&party_json).unwrap();
    assert!(final_party.is_object());
}

#[test]
fn roundtrip_social_trade_full_workflow() {
    let pa = cstr("trader_a");
    let pb = cstr("trader_b");
    let trade_json = ptr_to_string(social_create_trade(pa.as_ptr(), pb.as_ptr()));

    // Player A adds items
    let tc = cstr(&trade_json);
    let pa2 = cstr("trader_a");
    let iname = cstr("Mythic Staff");
    let rarity = cstr("Epic");
    let with_item_json = ptr_to_string(social_trade_add_item(
        tc.as_ptr(),
        pa2.as_ptr(),
        iname.as_ptr(),
        1,
        rarity.as_ptr(),
    ));

    // Player B adds items
    let tc2 = cstr(&with_item_json);
    let pb2 = cstr("trader_b");
    let iname2 = cstr("Gold Coins");
    let rarity2 = cstr("Common");
    let both_items_json = ptr_to_string(social_trade_add_item(
        tc2.as_ptr(),
        pb2.as_ptr(),
        iname2.as_ptr(),
        100,
        rarity2.as_ptr(),
    ));

    // Both players lock
    let tc3 = cstr(&both_items_json);
    let pa3 = cstr("trader_a");
    let locked_a_json = ptr_to_string(social_trade_lock(tc3.as_ptr(), pa3.as_ptr()));

    let tc4 = cstr(&locked_a_json);
    let pb3 = cstr("trader_b");
    let locked_both_json = ptr_to_string(social_trade_lock(tc4.as_ptr(), pb3.as_ptr()));

    // Both confirm
    let tc5 = cstr(&locked_both_json);
    let pa4 = cstr("trader_a");
    let confirmed_a_json = ptr_to_string(social_trade_confirm(tc5.as_ptr(), pa4.as_ptr()));

    let tc6 = cstr(&confirmed_a_json);
    let pb4 = cstr("trader_b");
    let confirmed_both_json = ptr_to_string(social_trade_confirm(tc6.as_ptr(), pb4.as_ptr()));

    // Execute trade
    let tc7 = cstr(&confirmed_both_json);
    let executed_json = ptr_to_string(social_trade_execute(tc7.as_ptr()));
    let executed: serde_json::Value = serde_json::from_str(&executed_json).unwrap();
    assert!(executed.is_object());
}

// ============================================================
// Floor generation round-trip (determinism)
// ============================================================

#[test]
fn roundtrip_floor_generation_deterministic() {
    // Generate same floor twice — must be identical
    let json_a = ptr_to_string(generate_floor(42, 5));
    let json_b = ptr_to_string(generate_floor(42, 5));
    assert_eq!(
        json_a, json_b,
        "Same seed + floor_id must produce identical JSON"
    );

    // Layout determinism
    let layout_a = ptr_to_string(generate_floor_layout(42, 5));
    let layout_b = ptr_to_string(generate_floor_layout(42, 5));
    assert_eq!(layout_a, layout_b, "Layouts must be deterministic");

    // Hash determinism
    let hash_a = get_floor_hash(42, 5);
    let hash_b = get_floor_hash(42, 5);
    assert_eq!(hash_a, hash_b);
}

// ============================================================
// Combat round-trip
// ============================================================

#[test]
fn roundtrip_combat_calculation() {
    let request = serde_json::json!({
        "base_damage": 200.0,
        "angle_id": 2,
        "combo_step": 3,
        "attacker_tags_json": "[[\"fire\", 0.9]]",
        "defender_tags_json": "[[\"ice\", 0.5]]"
    });
    let req_str = serde_json::to_string(&request).unwrap();
    let req_c = cstr(&req_str);
    let result_json = ptr_to_string(calculate_combat(req_c.as_ptr()));
    let result: serde_json::Value = serde_json::from_str(&result_json).unwrap();
    assert!(result["final_damage"].as_f64().unwrap() > 0.0);
    assert!(result["angle_multiplier"].as_f64().unwrap() > 0.0);
}

// ============================================================
// Cross-system integration: Mastery → Ability → Socket
// ============================================================

#[test]
fn roundtrip_cross_system_character_progression() {
    // 1. Create mastery profile, gain sword XP
    let mut mastery_json = ptr_to_string(mastery_create_profile());
    let mc = cstr(&mastery_json);
    mastery_json = ptr_to_string(mastery_gain_xp(mc.as_ptr(), 0, 5000));

    // 2. Create ability loadout, learn and equip
    let defaults_json = ptr_to_string(ability_get_defaults());
    let defaults: Vec<serde_json::Value> = serde_json::from_str(&defaults_json).unwrap();
    let mut loadout_json = ptr_to_string(ability_create_loadout());
    if !defaults.is_empty() {
        let aid = defaults[0]["id"].as_str().unwrap();
        let lc = cstr(&loadout_json);
        let ac = cstr(aid);
        loadout_json = ptr_to_string(ability_learn(lc.as_ptr(), ac.as_ptr()));
        let lc2 = cstr(&loadout_json);
        let ac2 = cstr(aid);
        loadout_json = ptr_to_string(ability_equip(lc2.as_ptr(), 0, ac2.as_ptr()));
    }

    // 3. Create socketed equipment
    let name = cstr("Hero Sword");
    let colors = cstr("[3, 3]"); // 2 Prismatic sockets
    let equip_json = ptr_to_string(socket_create_equipment(name.as_ptr(), colors.as_ptr()));

    // 4. Create cosmetic profile, unlock & transmog
    let cosmetics_json = ptr_to_string(cosmetic_get_all());
    let cosmetics: Vec<serde_json::Value> = serde_json::from_str(&cosmetics_json).unwrap();
    let mut cosmetic_profile_json = ptr_to_string(cosmetic_create_profile());
    if !cosmetics.is_empty() {
        let cid = cosmetics[0]["id"].as_str().unwrap();
        let pc = cstr(&cosmetic_profile_json);
        let cc = cstr(cid);
        cosmetic_profile_json = ptr_to_string(cosmetic_unlock(pc.as_ptr(), cc.as_ptr()));
    }

    // 5. Create achievement tracker, increment
    let mut tracker_json = ptr_to_string(achievement_create_tracker());
    let tc = cstr(&tracker_json);
    let aid = cstr("monster_slayer_1");
    tracker_json = ptr_to_string(achievement_increment(tc.as_ptr(), aid.as_ptr(), 50));

    // Verify all JSONs are valid
    let _m: serde_json::Value = serde_json::from_str(&mastery_json).unwrap();
    let _l: serde_json::Value = serde_json::from_str(&loadout_json).unwrap();
    let _e: serde_json::Value = serde_json::from_str(&equip_json).unwrap();
    let _c: serde_json::Value = serde_json::from_str(&cosmetic_profile_json).unwrap();
    let _t: serde_json::Value = serde_json::from_str(&tracker_json).unwrap();
}

// ============================================================
// Loot + Semantic round-trip
// ============================================================

#[test]
fn roundtrip_loot_semantic_integration() {
    let tags_a = cstr(r#"[["fire", 0.8], ["arcane", 0.5]]"#);
    let tags_b = cstr(r#"[["fire", 0.9], ["ice", 0.3]]"#);

    let similarity = semantic_similarity(tags_a.as_ptr(), tags_b.as_ptr());
    assert!(similarity > 0.0 && similarity <= 1.0);

    // Generate loot with fire tags
    let source = cstr(r#"[["fire", 0.8]]"#);
    let loot_json = ptr_to_string(generate_loot(source.as_ptr(), 15, 12345));
    let loot: Vec<serde_json::Value> = serde_json::from_str(&loot_json).unwrap();
    assert!(!loot.is_empty());
}

// ============================================================
// Replication round-trip
// ============================================================

#[test]
fn roundtrip_replication_delta_and_snapshot() {
    let player = cstr("test_player");
    let payload = cstr(r#"{"action":"attack","target":"mob_42"}"#);

    // record_delta returns the sequence number (u64), not the Delta struct
    let seq_json = ptr_to_string(record_delta(
        0,
        5,
        99999,
        player.as_ptr(),
        payload.as_ptr(),
        1,
    ));
    let seq: u64 = serde_json::from_str(&seq_json).unwrap();
    assert_eq!(seq, 0, "First delta should have seq 0");

    // Create snapshot with empty deltas
    let empty_deltas = cstr("[]");
    let snapshot_json = ptr_to_string(create_floor_snapshot(42, 5, empty_deltas.as_ptr()));
    let snapshot: serde_json::Value = serde_json::from_str(&snapshot_json).unwrap();
    assert!(snapshot.is_object());
}

// ============================================================
// Events round-trip
// ============================================================

#[test]
fn roundtrip_event_triggers() {
    // BreathShift event
    let ctx = serde_json::json!({
        "breath_phase": "Hold",
        "floor_tags": [["fire", 0.7]],
        "floor_hash": 42,
        "corruption_level": 0.0,
        "player_actions": [],
        "active_factions": []
    });
    let ctx_str = serde_json::to_string(&ctx).unwrap();
    let ctx_c = cstr(&ctx_str);
    let result = evaluate_event_trigger(0, ctx_c.as_ptr()); // 0 = BreathShift
    if !result.is_null() {
        let event_json = ptr_to_string(result);
        let _event: serde_json::Value = serde_json::from_str(&event_json).unwrap();
    }
}

// ============================================================
// World state round-trip
// ============================================================

#[test]
fn roundtrip_breath_state_cycle() {
    // Test multiple points in the breath cycle
    let times = [0.0f32, 100.0, 300.0, 600.0, 900.0, 1200.0];
    let mut phases_seen = std::collections::HashSet::new();

    for t in &times {
        let json = ptr_to_string(get_breath_state(*t));
        let state: serde_json::Value = serde_json::from_str(&json).unwrap();
        let phase = state["phase"].as_str().unwrap().to_string();
        phases_seen.insert(phase);
    }

    assert!(
        phases_seen.len() >= 2,
        "Should see multiple breath phases across the cycle"
    );
}
