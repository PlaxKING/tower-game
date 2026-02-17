//! Repository traits - abstraction layer for data access
//!
//! All game systems interact with data through these traits,
//! making it easy to swap storage backends (LMDB → FoundationDB, etc.)

use async_trait::async_trait;
use std::error::Error;

/// Generic result type for repository operations
pub type RepoResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

// ============================================================================
// Template Repositories (Read-only, LMDB-backed)
// ============================================================================

/// Repository for monster templates (static data)
#[async_trait]
pub trait MonsterTemplateRepo: Send + Sync {
    async fn get(&self, id: &str) -> RepoResult<Option<crate::proto::tower::entities::MonsterTemplate>>;
    async fn get_by_tier(&self, tier: u32) -> RepoResult<Vec<crate::proto::tower::entities::MonsterTemplate>>;
    async fn get_all(&self) -> RepoResult<Vec<crate::proto::tower::entities::MonsterTemplate>>;
    async fn count(&self) -> RepoResult<usize>;
}

/// Repository for item templates (static data)
#[async_trait]
pub trait ItemTemplateRepo: Send + Sync {
    async fn get(&self, id: &str) -> RepoResult<Option<crate::proto::tower::entities::ItemTemplate>>;
    async fn get_by_type(&self, item_type: i32) -> RepoResult<Vec<crate::proto::tower::entities::ItemTemplate>>;
    async fn get_by_rarity(&self, rarity: i32) -> RepoResult<Vec<crate::proto::tower::entities::ItemTemplate>>;
    async fn get_set_items(&self, set_id: &str) -> RepoResult<Vec<crate::proto::tower::entities::ItemTemplate>>;
    async fn get_all(&self) -> RepoResult<Vec<crate::proto::tower::entities::ItemTemplate>>;
    async fn count(&self) -> RepoResult<usize>;
}

/// Repository for ability templates
#[async_trait]
pub trait AbilityTemplateRepo: Send + Sync {
    async fn get(&self, id: &str) -> RepoResult<Option<crate::proto::tower::entities::AbilityTemplate>>;
    async fn get_by_domain(&self, domain: &str) -> RepoResult<Vec<crate::proto::tower::entities::AbilityTemplate>>;
    async fn get_all(&self) -> RepoResult<Vec<crate::proto::tower::entities::AbilityTemplate>>;
    async fn count(&self) -> RepoResult<usize>;
}

/// Repository for crafting recipes
#[async_trait]
pub trait RecipeRepo: Send + Sync {
    async fn get(&self, id: &str) -> RepoResult<Option<crate::proto::tower::economy::CraftingRecipe>>;
    async fn get_by_profession(&self, profession: &str) -> RepoResult<Vec<crate::proto::tower::economy::CraftingRecipe>>;
    async fn get_all(&self) -> RepoResult<Vec<crate::proto::tower::economy::CraftingRecipe>>;
    async fn count(&self) -> RepoResult<usize>;
}

/// Repository for quest templates
#[async_trait]
pub trait QuestTemplateRepo: Send + Sync {
    async fn get(&self, id: &str) -> RepoResult<Option<crate::proto::tower::quests::QuestTemplate>>;
    async fn get_by_type(&self, quest_type: i32) -> RepoResult<Vec<crate::proto::tower::quests::QuestTemplate>>;
    async fn get_available_for_floor(&self, floor_id: u32) -> RepoResult<Vec<crate::proto::tower::quests::QuestTemplate>>;
    async fn get_all(&self) -> RepoResult<Vec<crate::proto::tower::quests::QuestTemplate>>;
    async fn count(&self) -> RepoResult<usize>;
}

/// Repository for faction templates
#[async_trait]
pub trait FactionTemplateRepo: Send + Sync {
    async fn get(&self, id: &str) -> RepoResult<Option<crate::proto::tower::quests::FactionTemplate>>;
    async fn get_all(&self) -> RepoResult<Vec<crate::proto::tower::quests::FactionTemplate>>;
    async fn count(&self) -> RepoResult<usize>;
}

/// Repository for loot tables
#[async_trait]
pub trait LootTableRepo: Send + Sync {
    async fn get(&self, id: &str) -> RepoResult<Option<crate::proto::tower::entities::LootTable>>;
    async fn get_all(&self) -> RepoResult<Vec<crate::proto::tower::entities::LootTable>>;
    async fn count(&self) -> RepoResult<usize>;
}

