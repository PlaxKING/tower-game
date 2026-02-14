//! Crafting system.
//!
//! Combines materials + semantic tags to produce equipment.
//! Recipes are discovered through semantic combinations, not preset lists.

use serde::{Deserialize, Serialize};

use crate::economy::ItemRarity;
use crate::semantic::SemanticTags;

/// Crafting recipe — discovered by combining materials with matching tags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftingRecipe {
    pub name: String,
    pub required_tags: Vec<(String, f32)>,
    pub material_count: u32,
    pub min_rarity: ItemRarity,
    pub result_category: CraftResultCategory,
    pub shard_cost: u64,
}

/// What the crafting produces
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CraftResultCategory {
    Weapon,
    Armor,
    Accessory,
    Consumable,
    Enhancement,
}

/// Result of a crafting attempt
#[derive(Debug, Clone)]
pub struct CraftResult {
    pub name: String,
    pub category: CraftResultCategory,
    pub rarity: ItemRarity,
    pub quality: f32, // 0.0-1.0 based on material quality
    pub semantic_tags: Vec<(String, f32)>,
}

/// Semantic-based crafting: combine materials based on tag similarity
pub fn attempt_craft(
    materials: &[CraftMaterial],
    recipe: &CraftingRecipe,
) -> Result<CraftResult, CraftError> {
    // Check material count
    if materials.len() < recipe.material_count as usize {
        return Err(CraftError::NotEnoughMaterials {
            required: recipe.material_count,
            provided: materials.len() as u32,
        });
    }

    // Check semantic tag requirements
    let combined_tags = combine_material_tags(materials);
    let recipe_tags = SemanticTags::new(
        recipe
            .required_tags
            .iter()
            .map(|(k, v)| (k.as_str(), *v))
            .collect(),
    );

    let similarity = combined_tags.similarity(&recipe_tags);
    if similarity < 0.4 {
        return Err(CraftError::TagMismatch {
            similarity,
            required: 0.4,
        });
    }

    // Check rarity floor
    let avg_rarity = average_rarity(materials);

    // Calculate quality from similarity
    let quality = ((similarity - 0.4) / 0.6).min(1.0); // 0.4->0.0, 1.0->1.0

    // Determine result rarity (upgrade chance based on quality)
    let result_rarity = if quality > 0.8 {
        upgrade_rarity(avg_rarity)
    } else {
        avg_rarity
    };

    // Blend output tags from materials
    let result_tags: Vec<(String, f32)> = combined_tags
        .tags
        .iter()
        .filter(|(_, v)| *v > 0.2)
        .map(|(k, v)| (k.clone(), *v * quality))
        .collect();

    let name = generate_craft_name(&combined_tags, recipe);

    Ok(CraftResult {
        name,
        category: recipe.result_category,
        rarity: result_rarity,
        quality,
        semantic_tags: result_tags,
    })
}

/// Material input for crafting
#[derive(Debug, Clone)]
pub struct CraftMaterial {
    pub name: String,
    pub rarity: ItemRarity,
    pub tags: SemanticTags,
}

/// Crafting errors
#[derive(Debug, Clone)]
pub enum CraftError {
    NotEnoughMaterials { required: u32, provided: u32 },
    TagMismatch { similarity: f32, required: f32 },
    InsufficientShards,
}

fn combine_material_tags(materials: &[CraftMaterial]) -> SemanticTags {
    let mut combined = SemanticTags::new(vec![]);
    for mat in materials {
        combined.blend(&mat.tags, 1.0 / materials.len() as f32);
    }
    combined
}

fn average_rarity(materials: &[CraftMaterial]) -> ItemRarity {
    if materials.is_empty() {
        return ItemRarity::Common;
    }
    let total: u32 = materials.iter().map(|m| rarity_to_num(m.rarity)).sum();
    let avg = total / materials.len() as u32;
    num_to_rarity(avg)
}

fn rarity_to_num(r: ItemRarity) -> u32 {
    match r {
        ItemRarity::Common => 0,
        ItemRarity::Uncommon => 1,
        ItemRarity::Rare => 2,
        ItemRarity::Epic => 3,
        ItemRarity::Legendary => 4,
        ItemRarity::Mythic => 5,
    }
}

fn num_to_rarity(n: u32) -> ItemRarity {
    match n {
        0 => ItemRarity::Common,
        1 => ItemRarity::Uncommon,
        2 => ItemRarity::Rare,
        3 => ItemRarity::Epic,
        4 => ItemRarity::Legendary,
        _ => ItemRarity::Mythic,
    }
}

fn upgrade_rarity(r: ItemRarity) -> ItemRarity {
    match r {
        ItemRarity::Common => ItemRarity::Uncommon,
        ItemRarity::Uncommon => ItemRarity::Rare,
        ItemRarity::Rare => ItemRarity::Epic,
        ItemRarity::Epic => ItemRarity::Legendary,
        ItemRarity::Legendary => ItemRarity::Mythic,
        ItemRarity::Mythic => ItemRarity::Mythic,
    }
}

