//! LMDB Repository Adapters
//!
//! Implements the Repository traits from `repository.rs` using LmdbTemplateStore
//! as the backend. These adapters wrap synchronous LMDB calls in async interfaces.

use async_trait::async_trait;
use std::sync::Arc;
use super::lmdb_templates::LmdbTemplateStore;
use super::repository::*;

use crate::proto::tower::entities;
use crate::proto::tower::economy;
use crate::proto::tower::quests;

/// Adapter wrapping LmdbTemplateStore for MonsterTemplateRepo
pub struct LmdbMonsterRepo {
    store: Arc<LmdbTemplateStore>,
}

impl LmdbMonsterRepo {
    pub fn new(store: Arc<LmdbTemplateStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl MonsterTemplateRepo for LmdbMonsterRepo {
    async fn get(&self, id: &str) -> RepoResult<Option<entities::MonsterTemplate>> {
        Ok(self.store.get_monster(id)?)
    }

    async fn get_by_tier(&self, tier: u32) -> RepoResult<Vec<entities::MonsterTemplate>> {
        let all: Vec<entities::MonsterTemplate> = self.store.get_all(self.store.monsters)?;
        Ok(all.into_iter().filter(|m| m.tier == tier).collect())
    }

    async fn get_all(&self) -> RepoResult<Vec<entities::MonsterTemplate>> {
        Ok(self.store.get_all(self.store.monsters)?)
    }

    async fn count(&self) -> RepoResult<usize> {
        Ok(self.store.count(self.store.monsters)?)
    }
}

/// Adapter for ItemTemplateRepo
pub struct LmdbItemRepo {
    store: Arc<LmdbTemplateStore>,
}

impl LmdbItemRepo {
    pub fn new(store: Arc<LmdbTemplateStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl ItemTemplateRepo for LmdbItemRepo {
    async fn get(&self, id: &str) -> RepoResult<Option<entities::ItemTemplate>> {
        Ok(self.store.get_item(id)?)
    }

    async fn get_by_type(&self, item_type: i32) -> RepoResult<Vec<entities::ItemTemplate>> {
        let all: Vec<entities::ItemTemplate> = self.store.get_all(self.store.items)?;
        Ok(all.into_iter().filter(|i| i.item_type == item_type).collect())
    }

    async fn get_by_rarity(&self, rarity: i32) -> RepoResult<Vec<entities::ItemTemplate>> {
        let all: Vec<entities::ItemTemplate> = self.store.get_all(self.store.items)?;
        Ok(all.into_iter().filter(|i| i.rarity == rarity).collect())
    }

    async fn get_set_items(&self, set_id: &str) -> RepoResult<Vec<entities::ItemTemplate>> {
        let all: Vec<entities::ItemTemplate> = self.store.get_all(self.store.items)?;
        Ok(all.into_iter().filter(|i| i.set_id == set_id).collect())
    }

    async fn get_all(&self) -> RepoResult<Vec<entities::ItemTemplate>> {
        Ok(self.store.get_all(self.store.items)?)
    }

    async fn count(&self) -> RepoResult<usize> {
        Ok(self.store.count(self.store.items)?)
    }
}

/// Adapter for AbilityTemplateRepo
pub struct LmdbAbilityRepo {
    store: Arc<LmdbTemplateStore>,
}

impl LmdbAbilityRepo {
    pub fn new(store: Arc<LmdbTemplateStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl AbilityTemplateRepo for LmdbAbilityRepo {
    async fn get(&self, id: &str) -> RepoResult<Option<entities::AbilityTemplate>> {
        Ok(self.store.get_ability(id)?)
    }

    async fn get_by_domain(&self, domain: &str) -> RepoResult<Vec<entities::AbilityTemplate>> {
        let all: Vec<entities::AbilityTemplate> = self.store.get_all(self.store.abilities)?;
        Ok(all.into_iter().filter(|a| a.required_mastery_domain == domain).collect())
    }

    async fn get_all(&self) -> RepoResult<Vec<entities::AbilityTemplate>> {
        Ok(self.store.get_all(self.store.abilities)?)
    }

    async fn count(&self) -> RepoResult<usize> {
        Ok(self.store.count(self.store.abilities)?)
    }
}

/// Adapter for RecipeRepo
pub struct LmdbRecipeRepo {
    store: Arc<LmdbTemplateStore>,
}

impl LmdbRecipeRepo {
    pub fn new(store: Arc<LmdbTemplateStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl RecipeRepo for LmdbRecipeRepo {
    async fn get(&self, id: &str) -> RepoResult<Option<economy::CraftingRecipe>> {
        Ok(self.store.get_recipe(id)?)
    }

    async fn get_by_profession(&self, profession: &str) -> RepoResult<Vec<economy::CraftingRecipe>> {
        let all: Vec<economy::CraftingRecipe> = self.store.get_all(self.store.recipes)?;
        Ok(all.into_iter().filter(|r| r.profession == profession).collect())
    }

    async fn get_all(&self) -> RepoResult<Vec<economy::CraftingRecipe>> {
        Ok(self.store.get_all(self.store.recipes)?)
    }

    async fn count(&self) -> RepoResult<usize> {
        Ok(self.store.count(self.store.recipes)?)
    }
}

/// Adapter for QuestTemplateRepo
pub struct LmdbQuestRepo {
    store: Arc<LmdbTemplateStore>,
}

impl LmdbQuestRepo {
    pub fn new(store: Arc<LmdbTemplateStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl QuestTemplateRepo for LmdbQuestRepo {
    async fn get(&self, id: &str) -> RepoResult<Option<quests::QuestTemplate>> {
        Ok(self.store.get_quest(id)?)
    }

    async fn get_by_type(&self, quest_type: i32) -> RepoResult<Vec<quests::QuestTemplate>> {
        let all: Vec<quests::QuestTemplate> = self.store.get_all(self.store.quests)?;
        Ok(all.into_iter().filter(|q| q.quest_type == quest_type).collect())
    }

    async fn get_available_for_floor(&self, floor_id: u32) -> RepoResult<Vec<quests::QuestTemplate>> {
        let all: Vec<quests::QuestTemplate> = self.store.get_all(self.store.quests)?;
        Ok(all.into_iter()
            .filter(|q| floor_id >= q.required_floor_min)
            .collect())
    }

    async fn get_all(&self) -> RepoResult<Vec<quests::QuestTemplate>> {
        Ok(self.store.get_all(self.store.quests)?)
    }

    async fn count(&self) -> RepoResult<usize> {
        Ok(self.store.count(self.store.quests)?)
    }
}

/// Adapter for FactionTemplateRepo
pub struct LmdbFactionRepo {
    store: Arc<LmdbTemplateStore>,
}

impl LmdbFactionRepo {
    pub fn new(store: Arc<LmdbTemplateStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl FactionTemplateRepo for LmdbFactionRepo {
    async fn get(&self, id: &str) -> RepoResult<Option<quests::FactionTemplate>> {
        let result: Option<quests::FactionTemplate> = self.store.get(self.store.factions, id)?;
        Ok(result)
    }

    async fn get_all(&self) -> RepoResult<Vec<quests::FactionTemplate>> {
        Ok(self.store.get_all(self.store.factions)?)
    }

    async fn count(&self) -> RepoResult<usize> {
        Ok(self.store.count(self.store.factions)?)
    }
}

/// Adapter for LootTableRepo
pub struct LmdbLootTableRepo {
    store: Arc<LmdbTemplateStore>,
}

impl LmdbLootTableRepo {
    pub fn new(store: Arc<LmdbTemplateStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl LootTableRepo for LmdbLootTableRepo {
    async fn get(&self, id: &str) -> RepoResult<Option<entities::LootTable>> {
        Ok(self.store.get_loot_table(id)?)
    }

    async fn get_all(&self) -> RepoResult<Vec<entities::LootTable>> {
        Ok(self.store.get_all(self.store.loot_tables)?)
    }

    async fn count(&self) -> RepoResult<usize> {
        Ok(self.store.count(self.store.loot_tables)?)
    }
}

/// Adapter for ItemSetRepo
pub struct LmdbItemSetRepo {
    store: Arc<LmdbTemplateStore>,
}

impl LmdbItemSetRepo {
    pub fn new(store: Arc<LmdbTemplateStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl ItemSetRepo for LmdbItemSetRepo {
    async fn get(&self, id: &str) -> RepoResult<Option<entities::ItemSet>> {
        let result: Option<entities::ItemSet> = self.store.get(self.store.item_sets, id)?;
        Ok(result)
    }

    async fn get_all(&self) -> RepoResult<Vec<entities::ItemSet>> {
        Ok(self.store.get_all(self.store.item_sets)?)
    }

    async fn count(&self) -> RepoResult<usize> {
        Ok(self.store.count(self.store.item_sets)?)
    }
}

/// Adapter for GemTemplateRepo
pub struct LmdbGemRepo {
    store: Arc<LmdbTemplateStore>,
}

impl LmdbGemRepo {
    pub fn new(store: Arc<LmdbTemplateStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl GemTemplateRepo for LmdbGemRepo {
    async fn get(&self, id: &str) -> RepoResult<Option<entities::GemTemplate>> {
        let result: Option<entities::GemTemplate> = self.store.get(self.store.gems, id)?;
        Ok(result)
    }

    async fn get_by_socket_type(&self, socket_type: i32) -> RepoResult<Vec<entities::GemTemplate>> {
        let all: Vec<entities::GemTemplate> = self.store.get_all(self.store.gems)?;
        Ok(all.into_iter().filter(|g| g.socket_type == socket_type).collect())
    }

    async fn get_all(&self) -> RepoResult<Vec<entities::GemTemplate>> {
        Ok(self.store.get_all(self.store.gems)?)
    }

    async fn count(&self) -> RepoResult<usize> {
        Ok(self.store.count(self.store.gems)?)
    }
}