/// Repository for item sets
#[async_trait]
pub trait ItemSetRepo: Send + Sync {
    async fn get(&self, id: &str) -> RepoResult<Option<crate::proto::tower::entities::ItemSet>>;
    async fn get_all(&self) -> RepoResult<Vec<crate::proto::tower::entities::ItemSet>>;
    async fn count(&self) -> RepoResult<usize>;
}

/// Repository for gem templates
#[async_trait]
pub trait GemTemplateRepo: Send + Sync {
    async fn get(&self, id: &str) -> RepoResult<Option<crate::proto::tower::entities::GemTemplate>>;
    async fn get_by_socket_type(&self, socket_type: i32) -> RepoResult<Vec<crate::proto::tower::entities::GemTemplate>>;
    async fn get_all(&self) -> RepoResult<Vec<crate::proto::tower::entities::GemTemplate>>;
    async fn count(&self) -> RepoResult<usize>;
}

// ============================================================================
// Player Data Repositories (Read-write, PostgreSQL-backed)
// ============================================================================

/// Repository for player profiles
#[async_trait]
pub trait PlayerRepo: Send + Sync {
    async fn get(&self, id: u64) -> RepoResult<Option<crate::proto::tower::entities::PlayerProfile>>;
    async fn get_by_username(&self, username: &str) -> RepoResult<Option<crate::proto::tower::entities::PlayerProfile>>;
    async fn create(&self, player: &crate::proto::tower::entities::PlayerProfile) -> RepoResult<u64>;
    async fn update(&self, player: &crate::proto::tower::entities::PlayerProfile) -> RepoResult<()>;
    async fn delete(&self, id: u64) -> RepoResult<()>;
    async fn update_position(&self, id: u64, floor_id: u32, x: f32, y: f32, z: f32) -> RepoResult<()>;
    async fn update_health(&self, id: u64, health: f32) -> RepoResult<()>;
    async fn count(&self) -> RepoResult<usize>;
}

/// Repository for mastery progress
#[async_trait]
pub trait MasteryRepo: Send + Sync {
    async fn get(&self, player_id: u64, domain: &str) -> RepoResult<Option<crate::proto::tower::entities::MasteryProgress>>;
    async fn get_all_for_player(&self, player_id: u64) -> RepoResult<Vec<crate::proto::tower::entities::MasteryProgress>>;
    async fn add_experience(&self, player_id: u64, domain: &str, exp: u64) -> RepoResult<crate::proto::tower::entities::MasteryProgress>;
    async fn set_specialization(&self, player_id: u64, domain: &str, spec: &str) -> RepoResult<()>;
}

/// Repository for inventory
#[async_trait]
pub trait InventoryRepo: Send + Sync {
    async fn get_bag(&self, player_id: u64) -> RepoResult<Vec<crate::proto::tower::entities::InventorySlot>>;
    async fn get_equipment(&self, player_id: u64) -> RepoResult<Vec<crate::proto::tower::entities::InventorySlot>>;
    async fn get_bank(&self, player_id: u64) -> RepoResult<Vec<crate::proto::tower::entities::InventorySlot>>;
    async fn add_item(&self, player_id: u64, item_template_id: &str, quantity: u32) -> RepoResult<u64>;
    async fn remove_item(&self, slot_id: u64, quantity: u32) -> RepoResult<()>;
    async fn equip_item(&self, player_id: u64, slot_id: u64, equipment_slot: i32) -> RepoResult<()>;
    async fn unequip_item(&self, player_id: u64, equipment_slot: i32) -> RepoResult<()>;
    async fn move_item(&self, slot_id: u64, new_slot_index: u32) -> RepoResult<()>;
}

/// Repository for wallets
#[async_trait]
pub trait WalletRepo: Send + Sync {
    async fn get(&self, player_id: u64) -> RepoResult<crate::proto::tower::entities::Wallet>;
    async fn add_gold(&self, player_id: u64, amount: u64) -> RepoResult<u64>;
    async fn remove_gold(&self, player_id: u64, amount: u64) -> RepoResult<u64>;
    async fn transfer_gold(&self, from: u64, to: u64, amount: u64) -> RepoResult<()>;
}

