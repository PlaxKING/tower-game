//! End-to-End Integration Tests
//!
//! Tests the full pipeline: generation → combat → loot → storage
//! Validates that all systems work together correctly.

use tower_bevy_server::combat;
use tower_bevy_server::monster_gen;
use tower_bevy_server::loot;
use tower_bevy_server::destruction;
use tower_bevy_server::ecs_bridge;
use tower_bevy_server::semantic_tags::SemanticTags;
use std::collections::HashMap;

// ============================================================================
// Full Pipeline: Floor → Monsters → Combat → Loot
// ============================================================================

#[test]
fn test_full_floor_pipeline() {
    let tower_seed = 42u64;
    let floor_id = 5u32;
    let biome_tags = vec![
        ("forest".to_string(), 0.8),
        ("nature".to_string(), 0.6),
    ];

    // Step 1: Generate monsters for this floor
    let monsters = monster_gen::generate_room_monsters(tower_seed, floor_id, 1, &biome_tags, 5);
    assert_eq!(monsters.len(), 5);

    for monster in &monsters {
        // Every monster should have valid stats
        assert!(monster.max_health > 0.0, "Monster {} has 0 HP", monster.name);
        assert!(monster.base_damage > 0.0, "Monster {} has 0 damage", monster.name);
        assert!(!monster.name.is_empty());

        // Every monster should inherit floor biome tags
        assert!(monster.semantic_tags.contains_key("aggression"));
        assert!(monster.semantic_tags.contains_key("presence"));

        // Step 2: Simulate combat with this monster
        let mut combat_state = combat::CombatState::default();
        let weapon = combat::EquippedWeapon {
            weapon_type: combat::WeaponType::Sword,
            weapon_id: "player_sword".into(),
            base_damage: 30.0,
            attack_speed: 1.0,
            range: 2.0,
        };
        let movesets = combat::WeaponMovesets::default();

        // Attack combo
        let r1 = combat::try_combat_action(
            &mut combat_state, combat::ActionType::Attack, &weapon, &movesets,
        );
        assert!(r1.success);
        assert_eq!(r1.combo_step, 0);

        // Simulate damage calculation
        let attack = r1.attack_data.unwrap();
        let damage = combat::calculate_damage(
            weapon.base_damage,
            &attack,
            combat::AttackAngle::Back, // Backstab
            0,
            0.2, // Semantic affinity bonus
            false,
            false,
        );
        assert!(damage.final_damage > weapon.base_damage, "Back attack should deal more damage");
        assert_eq!(damage.angle, combat::AttackAngle::Back);

        // Step 3: Generate loot from killed monster
        let loot_config = loot::LootConfig {
            floor_id,
            luck: 0.0,
            semantic_affinity: 0.3,
            loot_tier: monster.loot_tier,
            monster_tags: monster.semantic_tags.clone(),
        };
        let drops = loot::generate_loot(monster.variant_id, &loot_config);
        assert!(!drops.is_empty(), "Killed monster should drop loot");

        for drop in &drops {
            assert!(!drop.item_id.is_empty());
            assert!(drop.quantity > 0);
            assert!(drop.gold_value > 0);
        }
    }
}

// ============================================================================
// Combat System Integration
// ============================================================================

