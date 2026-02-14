use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::ffi::{CStr, CString};
use tower_core::bridge::*;

fn bench_floor_generation(c: &mut Criterion) {
    c.bench_function("generate_floor", |b| {
        b.iter(|| {
            let ptr = generate_floor(black_box(42), black_box(1));
            free_string(ptr);
        })
    });

    c.bench_function("generate_floor_layout", |b| {
        b.iter(|| {
            let ptr = generate_floor_layout(black_box(42), black_box(1));
            free_string(ptr);
        })
    });

    c.bench_function("get_floor_hash", |b| {
        b.iter(|| {
            get_floor_hash(black_box(42), black_box(1));
        })
    });
}

fn bench_monster_generation(c: &mut Criterion) {
    c.bench_function("generate_monster", |b| {
        b.iter(|| {
            let ptr = generate_monster(black_box(12345), black_box(10));
            free_string(ptr);
        })
    });

    c.bench_function("generate_floor_monsters_5", |b| {
        b.iter(|| {
            let ptr = generate_floor_monsters(black_box(42), black_box(5), black_box(5));
            free_string(ptr);
        })
    });
}

fn bench_combat(c: &mut Criterion) {
    let request_json = serde_json::json!({
        "base_damage": 100.0,
        "angle_id": 2,
        "combo_step": 1,
        "attacker_tags_json": "[[\"fire\", 0.8]]",
        "defender_tags_json": "[[\"water\", 0.9]]"
    });
    let request_str = CString::new(serde_json::to_string(&request_json).unwrap()).unwrap();

    c.bench_function("calculate_combat", |b| {
        b.iter(|| {
            let ptr = calculate_combat(black_box(request_str.as_ptr()));
            free_string(ptr);
        })
    });

    c.bench_function("get_angle_multiplier", |b| {
        b.iter(|| {
            get_angle_multiplier(black_box(2));
        })
    });
}

fn bench_semantic(c: &mut Criterion) {
    let tags_a = CString::new(r#"[["fire", 0.8], ["arcane", 0.5]]"#).unwrap();
    let tags_b = CString::new(r#"[["fire", 0.9], ["ice", 0.3]]"#).unwrap();

    c.bench_function("semantic_similarity", |b| {
        b.iter(|| {
            semantic_similarity(black_box(tags_a.as_ptr()), black_box(tags_b.as_ptr()));
        })
    });
}

fn bench_loot(c: &mut Criterion) {
    let tags = CString::new(r#"[["fire", 0.8], ["corruption", 0.3]]"#).unwrap();

    c.bench_function("generate_loot", |b| {
        b.iter(|| {
            let ptr = generate_loot(black_box(tags.as_ptr()), black_box(10), black_box(42));
            free_string(ptr);
        })
    });
}

fn bench_mastery(c: &mut Criterion) {
    c.bench_function("mastery_create_profile", |b| {
        b.iter(|| {
            let ptr = mastery_create_profile();
            free_string(ptr);
        })
    });

    // Create a profile once, then bench gain_xp on it
    let profile_ptr = mastery_create_profile();
    let profile_json = unsafe { CStr::from_ptr(profile_ptr).to_str().unwrap().to_owned() };
    free_string(profile_ptr);
    let profile_cstr = CString::new(profile_json).unwrap();

    c.bench_function("mastery_gain_xp", |b| {
        b.iter(|| {
            let ptr = mastery_gain_xp(
                black_box(profile_cstr.as_ptr()),
                black_box(0),
                black_box(100),
            );
            free_string(ptr);
        })
    });
}

fn bench_json_roundtrip(c: &mut Criterion) {
    // Full mastery round-trip: create → serialize → deserialize → modify → serialize
    c.bench_function("mastery_full_roundtrip", |b| {
        b.iter(|| {
            let create_ptr = mastery_create_profile();
            let updated_ptr = mastery_gain_xp(create_ptr, 0, 500);
            let _ = mastery_get_tier(updated_ptr, 0);
            free_string(create_ptr);
            free_string(updated_ptr);
        })
    });

    // Full ability round-trip: create loadout → learn → equip
    let defaults_ptr = ability_get_defaults();
    let defaults_json = unsafe { CStr::from_ptr(defaults_ptr).to_str().unwrap().to_owned() };
    free_string(defaults_ptr);
    let defaults: Vec<serde_json::Value> = serde_json::from_str(&defaults_json).unwrap();
    let ability_id = CString::new(defaults[0]["id"].as_str().unwrap()).unwrap();

    c.bench_function("ability_full_roundtrip", |b| {
        b.iter(|| {
            let loadout = ability_create_loadout();
            let learned = ability_learn(loadout, black_box(ability_id.as_ptr()));
            let equipped = ability_equip(learned, 0, black_box(ability_id.as_ptr()));
            free_string(loadout);
            free_string(learned);
            if !equipped.is_null() {
                free_string(equipped);
            }
        })
    });
}

fn bench_social(c: &mut Criterion) {
    let name = CString::new("Bench Guild").unwrap();
    let tag = CString::new("BG").unwrap();
    let lid = CString::new("leader1").unwrap();
    let lname = CString::new("Leader").unwrap();
    let faction = CString::new("AscendingOrder").unwrap();

    c.bench_function("social_create_guild", |b| {
        b.iter(|| {
            let ptr = social_create_guild(
                black_box(name.as_ptr()),
                black_box(tag.as_ptr()),
                black_box(lid.as_ptr()),
                black_box(lname.as_ptr()),
                black_box(faction.as_ptr()),
            );
            free_string(ptr);
        })
    });

    let pa = CString::new("player_a").unwrap();
    let pb = CString::new("player_b").unwrap();

    c.bench_function("social_create_trade", |b| {
        b.iter(|| {
            let ptr = social_create_trade(black_box(pa.as_ptr()), black_box(pb.as_ptr()));
            free_string(ptr);
        })
    });
}

criterion_group!(
    benches,
    bench_floor_generation,
    bench_monster_generation,
    bench_combat,
    bench_semantic,
    bench_loot,
    bench_mastery,
    bench_json_roundtrip,
    bench_social,
);
criterion_main!(benches);