fn generate_craft_name(tags: &SemanticTags, recipe: &CraftingRecipe) -> String {
    let prefix = if let Some((dominant, _)) = tags.dominant() {
        match dominant {
            "fire" => "Ember",
            "water" => "Tide",
            "corruption" => "Shadow",
            "void" => "Void",
            "wind" => "Storm",
            "earth" => "Stone",
            _ => "Tower",
        }
    } else {
        "Tower"
    };

    let suffix = match recipe.result_category {
        CraftResultCategory::Weapon => "Blade",
        CraftResultCategory::Armor => "Guard",
        CraftResultCategory::Accessory => "Charm",
        CraftResultCategory::Consumable => "Elixir",
        CraftResultCategory::Enhancement => "Sigil",
    };

    format!("{} {}", prefix, suffix)
}

/// Standard recipes
pub fn basic_recipes() -> Vec<CraftingRecipe> {
    vec![
        CraftingRecipe {
            name: "Fire Weapon".into(),
            required_tags: vec![("fire".into(), 0.5)],
            material_count: 3,
            min_rarity: ItemRarity::Common,
            result_category: CraftResultCategory::Weapon,
            shard_cost: 100,
        },
        CraftingRecipe {
            name: "Water Armor".into(),
            required_tags: vec![("water".into(), 0.5)],
            material_count: 4,
            min_rarity: ItemRarity::Uncommon,
            result_category: CraftResultCategory::Armor,
            shard_cost: 150,
        },
        CraftingRecipe {
            name: "Healing Elixir".into(),
            required_tags: vec![("healing".into(), 0.3), ("water".into(), 0.2)],
            material_count: 2,
            min_rarity: ItemRarity::Common,
            result_category: CraftResultCategory::Consumable,
            shard_cost: 50,
        },
        CraftingRecipe {
            name: "Void Enhancement".into(),
            required_tags: vec![("void".into(), 0.6), ("corruption".into(), 0.3)],
            material_count: 5,
            min_rarity: ItemRarity::Rare,
            result_category: CraftResultCategory::Enhancement,
            shard_cost: 500,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fire_materials() -> Vec<CraftMaterial> {
        vec![
            CraftMaterial {
                name: "Ember Shard".into(),
                rarity: ItemRarity::Common,
                tags: SemanticTags::new(vec![("fire", 0.8), ("aggression", 0.3)]),
            },
            CraftMaterial {
                name: "Thermal Core".into(),
                rarity: ItemRarity::Uncommon,
                tags: SemanticTags::new(vec![("fire", 0.9), ("energy", 0.5)]),
            },
            CraftMaterial {
                name: "Flame Essence".into(),
                rarity: ItemRarity::Rare,
                tags: SemanticTags::new(vec![("fire", 0.7), ("corruption", 0.2)]),
            },
        ]
    }

    #[test]
    fn test_basic_craft_success() {
        let materials = fire_materials();
        let recipes = basic_recipes();
        let fire_recipe = &recipes[0]; // Fire Weapon

        let result = attempt_craft(&materials, fire_recipe);
        assert!(result.is_ok(), "Fire materials should craft fire weapon");

        let item = result.unwrap();
        assert_eq!(item.category, CraftResultCategory::Weapon);
        assert!(
            item.name.contains("Ember"),
            "Name should reflect fire theme"
        );
    }

    #[test]
    fn test_craft_not_enough_materials() {
        let materials = vec![fire_materials()[0].clone()];
        let recipes = basic_recipes();
        let fire_recipe = &recipes[0]; // requires 3

        let result = attempt_craft(&materials, fire_recipe);
        assert!(matches!(result, Err(CraftError::NotEnoughMaterials { .. })));
    }

    #[test]
    fn test_craft_tag_mismatch() {
        let water_materials = vec![
            CraftMaterial {
                name: "Tide Shard".into(),
                rarity: ItemRarity::Common,
                tags: SemanticTags::new(vec![("water", 0.9)]),
            },
            CraftMaterial {
                name: "Tide Core".into(),
                rarity: ItemRarity::Common,
                tags: SemanticTags::new(vec![("water", 0.8)]),
            },
            CraftMaterial {
                name: "Tide Essence".into(),
                rarity: ItemRarity::Common,
                tags: SemanticTags::new(vec![("water", 0.7)]),
            },
        ];
        let recipes = basic_recipes();
        let fire_recipe = &recipes[0]; // Fire Weapon

        let result = attempt_craft(&water_materials, fire_recipe);
        assert!(matches!(result, Err(CraftError::TagMismatch { .. })));
    }

    #[test]
    fn test_rarity_upgrade() {
        assert_eq!(upgrade_rarity(ItemRarity::Common), ItemRarity::Uncommon);
        assert_eq!(upgrade_rarity(ItemRarity::Mythic), ItemRarity::Mythic);
    }

    #[test]
    fn test_basic_recipes_exist() {
        let recipes = basic_recipes();
        assert!(recipes.len() >= 4, "Should have at least 4 basic recipes");
    }
}