#[test]
fn test_combat_parry_counter_sequence() {
    let mut defender = combat::CombatState::default();
    let mut attacker = combat::CombatState::default();
    let weapon = combat::EquippedWeapon {
        weapon_type: combat::WeaponType::Sword,
        weapon_id: "test".into(),
        base_damage: 50.0,
        attack_speed: 1.0,
        range: 2.0,
    };
    let movesets = combat::WeaponMovesets::default();

    // Attacker attacks
    let atk = combat::try_combat_action(
        &mut attacker, combat::ActionType::Attack, &weapon, &movesets,
    );
    assert!(atk.success);

    // Defender parries
    let parry = combat::try_combat_action(
        &mut defender, combat::ActionType::Parry, &weapon, &movesets,
    );
    assert!(parry.success);
    assert!(defender.parry_window > 0.0);

    // Damage with parry active → 0 damage
    let attack_data = atk.attack_data.unwrap();
    let damage = combat::calculate_damage(
        50.0, &attack_data, combat::AttackAngle::Front, 0, 0.0,
        false, true, // target is parrying
    );
    assert!(damage.was_parried);
    assert_eq!(damage.final_damage, 0.0);

    // Attacker gets staggered from parry
    let mut attacker_health = 100.0;
    let stagger_damage = combat::DamageCalcResult {
        base_damage: 0.0,
        angle_mult: 1.0,
        combo_mult: 1.0,
        semantic_mult: 1.0,
        final_damage: 0.0,
        poise_damage: 100.0, // Full poise break
        knockback: 0.0,
        angle: combat::AttackAngle::Front,
        was_critical: false,
        was_blocked: false,
        was_parried: false,
    };
    let outcome = combat::apply_damage_to_target(
        &mut attacker, &mut attacker_health, &stagger_damage,
    );
    assert_eq!(outcome, combat::DamageOutcome::Staggered);

    // Defender counters while attacker is staggered
    defender.phase = combat::CombatPhase::Idle;
    let counter = combat::try_combat_action(
        &mut defender, combat::ActionType::Attack, &weapon, &movesets,
    );
    assert!(counter.success);

    // Mastery XP for parry should be highest
    let (domain, xp) = combat::mastery_xp_for_action(
        combat::ActionType::Parry, combat::DamageOutcome::Parried,
    );
    assert_eq!(domain, "ParryMastery");
    assert_eq!(xp, 50.0);
}

#[test]
fn test_weapon_type_differences() {
    let movesets = combat::WeaponMovesets::default();

    // Sword: fast, wide
    let sword = &movesets.movesets[&combat::WeaponType::Sword];
    // Spear: long range, narrow
    let spear = &movesets.movesets[&combat::WeaponType::Spear];
    // Hammer: slow, devastating
    let hammer = &movesets.movesets[&combat::WeaponType::Hammer];

    // Sword has most combo steps
    assert!(sword.len() > hammer.len());

    // Spear has longest reach
    assert!(spear[0].hitbox_offset > sword[0].hitbox_offset);

    // Hammer deals most poise damage
    assert!(hammer[0].poise_damage > sword[0].poise_damage);
    assert!(hammer[0].poise_damage > spear[0].poise_damage);

    // Hammer finisher is devastating
    assert!(hammer[1].damage_mult > sword[sword.len()-1].damage_mult);
}

// ============================================================================
// Monster Generation + Semantic Coherence
// ============================================================================

#[test]
fn test_biome_monster_coherence() {
    // Fire biome should produce fire-themed monsters more often
    let fire_biome = vec![("fire".to_string(), 0.9), ("volcanic".to_string(), 0.7)];
    let ice_biome = vec![("ice".to_string(), 0.9), ("frozen".to_string(), 0.7)];

    let fire_monsters = monster_gen::generate_room_monsters(42, 10, 1, &fire_biome, 20);
    let ice_monsters = monster_gen::generate_room_monsters(42, 10, 1, &ice_biome, 20);

    // Fire monsters should carry fire tags
    for m in &fire_monsters {
        // Should inherit fire biome at 70%
        let fire_tag = m.semantic_tags.get("fire");
        assert!(fire_tag.is_some() || m.semantic_tags.get("volcanic").is_some(),
            "Fire biome monster should have fire/volcanic tags");
    }

    // Ice monsters should carry ice tags
    for m in &ice_monsters {
        let ice_tag = m.semantic_tags.get("ice");
        assert!(ice_tag.is_some() || m.semantic_tags.get("frozen").is_some(),
            "Ice biome monster should have ice/frozen tags");
    }
}