/// Repository for guilds
#[async_trait]
pub trait GuildRepo: Send + Sync {
    async fn get(&self, id: u64) -> RepoResult<Option<crate::proto::tower::social::Guild>>;
    async fn get_by_name(&self, name: &str) -> RepoResult<Option<crate::proto::tower::social::Guild>>;
    async fn create(&self, guild: &crate::proto::tower::social::Guild) -> RepoResult<u64>;
    async fn add_member(&self, guild_id: u64, player_id: u64, rank: i32) -> RepoResult<()>;
    async fn remove_member(&self, guild_id: u64, player_id: u64) -> RepoResult<()>;
    async fn get_members(&self, guild_id: u64) -> RepoResult<Vec<crate::proto::tower::social::GuildMember>>;
    async fn update_rank(&self, guild_id: u64, player_id: u64, rank: i32) -> RepoResult<()>;
}

/// Repository for quest progress
#[async_trait]
pub trait QuestProgressRepo: Send + Sync {
    async fn get(&self, player_id: u64, quest_id: &str) -> RepoResult<Option<crate::proto::tower::quests::PlayerQuest>>;
    async fn get_active(&self, player_id: u64) -> RepoResult<Vec<crate::proto::tower::quests::PlayerQuest>>;
    async fn get_completed(&self, player_id: u64) -> RepoResult<Vec<crate::proto::tower::quests::PlayerQuest>>;
    async fn start_quest(&self, player_id: u64, quest_id: &str) -> RepoResult<()>;
    async fn update_objective(&self, player_id: u64, quest_id: &str, objective_idx: u8, count: u32) -> RepoResult<()>;
    async fn complete_quest(&self, player_id: u64, quest_id: &str) -> RepoResult<()>;
}

/// Repository for auctions
#[async_trait]
pub trait AuctionRepo: Send + Sync {
    async fn get(&self, id: u64) -> RepoResult<Option<crate::proto::tower::economy::AuctionListing>>;
    async fn get_active(&self) -> RepoResult<Vec<crate::proto::tower::economy::AuctionListing>>;
    async fn get_by_seller(&self, seller_id: u64) -> RepoResult<Vec<crate::proto::tower::economy::AuctionListing>>;
    async fn create(&self, listing: &crate::proto::tower::economy::AuctionListing) -> RepoResult<u64>;
    async fn place_bid(&self, auction_id: u64, bidder_id: u64, amount: u64) -> RepoResult<()>;
    async fn buyout(&self, auction_id: u64, buyer_id: u64) -> RepoResult<()>;
    async fn cancel(&self, auction_id: u64) -> RepoResult<()>;
    async fn expire_old(&self) -> RepoResult<u32>;
}

/// Repository for faction reputation
#[async_trait]
pub trait ReputationRepo: Send + Sync {
    async fn get(&self, player_id: u64, faction_id: &str) -> RepoResult<Option<crate::proto::tower::quests::PlayerReputation>>;
    async fn get_all_for_player(&self, player_id: u64) -> RepoResult<Vec<crate::proto::tower::quests::PlayerReputation>>;
    async fn add_reputation(&self, player_id: u64, faction_id: &str, amount: i32) -> RepoResult<crate::proto::tower::quests::PlayerReputation>;
}

// ============================================================================
// Unified Storage Manager
// ============================================================================

/// Central storage manager that holds all repositories
pub struct StorageManager {
    // Template stores (LMDB)
    pub monsters: Box<dyn MonsterTemplateRepo>,
    pub items: Box<dyn ItemTemplateRepo>,
    pub abilities: Box<dyn AbilityTemplateRepo>,
    pub recipes: Box<dyn RecipeRepo>,
    pub quests: Box<dyn QuestTemplateRepo>,
    pub factions: Box<dyn FactionTemplateRepo>,
    pub loot_tables: Box<dyn LootTableRepo>,
    pub item_sets: Box<dyn ItemSetRepo>,
    pub gems: Box<dyn GemTemplateRepo>,

    // Player data stores (PostgreSQL)
    pub players: Box<dyn PlayerRepo>,
    pub mastery: Box<dyn MasteryRepo>,
    pub inventory: Box<dyn InventoryRepo>,
    pub wallets: Box<dyn WalletRepo>,
    pub guilds: Box<dyn GuildRepo>,
    pub quest_progress: Box<dyn QuestProgressRepo>,
    pub auctions: Box<dyn AuctionRepo>,
    pub reputation: Box<dyn ReputationRepo>,
}
