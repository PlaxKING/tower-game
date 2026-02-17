//! PostgreSQL Repository Adapters
//!
//! Implements the Repository traits from `repository.rs` using PostgresStore
//! as the backend. Converts between SQL row types and protobuf types.

use async_trait::async_trait;
use std::sync::Arc;

use super::postgres::PostgresStore;
use super::repository::*;

use crate::proto::tower::economy;
use crate::proto::tower::entities;
use crate::proto::tower::game;
use crate::proto::tower::quests;
use crate::proto::tower::social;

// ============================================================================
// Type Conversion Helpers
// ============================================================================

fn row_to_player_profile(row: &super::postgres::PlayerRow) -> entities::PlayerProfile {
    entities::PlayerProfile {
        id: row.id as u64,
        username: row.username.clone(),
        created_at: row.created_at.map(|t| t.timestamp()).unwrap_or(0),
        last_login: row.last_login.map(|t| t.timestamp()).unwrap_or(0),
        playtime_seconds: row.playtime_seconds as u64,
        base_str: row.base_str as u32,
        base_dex: row.base_dex as u32,
        base_int: row.base_int as u32,
        base_vit: row.base_vit as u32,
        base_luk: row.base_luk as u32,
        health: row.health,
        max_health: row.max_health,
        kinetic_energy: row.kinetic_energy,
        thermal_energy: row.thermal_energy,
        semantic_energy: row.semantic_energy,
        floor_id: row.floor_id as u32,
        position: Some(game::Vec3 {
            x: row.pos_x,
            y: row.pos_y,
            z: row.pos_z,
        }),
        rotation: Some(game::Rotation {
            pitch: row.rot_pitch,
            yaw: row.rot_yaw,
            roll: 0.0,
        }),
        is_alive: row.is_alive,
        respawn_floor: row.respawn_floor as u32,
        ..Default::default()
    }
}

fn row_to_mastery(row: &super::postgres::MasteryRow) -> entities::MasteryProgress {
    entities::MasteryProgress {
        domain: row.domain.clone(),
        experience: row.experience as u64,
        tier: row.tier as i32,
        specialization: row.specialization.clone().unwrap_or_default(),
    }
}

fn row_to_inventory_slot(row: &super::postgres::InventoryRow) -> entities::InventorySlot {
    entities::InventorySlot {
        slot_id: row.id as u64,
        item_template_id: row.item_template_id.clone(),
        quantity: row.quantity as u32,
        slot_type: row.slot_type as u32,
        slot_index: row.slot_index as u32,
        instance: if row.instance_id.is_some() {
            Some(entities::ItemInstance {
                instance_id: row.instance_id.unwrap_or(0) as u64,
                durability: row.durability.unwrap_or(100.0),
                enhancement_level: row.enhancement.unwrap_or(0) as u32,
                ..Default::default()
            })
        } else {
            None
        },
    }
}

fn row_to_wallet(row: &super::postgres::WalletRow) -> entities::Wallet {
    let mut event_tokens = std::collections::HashMap::new();
    if let Some(ref tokens) = row.event_tokens {
        if let Some(map) = tokens.as_object() {
            for (k, v) in map {
                if let Some(n) = v.as_u64() {
                    event_tokens.insert(k.clone(), n as u32);
                }
            }
        }
    }
    entities::Wallet {
        gold: row.gold as u64,
        premium_currency: row.premium_currency as u32,
        honor_points: row.honor_points as u32,
        event_tokens,
    }
}

fn row_to_guild(row: &super::postgres::GuildRow) -> social::Guild {
    social::Guild {
        id: row.id as u64,
        name: row.name.clone(),
        tag: row.tag.clone(),
        leader_id: row.leader_id as u64,
        created_at: row.created_at.map(|t| t.timestamp()).unwrap_or(0),
        level: row.level as u32,
        experience: row.experience as u64,
        max_members: row.max_members as u32,
        description: row.description.clone().unwrap_or_default(),
        motd: row.motd.clone().unwrap_or_default(),
        is_recruiting: row.is_recruiting,
        bank_gold: row.bank_gold as u64,
        ..Default::default()
    }
}

