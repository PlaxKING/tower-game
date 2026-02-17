//! Integration tests for semantic tag system
//!
//! Tests the complete flow:
//! Floor generation → Semantic tagging → Monster inheritance → Ability interactions

use tower_bevy_server::async_generation::{FloorGenerator, GenerationConfig};
use tower_bevy_server::semantic_tags::{SemanticTags, MasteryDomain, DomainCategory};

#[tokio::test]
async fn test_floor_has_semantic_tags() {
    let config = GenerationConfig {
        cache_capacity: 10,
        worker_threads: 2,
        floor_size: 10,
        enable_warmup: false,
        warmup_count: 0,
        enable_lmdb: false,
        lmdb_path: None,
        lmdb_size: 0,
    };

    let generator = FloorGenerator::new(config);

    // Generate floor 1 (Plains biome)
    let chunk = generator.get_or_generate(1, 0x12345).await.unwrap();

    // Verify semantic tags exist
    assert!(chunk.semantic_tags.is_some(), "Floor should have semantic tags");

    let tags = chunk.semantic_tags.unwrap();
    assert!(!tags.tags.is_empty(), "Semantic tags should not be empty");

    // Verify plains tags
    let has_plains = tags.tags.iter().any(|t| t.tag == "plains");
    let has_exploration = tags.tags.iter().any(|t| t.tag == "exploration");

    assert!(has_plains, "Plains floor should have 'plains' tag");
    assert!(has_exploration, "Plains floor should have 'exploration' tag");

    println!("Floor 1 tags:");
    for tag_pair in tags.tags {
        println!("  - {}: {:.2}", tag_pair.tag, tag_pair.weight);
    }
}

#[tokio::test]
async fn test_biome_semantic_differences() {
    let config = GenerationConfig::default();
    let generator = FloorGenerator::new(config);

    // Generate floors from different biomes
    let plains = generator.get_or_generate(1, 0x1111).await.unwrap();     // Biome 1
    let forest = generator.get_or_generate(150, 0x1111).await.unwrap();   // Biome 2
    let desert = generator.get_or_generate(250, 0x1111).await.unwrap();   // Biome 3
    let volcano = generator.get_or_generate(800, 0x1111).await.unwrap();  // Biome 6

    // Extract tag names
    let plains_tags: Vec<String> = plains.semantic_tags.unwrap().tags.iter().map(|t| t.tag.clone()).collect();
    let forest_tags: Vec<String> = forest.semantic_tags.unwrap().tags.iter().map(|t| t.tag.clone()).collect();
    let desert_tags: Vec<String> = desert.semantic_tags.unwrap().tags.iter().map(|t| t.tag.clone()).collect();
    let volcano_tags: Vec<String> = volcano.semantic_tags.unwrap().tags.iter().map(|t| t.tag.clone()).collect();

    // Verify biome-specific tags
    assert!(plains_tags.contains(&"plains".to_string()));
    assert!(forest_tags.contains(&"forest".to_string()));
    assert!(desert_tags.contains(&"desert".to_string()));
    assert!(volcano_tags.contains(&"volcano".to_string()));

    // Verify different biomes have different tag sets
    assert_ne!(plains_tags, forest_tags, "Different biomes should have different tags");
    assert_ne!(desert_tags, volcano_tags, "Different biomes should have different tags");
}

#[tokio::test]
async fn test_corruption_progression() {
    let config = GenerationConfig::default();
    let generator = FloorGenerator::new(config);

    // Generate floors at different depths
    let floor_1 = generator.get_or_generate(1, 0x9999).await.unwrap();
    let floor_500 = generator.get_or_generate(500, 0x9999).await.unwrap();
    let floor_1000 = generator.get_or_generate(1000, 0x9999).await.unwrap();

    // Helper to get tag weight
    let get_weight = |chunk: &tower_bevy_server::proto::tower::game::ChunkData, tag_name: &str| -> f32 {
        chunk.semantic_tags.as_ref()
            .and_then(|tags| tags.tags.iter().find(|t| t.tag == tag_name))
            .map(|t| t.weight)
            .unwrap_or(0.0)
    };

    let corruption_1 = get_weight(&floor_1, "corruption");
    let corruption_500 = get_weight(&floor_500, "corruption");
    let corruption_1000 = get_weight(&floor_1000, "corruption");

    // Corruption should increase with depth
    assert!(corruption_1 < corruption_500, "Corruption should increase with floor depth");
    assert!(corruption_500 < corruption_1000, "Corruption should increase with floor depth");

    println!("Corruption progression:");
    println!("  Floor 1:    {:.3}", corruption_1);
    println!("  Floor 500:  {:.3}", corruption_500);
    println!("  Floor 1000: {:.3}", corruption_1000);
}

#[test]
fn test_mastery_domain_similarity() {
    // Weapons should be similar to each other
    let sword = MasteryDomain::SwordMastery.to_tags();
    let axe = MasteryDomain::AxeMastery.to_tags();
    let bow = MasteryDomain::BowMastery.to_tags();

    let sword_axe_sim = sword.similarity(&axe);
    let sword_bow_sim = sword.similarity(&bow);

    // Melee weapons (sword, axe) should be more similar than melee vs ranged (sword, bow)
    assert!(
        sword_axe_sim > sword_bow_sim,
        "Melee weapons should be more similar to each other than to ranged weapons"
    );

    println!("Domain similarity:");
    println!("  Sword ↔ Axe: {:.3}", sword_axe_sim);
    println!("  Sword ↔ Bow: {:.3}", sword_bow_sim);
}