#[test]
fn test_floor_depth_scaling() {
    let biome = vec![("dungeon".to_string(), 0.7)];

    let floor1 = monster_gen::generate_room_monsters(42, 1, 1, &biome, 10);
    let floor50 = monster_gen::generate_room_monsters(42, 50, 1, &biome, 10);

    let avg_hp_f1: f32 = floor1.iter().map(|m| m.max_health).sum::<f32>() / 10.0;
    let avg_hp_f50: f32 = floor50.iter().map(|m| m.max_health).sum::<f32>() / 10.0;

    // Floor 50 should have significantly higher HP monsters
    assert!(avg_hp_f50 > avg_hp_f1 * 3.0,
        "Floor 50 avg HP ({:.0}) should be >3x floor 1 ({:.0})", avg_hp_f50, avg_hp_f1);
}

// ============================================================================
// Loot + Semantic Integration
// ============================================================================

#[test]
fn test_loot_semantic_theme_consistency() {
    // Kill a fire monster → should get fire-themed drops
    let fire_tags = HashMap::from([
        ("fire".to_string(), 0.9),
        ("volcanic".to_string(), 0.7),
        ("aggression".to_string(), 0.6),
    ]);

    let config = loot::LootConfig {
        floor_id: 10,
        luck: 0.0,
        semantic_affinity: 0.5,
        loot_tier: 2,
        monster_tags: fire_tags,
    };

    let mut fire_items = 0;
    let total_seeds = 100;
    for seed in 0..total_seeds {
        let drops = loot::generate_loot(seed, &config);
        for drop in &drops {
            if drop.item_id.contains("fire") || drop.display_name.contains("Ember") {
                fire_items += 1;
            }
        }
    }

    assert!(fire_items > total_seeds / 4,
        "Fire monsters should produce fire-themed items ({}/{})", fire_items, total_seeds);
}

#[test]
fn test_rarity_distribution_is_reasonable() {
    let config = loot::LootConfig {
        floor_id: 20,
        luck: 0.0,
        semantic_affinity: 0.0,
        loot_tier: 3,
        monster_tags: HashMap::from([("neutral".to_string(), 0.5)]),
    };

    let mut rarity_counts: HashMap<loot::Rarity, usize> = HashMap::new();
    for seed in 0..1000u64 {
        let drops = loot::generate_loot(seed, &config);
        for drop in &drops {
            *rarity_counts.entry(drop.rarity).or_insert(0) += 1;
        }
    }

    // Common should be most frequent
    let common = rarity_counts.get(&loot::Rarity::Common).copied().unwrap_or(0);
    let uncommon = rarity_counts.get(&loot::Rarity::Uncommon).copied().unwrap_or(0);
    let rare = rarity_counts.get(&loot::Rarity::Rare).copied().unwrap_or(0);

    assert!(common > uncommon, "Common ({}) should exceed Uncommon ({})", common, uncommon);
    assert!(uncommon > rare, "Uncommon ({}) should exceed Rare ({})", uncommon, rare);
}

// ============================================================================
// Destruction + Combat Integration
// ============================================================================

#[test]
fn test_destruction_with_combat_damage_types() {
    let mut manager = destruction::FloorDestructionManager::new();

    // Spawn a wooden wall
    let entity_id = manager.spawn("wall_wood_3m", 1,
        bevy::math::Vec3::new(0.0, 0.0, 0.0)).unwrap();

    // Fire damage should deal bonus to wood
    let fire_result = manager.apply_damage(
        entity_id, 1,
        bevy::math::Vec3::new(0.5, 0.5, 0.0), // impact
        bevy::math::Vec3::ZERO, // entity pos
        50.0,
        0.0,
        destruction::DestructionDamageType::ElementalFire,
    );

    assert!(fire_result.is_some());
    let fire_dmg = fire_result.unwrap();
    assert!(fire_dmg.damage_dealt > 0.0);

    // Spawn stone wall for comparison
    let stone_id = manager.spawn("wall_stone_3m", 1,
        bevy::math::Vec3::new(10.0, 0.0, 0.0)).unwrap();

    let stone_result = manager.apply_damage(
        stone_id, 1,
        bevy::math::Vec3::new(10.5, 0.5, 0.0),
        bevy::math::Vec3::new(10.0, 0.0, 0.0),
        50.0,
        0.0,
        destruction::DestructionDamageType::ElementalFire,
    );

    assert!(stone_result.is_some());
    // Fire should deal more to wood than stone
    assert!(fire_dmg.damage_dealt > stone_result.unwrap().damage_dealt,
        "Fire should be more effective against wood than stone");
}