fn row_to_guild_member(row: &super::postgres::GuildMemberRow) -> social::GuildMember {
    social::GuildMember {
        guild_id: row.guild_id as u64,
        player_id: row.player_id as u64,
        rank: row.rank as i32,
        joined_at: row.joined_at.map(|t| t.timestamp()).unwrap_or(0),
        contribution_points: row.contribution as u64,
        note: row.note.clone().unwrap_or_default(),
        ..Default::default()
    }
}

fn row_to_player_quest(row: &super::postgres::QuestProgressRow) -> quests::PlayerQuest {
    let objectives = if let Some(ref obj_json) = row.objectives {
        if let Some(arr) = obj_json.as_array() {
            arr.iter()
                .map(|o| quests::ObjectiveProgress {
                    current_count: o.get("current").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    is_complete: o.get("complete").and_then(|v| v.as_bool()).unwrap_or(false),
                    ..Default::default()
                })
                .collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    quests::PlayerQuest {
        player_id: row.player_id as u64,
        quest_id: row.quest_id.clone(),
        status: row.status as i32,
        objectives,
        started_at: row.started_at.map(|t| t.timestamp()).unwrap_or(0),
        completed_at: row.completed_at.map(|t| t.timestamp()).unwrap_or(0),
        times_completed: row.times_completed as u32,
    }
}

fn row_to_auction(row: &super::postgres::AuctionRow) -> economy::AuctionListing {
    economy::AuctionListing {
        id: row.id as u64,
        seller_id: row.seller_id as u64,
        item_template_id: row.item_template_id.clone(),
        quantity: row.quantity as u32,
        buyout_price: row.buyout_price as u64,
        starting_bid: row.starting_bid as u64,
        current_bid: row.current_bid.unwrap_or(0) as u64,
        highest_bidder_id: row.highest_bidder.unwrap_or(0) as u64,
        created_at: row.created_at.map(|t| t.timestamp()).unwrap_or(0),
        expires_at: row.expires_at.timestamp(),
        status: row.status as i32,
        tax_rate_percent: row.tax_rate as u32,
        ..Default::default()
    }
}

fn row_to_reputation(row: &super::postgres::ReputationRow) -> quests::PlayerReputation {
    quests::PlayerReputation {
        player_id: row.player_id as u64,
        faction_id: row.faction_id.clone(),
        reputation: row.reputation,
        standing: row.standing as i32,
    }
}

// ============================================================================
// PlayerRepo Adapter
// ============================================================================

pub struct PgPlayerRepo {
    store: Arc<PostgresStore>,
}

impl PgPlayerRepo {
    pub fn new(store: Arc<PostgresStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl PlayerRepo for PgPlayerRepo {
    async fn get(&self, id: u64) -> RepoResult<Option<entities::PlayerProfile>> {
        let row = self.store.get_player(id as i64).await?;
        Ok(row.as_ref().map(row_to_player_profile))
    }

    async fn get_by_username(&self, username: &str) -> RepoResult<Option<entities::PlayerProfile>> {
        let row = self.store.get_player_by_username(username).await?;
        Ok(row.as_ref().map(row_to_player_profile))
    }

    async fn create(&self, player: &entities::PlayerProfile) -> RepoResult<u64> {
        let id = self
            .store
            .create_player(
                &player.username,
                "", // password_hash handled by auth layer
                player.base_str as i16,
                player.base_dex as i16,
                player.base_int as i16,
                player.base_vit as i16,
                player.base_luk as i16,
            )
            .await?;
        Ok(id as u64)
    }

    async fn update(&self, player: &entities::PlayerProfile) -> RepoResult<()> {
        let pos = player.position.as_ref();
        self.store
            .update_player_position(
                player.id as i64,
                player.floor_id as i32,
                pos.map(|p| p.x).unwrap_or(0.0),
                pos.map(|p| p.y).unwrap_or(0.0),
                pos.map(|p| p.z).unwrap_or(0.0),
            )
            .await?;
        self.store
            .update_player_health(player.id as i64, player.health, player.is_alive)
            .await?;
        Ok(())
    }

    async fn delete(&self, id: u64) -> RepoResult<()> {
        self.store.delete_player(id as i64).await?;
        Ok(())
    }

    async fn update_position(
        &self,
        id: u64,
        floor_id: u32,
        x: f32,
        y: f32,
        z: f32,
    ) -> RepoResult<()> {
        self.store
            .update_player_position(id as i64, floor_id as i32, x, y, z)
            .await?;
        Ok(())
    }

    async fn update_health(&self, id: u64, health: f32) -> RepoResult<()> {
        self.store
            .update_player_health(id as i64, health, health > 0.0)
            .await?;
        Ok(())
    }

    async fn count(&self) -> RepoResult<usize> {
        let c = self.store.count_players().await?;
        Ok(c as usize)
    }
}

// ============================================================================
// MasteryRepo Adapter
// ============================================================================

pub struct PgMasteryRepo {
    store: Arc<PostgresStore>,
}

impl PgMasteryRepo {
    pub fn new(store: Arc<PostgresStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl MasteryRepo for PgMasteryRepo {
    async fn get(
        &self,
        player_id: u64,
        domain: &str,
    ) -> RepoResult<Option<entities::MasteryProgress>> {
        let row = self.store.get_mastery(player_id as i64, domain).await?;
        Ok(row.as_ref().map(row_to_mastery))
    }

    async fn get_all_for_player(
        &self,
        player_id: u64,
    ) -> RepoResult<Vec<entities::MasteryProgress>> {
        let rows = self.store.get_all_mastery(player_id as i64).await?;
        Ok(rows.iter().map(row_to_mastery).collect())
    }

    async fn add_experience(
        &self,
        player_id: u64,
        domain: &str,
        exp: u64,
    ) -> RepoResult<entities::MasteryProgress> {
        let row = self
            .store
            .add_mastery_experience(player_id as i64, domain, exp as i64)
            .await?;
        Ok(row_to_mastery(&row))
    }

    async fn set_specialization(&self, player_id: u64, domain: &str, spec: &str) -> RepoResult<()> {
        self.store
            .set_mastery_specialization(player_id as i64, domain, spec)
            .await?;
        Ok(())
    }
}

// ============================================================================
// InventoryRepo Adapter
// ============================================================================

pub struct PgInventoryRepo {
    store: Arc<PostgresStore>,
}

impl PgInventoryRepo {
    pub fn new(store: Arc<PostgresStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl InventoryRepo for PgInventoryRepo {
    async fn get_bag(&self, player_id: u64) -> RepoResult<Vec<entities::InventorySlot>> {
        let rows = self.store.get_bag(player_id as i64).await?;
        Ok(rows.iter().map(row_to_inventory_slot).collect())
    }

    async fn get_equipment(&self, player_id: u64) -> RepoResult<Vec<entities::InventorySlot>> {
        let rows = self.store.get_equipment(player_id as i64).await?;
        Ok(rows.iter().map(row_to_inventory_slot).collect())
    }

    async fn get_bank(&self, player_id: u64) -> RepoResult<Vec<entities::InventorySlot>> {
        // Bank uses slot_type = 2
        let rows = sqlx::query_as::<_, super::postgres::InventoryRow>(
            "SELECT id, player_id, item_template_id, quantity, slot_type, slot_index,
                    instance_id, durability, enhancement, sockets, rolled_effects
             FROM inventory WHERE player_id = $1 AND slot_type = 2
             ORDER BY slot_index",
        )
        .bind(player_id as i64)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows.iter().map(row_to_inventory_slot).collect())
    }

    async fn add_item(
        &self,
        player_id: u64,
        item_template_id: &str,
        quantity: u32,
    ) -> RepoResult<u64> {
        let id = self
            .store
            .add_item(player_id as i64, item_template_id, quantity as i32, 0)
            .await?;
        Ok(id as u64)
    }

    async fn remove_item(&self, slot_id: u64, quantity: u32) -> RepoResult<()> {
        self.store
            .remove_item(slot_id as i64, quantity as i32)
            .await?;
        Ok(())
    }

    async fn equip_item(
        &self,
        player_id: u64,
        slot_id: u64,
        equipment_slot: i32,
    ) -> RepoResult<()> {
        self.store
            .equip_item(player_id as i64, slot_id as i64, equipment_slot as i16)
            .await?;
        Ok(())
    }

    async fn unequip_item(&self, player_id: u64, equipment_slot: i32) -> RepoResult<()> {
        // Move equipped item back to bag
        sqlx::query(
            "UPDATE inventory SET slot_type = 0,
                    slot_index = (SELECT COALESCE(MAX(slot_index) + 1, 0) FROM inventory WHERE player_id = $1 AND slot_type = 0)
             WHERE player_id = $1 AND slot_type = 1 AND slot_index = $2"
        )
        .bind(player_id as i64)
        .bind(equipment_slot as i16)
        .execute(self.store.pool())
        .await?;
        Ok(())
    }

    async fn move_item(&self, slot_id: u64, new_slot_index: u32) -> RepoResult<()> {
        sqlx::query("UPDATE inventory SET slot_index = $2 WHERE id = $1")
            .bind(slot_id as i64)
            .bind(new_slot_index as i16)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }
}

// ============================================================================
// WalletRepo Adapter
// ============================================================================

pub struct PgWalletRepo {
    store: Arc<PostgresStore>,
}

impl PgWalletRepo {
    pub fn new(store: Arc<PostgresStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl WalletRepo for PgWalletRepo {
    async fn get(&self, player_id: u64) -> RepoResult<entities::Wallet> {
        let row = self.store.get_wallet(player_id as i64).await?;
        Ok(row_to_wallet(&row))
    }

    async fn add_gold(&self, player_id: u64, amount: u64) -> RepoResult<u64> {
        let new_gold = self.store.add_gold(player_id as i64, amount as i64).await?;
        Ok(new_gold as u64)
    }

    async fn remove_gold(&self, player_id: u64, amount: u64) -> RepoResult<u64> {
        let new_gold = self
            .store
            .remove_gold(player_id as i64, amount as i64)
            .await?;
        Ok(new_gold as u64)
    }

    async fn transfer_gold(&self, from: u64, to: u64, amount: u64) -> RepoResult<()> {
        self.store
            .transfer_gold(from as i64, to as i64, amount as i64)
            .await?;
        Ok(())
    }
}

// ============================================================================
// GuildRepo Adapter
// ============================================================================

pub struct PgGuildRepo {
    store: Arc<PostgresStore>,
}

impl PgGuildRepo {
    pub fn new(store: Arc<PostgresStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl GuildRepo for PgGuildRepo {
    async fn get(&self, id: u64) -> RepoResult<Option<social::Guild>> {
        let row = self.store.get_guild(id as i64).await?;
        Ok(row.as_ref().map(row_to_guild))
    }

    async fn get_by_name(&self, name: &str) -> RepoResult<Option<social::Guild>> {
        let row: Option<super::postgres::GuildRow> = sqlx::query_as(
            "SELECT id, name, tag, leader_id, created_at, level, experience,
                    max_members, description, motd, is_recruiting, bank_gold
             FROM guilds WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(self.store.pool())
        .await?;
        Ok(row.as_ref().map(row_to_guild))
    }

    async fn create(&self, guild: &social::Guild) -> RepoResult<u64> {
        let id = self
            .store
            .create_guild(
                &guild.name,
                &guild.tag,
                guild.leader_id as i64,
                &guild.description,
            )
            .await?;
        Ok(id as u64)
    }

    async fn add_member(&self, guild_id: u64, player_id: u64, rank: i32) -> RepoResult<()> {
        self.store
            .add_guild_member(guild_id as i64, player_id as i64, rank as i16)
            .await?;
        Ok(())
    }

    async fn remove_member(&self, guild_id: u64, player_id: u64) -> RepoResult<()> {
        self.store
            .remove_guild_member(guild_id as i64, player_id as i64)
            .await?;
        Ok(())
    }

    async fn get_members(&self, guild_id: u64) -> RepoResult<Vec<social::GuildMember>> {
        let rows = self.store.get_guild_members(guild_id as i64).await?;
        Ok(rows.iter().map(row_to_guild_member).collect())
    }

    async fn update_rank(&self, guild_id: u64, player_id: u64, rank: i32) -> RepoResult<()> {
        sqlx::query("UPDATE guild_members SET rank = $3 WHERE guild_id = $1 AND player_id = $2")
            .bind(guild_id as i64)
            .bind(player_id as i64)
            .bind(rank as i16)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }
}

// ============================================================================
// QuestProgressRepo Adapter
// ============================================================================

pub struct PgQuestProgressRepo {
    store: Arc<PostgresStore>,
}

impl PgQuestProgressRepo {
    pub fn new(store: Arc<PostgresStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl QuestProgressRepo for PgQuestProgressRepo {
    async fn get(&self, player_id: u64, quest_id: &str) -> RepoResult<Option<quests::PlayerQuest>> {
        let row: Option<super::postgres::QuestProgressRow> = sqlx::query_as(
            "SELECT player_id, quest_id, status, objectives, started_at, completed_at, times_completed
             FROM player_quests WHERE player_id = $1 AND quest_id = $2"
        )
        .bind(player_id as i64)
        .bind(quest_id)
        .fetch_optional(self.store.pool())
        .await?;
        Ok(row.as_ref().map(row_to_player_quest))
    }

    async fn get_active(&self, player_id: u64) -> RepoResult<Vec<quests::PlayerQuest>> {
        let rows = self.store.get_active_quests(player_id as i64).await?;
        Ok(rows.iter().map(row_to_player_quest).collect())
    }

    async fn get_completed(&self, player_id: u64) -> RepoResult<Vec<quests::PlayerQuest>> {
        let rows: Vec<super::postgres::QuestProgressRow> = sqlx::query_as(
            "SELECT player_id, quest_id, status, objectives, started_at, completed_at, times_completed
             FROM player_quests WHERE player_id = $1 AND status = 3
             ORDER BY completed_at DESC"
        )
        .bind(player_id as i64)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows.iter().map(row_to_player_quest).collect())
    }

    async fn start_quest(&self, player_id: u64, quest_id: &str) -> RepoResult<()> {
        self.store
            .start_quest(player_id as i64, quest_id, 3)
            .await?;
        Ok(())
    }

    async fn update_objective(
        &self,
        player_id: u64,
        quest_id: &str,
        objective_idx: u8,
        count: u32,
    ) -> RepoResult<()> {
        self.store
            .update_quest_objective(
                player_id as i64,
                quest_id,
                objective_idx as i32,
                count as i32,
            )
            .await?;
        Ok(())
    }

    async fn complete_quest(&self, player_id: u64, quest_id: &str) -> RepoResult<()> {
        self.store
            .complete_quest(player_id as i64, quest_id)
            .await?;
        Ok(())
    }
}

// ============================================================================
// AuctionRepo Adapter
// ============================================================================

pub struct PgAuctionRepo {
    store: Arc<PostgresStore>,
}

impl PgAuctionRepo {
    pub fn new(store: Arc<PostgresStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl AuctionRepo for PgAuctionRepo {
    async fn get(&self, id: u64) -> RepoResult<Option<economy::AuctionListing>> {
        let row: Option<super::postgres::AuctionRow> = sqlx::query_as(
            "SELECT id, seller_id, item_template_id, quantity, buyout_price,
                    starting_bid, current_bid, highest_bidder,
                    created_at, expires_at, status, tax_rate
             FROM auctions WHERE id = $1",
        )
        .bind(id as i64)
        .fetch_optional(self.store.pool())
        .await?;
        Ok(row.as_ref().map(row_to_auction))
    }

    async fn get_active(&self) -> RepoResult<Vec<economy::AuctionListing>> {
        let rows = self.store.get_active_auctions(100, 0).await?;
        Ok(rows.iter().map(row_to_auction).collect())
    }

    async fn get_by_seller(&self, seller_id: u64) -> RepoResult<Vec<economy::AuctionListing>> {
        let rows: Vec<super::postgres::AuctionRow> = sqlx::query_as(
            "SELECT id, seller_id, item_template_id, quantity, buyout_price,
                    starting_bid, current_bid, highest_bidder,
                    created_at, expires_at, status, tax_rate
             FROM auctions WHERE seller_id = $1
             ORDER BY created_at DESC",
        )
        .bind(seller_id as i64)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows.iter().map(row_to_auction).collect())
    }

    async fn create(&self, listing: &economy::AuctionListing) -> RepoResult<u64> {
        let id = self
            .store
            .create_auction(
                listing.seller_id as i64,
                &listing.item_template_id,
                listing.quantity as i32,
                listing.buyout_price as i64,
                listing.starting_bid as i64,
                24, // default 24h duration
            )
            .await?;
        Ok(id as u64)
    }

    async fn place_bid(&self, auction_id: u64, bidder_id: u64, amount: u64) -> RepoResult<()> {
        self.store
            .place_bid(auction_id as i64, bidder_id as i64, amount as i64)
            .await?;
        Ok(())
    }

    async fn buyout(&self, auction_id: u64, buyer_id: u64) -> RepoResult<()> {
        self.store
            .buyout_auction(auction_id as i64, buyer_id as i64)
            .await?;
        Ok(())
    }

    async fn cancel(&self, auction_id: u64) -> RepoResult<()> {
        sqlx::query("UPDATE auctions SET status = 3 WHERE id = $1 AND status = 0")
            .bind(auction_id as i64)
            .execute(self.store.pool())
            .await?;
        Ok(())
    }

    async fn expire_old(&self) -> RepoResult<u32> {
        let count = self.store.expire_old_auctions().await?;
        Ok(count as u32)
    }
}

// ============================================================================
// ReputationRepo Adapter
// ============================================================================

pub struct PgReputationRepo {
    store: Arc<PostgresStore>,
}

impl PgReputationRepo {
    pub fn new(store: Arc<PostgresStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl ReputationRepo for PgReputationRepo {
    async fn get(
        &self,
        player_id: u64,
        faction_id: &str,
    ) -> RepoResult<Option<quests::PlayerReputation>> {
        let row = self
            .store
            .get_reputation(player_id as i64, faction_id)
            .await?;
        Ok(row.as_ref().map(row_to_reputation))
    }

    async fn get_all_for_player(
        &self,
        player_id: u64,
    ) -> RepoResult<Vec<quests::PlayerReputation>> {
        let rows: Vec<super::postgres::ReputationRow> = sqlx::query_as(
            "SELECT player_id, faction_id, reputation, standing
             FROM player_reputation WHERE player_id = $1
             ORDER BY faction_id",
        )
        .bind(player_id as i64)
        .fetch_all(self.store.pool())
        .await?;
        Ok(rows.iter().map(row_to_reputation).collect())
    }

    async fn add_reputation(
        &self,
        player_id: u64,
        faction_id: &str,
        amount: i32,
    ) -> RepoResult<quests::PlayerReputation> {
        let row = self
            .store
            .add_reputation(player_id as i64, faction_id, amount)
            .await?;
        Ok(row_to_reputation(&row))
    }
}