#[test]
fn test_mastery_domain_categories() {
    let all_domains = MasteryDomain::all();
    assert_eq!(all_domains.len(), 21, "Should have exactly 21 mastery domains");

    // Count domains per category
    let mut weapon_count = 0;
    let mut combat_count = 0;
    let mut crafting_count = 0;
    let mut gathering_count = 0;
    let mut other_count = 0;

    for domain in &all_domains {
        match domain.category() {
            DomainCategory::Weapon => weapon_count += 1,
            DomainCategory::Combat => combat_count += 1,
            DomainCategory::Crafting => crafting_count += 1,
            DomainCategory::Gathering => gathering_count += 1,
            DomainCategory::Other => other_count += 1,
        }
    }

    // Verify distribution (from CLAUDE.md)
    assert_eq!(weapon_count, 7, "Should have 7 weapon domains");
    assert_eq!(combat_count, 5, "Should have 5 combat technique domains");
    assert_eq!(crafting_count, 3, "Should have 3 crafting domains");
    assert_eq!(gathering_count, 3, "Should have 3 gathering domains");
    assert_eq!(other_count, 3, "Should have 3 other domains");

    println!("Domain distribution:");
    println!("  Weapon:    {}", weapon_count);
    println!("  Combat:    {}", combat_count);
    println!("  Crafting:  {}", crafting_count);
    println!("  Gathering: {}", gathering_count);
    println!("  Other:     {}", other_count);
}

#[test]
fn test_semantic_tag_blending() {
    // Simulate fire floor + water ability interaction
    let fire_floor = SemanticTags::from_pairs(vec![
        ("fire", 0.9),
        ("heat", 0.8),
        ("danger", 0.7),
    ]);

    let water_ability = SemanticTags::from_pairs(vec![
        ("water", 0.9),
        ("cold", 0.6),
    ]);

    // Blend for "steam" effect
    let steam = fire_floor.blend(&water_ability, 0.5);

    assert!(steam.get("fire") > 0.0, "Steam should have some fire");
    assert!(steam.get("water") > 0.0, "Steam should have some water");
    assert!(steam.get("heat") > 0.0, "Steam should retain heat from fire");

    println!("Tag blending (Fire + Water = Steam):");
    println!("  fire:  {:.2}", steam.get("fire"));
    println!("  water: {:.2}", steam.get("water"));
    println!("  heat:  {:.2}", steam.get("heat"));
    println!("  cold:  {:.2}", steam.get("cold"));
}

#[test]
fn test_conflicting_elements() {
    let fire = SemanticTags::from_pairs(vec![("fire", 1.0)]);
    let water = SemanticTags::from_pairs(vec![("water", 1.0)]);
    let ice = SemanticTags::from_pairs(vec![("ice", 1.0)]);

    // Fire and water have no shared tags = low similarity
    let fire_water = fire.similarity(&water);
    assert!(fire_water < 0.1, "Fire and water should have low similarity (orthogonal)");

    // Fire and ice also orthogonal
    let fire_ice = fire.similarity(&ice);
    assert!(fire_ice < 0.1, "Fire and ice should have low similarity (orthogonal)");

    println!("Conflicting element similarities:");
    println!("  Fire ↔ Water: {:.3}", fire_water);
    println!("  Fire ↔ Ice:   {:.3}", fire_ice);
}

#[test]
fn test_tag_normalization() {
    let tags = SemanticTags::from_pairs(vec![
        ("a", 2.0),
        ("b", 3.0),
        ("c", 5.0),
    ]);

    // Before normalization, weights are clamped to 1.0
    assert_eq!(tags.get("a"), 1.0);
    assert_eq!(tags.get("b"), 1.0);
    assert_eq!(tags.get("c"), 1.0);

    // Recreate with smaller values for normalization test
    let mut tags = SemanticTags::from_pairs(vec![
        ("a", 0.2),
        ("b", 0.3),
        ("c", 0.5),
    ]);

    tags.normalize();

    // After normalization, sum should be 1.0
    let sum: f32 = tags.tags.iter().map(|(_, w)| w).sum();
    assert!((sum - 1.0).abs() < 0.001, "Normalized tags should sum to 1.0");

    println!("Normalized tag weights:");
    for (tag, weight) in &tags.tags {
        println!("  {}: {:.3}", tag, weight);
    }
}

#[tokio::test]
async fn test_deterministic_floor_tags() {
    let config = GenerationConfig::default();
    let generator = FloorGenerator::new(config);

    // Generate same floor twice with same seed
    let chunk1 = generator.get_or_generate(42, 0xABCD).await.unwrap();

    // Create a new generator to force regeneration (clean cache)
    let config2 = GenerationConfig::default();
    let generator2 = FloorGenerator::new(config2);

    let chunk2 = generator2.get_or_generate(42, 0xABCD).await.unwrap();

    // Semantic tags should be identical (deterministic)
    let tags1 = chunk1.semantic_tags.unwrap();
    let tags2 = chunk2.semantic_tags.unwrap();

    assert_eq!(tags1.tags.len(), tags2.tags.len(), "Tag count should be identical");

    for (t1, t2) in tags1.tags.iter().zip(tags2.tags.iter()) {
        assert_eq!(t1.tag, t2.tag, "Tag names should match");
        assert!((t1.weight - t2.weight).abs() < 0.001, "Tag weights should match");
    }

    println!("✅ Deterministic tag generation verified");
}