// ============================================================================
// ECS Bridge Integration
// ============================================================================

#[tokio::test]
async fn test_ecs_bridge_combat_command() {
    let (tx, mut rx, _snapshot) = ecs_bridge::create_bridge();

    // Send a combat action command
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(ecs_bridge::GameCommand::CombatAction {
        player_id: 1,
        action: combat::ActionType::Attack,
        position: [0.0, 0.0, 0.0],
        facing: 0.0,
        reply: reply_tx,
    }).unwrap();

    // Receive on Bevy side
    let cmd = rx.receiver.recv().await.unwrap();
    match cmd {
        ecs_bridge::GameCommand::CombatAction { player_id, action, reply, .. } => {
            assert_eq!(player_id, 1);
            assert_eq!(action, combat::ActionType::Attack);
            reply.send(ecs_bridge::CombatActionCommandResult {
                success: true,
                action_result: None,
                message: "OK".into(),
            }).unwrap();
        }
        _ => panic!("Wrong command type"),
    }

    let result = reply_rx.await.unwrap();
    assert!(result.success);
}

#[tokio::test]
async fn test_ecs_bridge_spawn_and_query() {
    let (tx, mut rx, snapshot) = ecs_bridge::create_bridge();

    // Write player to snapshot
    {
        let mut snap = snapshot.write().unwrap();
        snap.players.insert(42, ecs_bridge::PlayerSnapshot {
            id: 42,
            position: [10.0, 0.0, 20.0],
            health: 85.0,
            max_health: 100.0,
            current_floor: 5,
            in_combat: true,
        });
        snap.tick = 100;
    }

    // Read from snapshot (API handler pattern)
    {
        let snap = snapshot.read().unwrap();
        assert_eq!(snap.tick, 100);
        let player = snap.players.get(&42).unwrap();
        assert_eq!(player.health, 85.0);
        assert!(player.in_combat);
    }

    // Query player through command channel
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    tx.send(ecs_bridge::GameCommand::GetPlayer {
        player_id: 42,
        reply: reply_tx,
    }).unwrap();

    let cmd = rx.receiver.recv().await.unwrap();
    match cmd {
        ecs_bridge::GameCommand::GetPlayer { player_id, reply } => {
            assert_eq!(player_id, 42);
            reply.send(Some(ecs_bridge::PlayerSnapshot {
                id: 42,
                position: [10.0, 0.0, 20.0],
                health: 85.0,
                max_health: 100.0,
                current_floor: 5,
                in_combat: true,
            })).unwrap();
        }
        _ => panic!("Wrong command"),
    }

    let player = reply_rx.await.unwrap();
    assert!(player.is_some());
    assert_eq!(player.unwrap().id, 42);
}

// ============================================================================
// Semantic Tag Similarity Pipeline
// ============================================================================

#[test]
fn test_semantic_similarity_affects_loot_quality() {
    let monster_tags = HashMap::from([
        ("fire".to_string(), 0.9),
        ("aggression".to_string(), 0.7),
    ]);

    // High affinity player
    let high_config = loot::LootConfig {
        floor_id: 10,
        luck: 0.0,
        semantic_affinity: 0.8,
        loot_tier: 2,
        monster_tags: monster_tags.clone(),
    };

    // Low affinity player
    let low_config = loot::LootConfig {
        floor_id: 10,
        luck: 0.0,
        semantic_affinity: 0.1,
        loot_tier: 2,
        monster_tags: monster_tags,
    };

    let mut high_total = 0usize;
    let mut low_total = 0usize;

    for seed in 0..200u64 {
        high_total += loot::generate_loot(seed, &high_config).len();
        low_total += loot::generate_loot(seed, &low_config).len();
    }

    // High affinity should yield more drops (due to bonus semantic drops)
    assert!(high_total >= low_total,
        "High affinity ({}) should yield >= drops than low ({})", high_total, low_total);
}

#[test]
fn test_semantic_tags_cosine_similarity() {
    let player = SemanticTags::from_pairs(vec![("fire", 0.8), ("combat", 0.6)]);
    let fire_monster = SemanticTags::from_pairs(vec![("fire", 0.9), ("aggression", 0.7)]);
    let ice_monster = SemanticTags::from_pairs(vec![("ice", 0.9), ("defense", 0.7)]);

    let fire_sim = player.similarity(&fire_monster);
    let ice_sim = player.similarity(&ice_monster);

    // Player with fire tags should be more similar to fire monster
    assert!(fire_sim > ice_sim,
        "Fire similarity ({:.3}) should exceed ice ({:.3})", fire_sim, ice_sim);
}

// ============================================================================
// Full Kill-to-Drop Pipeline
// ============================================================================

#[test]
fn test_kill_monster_full_pipeline() {
    let tower_seed = 99999u64;
    let floor_id = 15u32;
    let biome = vec![("volcanic".to_string(), 0.8), ("fire".to_string(), 0.9)];

    // Generate a monster
    let blueprint = monster_gen::generate_blueprint(tower_seed, floor_id, &biome);

    // Set up combat
    let mut player_state = combat::CombatState::default();
    let mut player_energy = combat::CombatEnergy::default();
    let weapon = combat::EquippedWeapon {
        weapon_type: combat::WeaponType::Hammer,
        weapon_id: "fire_hammer".into(),
        base_damage: 80.0,
        attack_speed: 0.8,
        range: 2.5,
    };
    let movesets = combat::WeaponMovesets::default();

    // Execute heavy attack
    let result = combat::try_combat_action(
        &mut player_state, combat::ActionType::HeavyAttack, &weapon, &movesets,
    );
    assert!(result.success);

    let attack = result.attack_data.unwrap();

    // Calculate damage with back attack + semantic bonus
    let damage = combat::calculate_damage(
        weapon.base_damage, &attack,
        combat::AttackAngle::Back,
        0, 0.3, // Semantic bonus
        false, false,
    );

    // Apply damage
    let mut monster_state = combat::CombatState { poise: 50.0, ..Default::default() };
    let mut monster_hp = blueprint.max_health;
    let outcome = combat::apply_damage_to_target(
        &mut monster_state, &mut monster_hp, &damage,
    );

    // Monster should take heavy damage (may or may not be killed depending on HP)
    assert!(damage.final_damage > 0.0);
    assert!(monster_hp < blueprint.max_health);

    // Generate energy from combat
    player_energy.gain_kinetic(damage.final_damage * 0.1);
    assert!(player_energy.kinetic > 0.0);

    // If killed, generate loot
    if matches!(outcome, combat::DamageOutcome::Killed) {
        let loot_config = loot::LootConfig {
            floor_id,
            luck: 0.0,
            semantic_affinity: 0.3,
            loot_tier: blueprint.loot_tier,
            monster_tags: blueprint.semantic_tags.clone(),
        };
        let drops = loot::generate_loot(blueprint.variant_id, &loot_config);
        assert!(!drops.is_empty());

        // Mastery XP
        let (domain, xp) = combat::mastery_xp_for_action(
            combat::ActionType::HeavyAttack, combat::DamageOutcome::Killed,
        );
        assert_eq!(domain, "WeaponMastery");
        assert_eq!(xp, 75.0); // Heavy attack kill = 75 XP
    }
}
