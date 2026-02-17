//! PostgreSQL Storage - Player data persistence
//!
//! All player-mutable data lives in PostgreSQL (shared with Nakama).
//! Uses `sqlx` for async, compile-time-checked queries.
//!
//! ## Tables
//! - players, mastery, inventory, player_abilities, wallets
//! - trades, auctions, transaction_log
//! - guilds, guild_members, friendships, mail
//! - player_quests, player_reputation, player_seasons, player_achievements
//! - shadow_recordings, duel_results, player_echoes, leaderboards

use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::FromRow;
use tracing::{debug, info};

use super::migrations;

/// PostgreSQL connection pool wrapper
#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
}

/// Error type for PostgreSQL operations
#[derive(Debug, thiserror::Error)]
pub enum PostgresError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Migration error: {0}")]
    Migration(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Constraint violation: {0}")]
    Constraint(String),
    #[error("Insufficient funds: have {have}, need {need}")]
    InsufficientFunds { have: u64, need: u64 },
}

impl PostgresStore {
    /// Connect to PostgreSQL and run migrations
    pub async fn new(database_url: &str, max_connections: u32) -> Result<Self, PostgresError> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;

        info!("PostgreSQL connected (max_connections={})", max_connections);

        let store = Self { pool };
        store.run_migrations().await?;

        Ok(store)
    }

    /// Connect with an existing pool (for testing / shared Nakama pool)
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get reference to the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Run all pending migrations
    pub async fn run_migrations(&self) -> Result<(), PostgresError> {
        // Create migrations tracking table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS _migrations (
                name VARCHAR(100) PRIMARY KEY,
                applied_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
            )",
        )
        .execute(&self.pool)
        .await?;

        for (name, sql) in migrations::get_migrations() {
            let applied: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM _migrations WHERE name = $1)")
                    .bind(name)
                    .fetch_one(&self.pool)
                    .await?;

            if !applied {
                info!("Running migration: {}", name);
                sqlx::raw_sql(sql)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| PostgresError::Migration(format!("{}: {}", name, e)))?;

                sqlx::query("INSERT INTO _migrations (name) VALUES ($1)")
                    .bind(name)
                    .execute(&self.pool)
                    .await?;

                info!("Migration applied: {}", name);
            } else {
                debug!("Migration already applied: {}", name);
            }
        }

        Ok(())
    }

    // ========================================================================
    // Player Operations
    // ========================================================================

    /// Create a new player
    #[allow(clippy::too_many_arguments)]
    pub async fn create_player(
        &self,
        username: &str,
        password_hash: &str,
        str_stat: i16,
        dex_stat: i16,
        int_stat: i16,
        vit_stat: i16,
        luk_stat: i16,
    ) -> Result<i64, PostgresError> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO players (username, password_hash, base_str, base_dex, base_int, base_vit, base_luk)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id"
        )
        .bind(username)
        .bind(password_hash)
        .bind(str_stat)
        .bind(dex_stat)
        .bind(int_stat)
        .bind(vit_stat)
        .bind(luk_stat)
        .fetch_one(&self.pool)
        .await?;

        info!("Created player: {} (id={})", username, id);
        Ok(id)
    }

    /// Get player by ID
    pub async fn get_player(&self, id: i64) -> Result<Option<PlayerRow>, PostgresError> {
        let row = sqlx::query_as::<_, PlayerRow>(
            "SELECT id, username, created_at, last_login, playtime_seconds,
                    base_str, base_dex, base_int, base_vit, base_luk,
                    health, max_health, kinetic_energy, thermal_energy, semantic_energy,
                    floor_id, pos_x, pos_y, pos_z, rot_pitch, rot_yaw,
                    is_alive, respawn_floor
             FROM players WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// Get player by username
    pub async fn get_player_by_username(
        &self,
        username: &str,
    ) -> Result<Option<PlayerRow>, PostgresError> {
        let row = sqlx::query_as::<_, PlayerRow>(
            "SELECT id, username, created_at, last_login, playtime_seconds,
                    base_str, base_dex, base_int, base_vit, base_luk,
                    health, max_health, kinetic_energy, thermal_energy, semantic_energy,
                    floor_id, pos_x, pos_y, pos_z, rot_pitch, rot_yaw,
                    is_alive, respawn_floor
             FROM players WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// Update player position
    pub async fn update_player_position(
        &self,
        player_id: i64,
        floor_id: i32,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<(), PostgresError> {
        sqlx::query(
            "UPDATE players SET floor_id = $2, pos_x = $3, pos_y = $4, pos_z = $5 WHERE id = $1",
        )
        .bind(player_id)
        .bind(floor_id)
        .bind(x)
        .bind(y)
        .bind(z)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update player health
    pub async fn update_player_health(
        &self,
        player_id: i64,
        health: f32,
        is_alive: bool,
    ) -> Result<(), PostgresError> {
        sqlx::query("UPDATE players SET health = $2, is_alive = $3 WHERE id = $1")
            .bind(player_id)
            .bind(health)
            .bind(is_alive)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Delete player
    pub async fn delete_player(&self, id: i64) -> Result<(), PostgresError> {
        sqlx::query("DELETE FROM players WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Count players
    pub async fn count_players(&self) -> Result<i64, PostgresError> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM players")
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    // ========================================================================
    // Mastery Operations
    // ========================================================================

    /// Get mastery for a specific domain
    pub async fn get_mastery(
        &self,
        player_id: i64,
        domain: &str,
    ) -> Result<Option<MasteryRow>, PostgresError> {
        let row = sqlx::query_as::<_, MasteryRow>(
            "SELECT player_id, domain, experience, tier, specialization
             FROM mastery WHERE player_id = $1 AND domain = $2",
        )
        .bind(player_id)
        .bind(domain)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// Get all mastery domains for a player
    pub async fn get_all_mastery(&self, player_id: i64) -> Result<Vec<MasteryRow>, PostgresError> {
        let rows = sqlx::query_as::<_, MasteryRow>(
            "SELECT player_id, domain, experience, tier, specialization
             FROM mastery WHERE player_id = $1 ORDER BY domain",
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Add experience to a mastery domain (handles tier-up logic)
    pub async fn add_mastery_experience(
        &self,
        player_id: i64,
        domain: &str,
        exp: i64,
    ) -> Result<MasteryRow, PostgresError> {
        // Add experience and calculate new tier
        let row = sqlx::query_as::<_, MasteryRow>(
            "UPDATE mastery SET experience = experience + $3,
                    tier = CASE
                        WHEN experience + $3 >= 1000000 THEN 5  -- Grandmaster
                        WHEN experience + $3 >= 500000 THEN 4   -- Master
                        WHEN experience + $3 >= 100000 THEN 3   -- Expert
                        WHEN experience + $3 >= 25000 THEN 2    -- Journeyman
                        WHEN experience + $3 >= 5000 THEN 1     -- Apprentice
                        ELSE 0                                  -- Novice
                    END
             WHERE player_id = $1 AND domain = $2
             RETURNING player_id, domain, experience, tier, specialization",
        )
        .bind(player_id)
        .bind(domain)
        .bind(exp)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    /// Set specialization for a mastery domain (requires Expert tier)
    pub async fn set_mastery_specialization(
        &self,
        player_id: i64,
        domain: &str,
        spec: &str,
    ) -> Result<(), PostgresError> {
        let result = sqlx::query(
            "UPDATE mastery SET specialization = $3
             WHERE player_id = $1 AND domain = $2 AND tier >= 3",
        )
        .bind(player_id)
        .bind(domain)
        .bind(spec)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(PostgresError::Constraint(
                "Specialization requires Expert tier (tier >= 3)".to_string(),
            ));
        }

        Ok(())
    }

    // ========================================================================
    // Inventory Operations
    // ========================================================================

    /// Get player bag inventory
    pub async fn get_bag(&self, player_id: i64) -> Result<Vec<InventoryRow>, PostgresError> {
        let rows = sqlx::query_as::<_, InventoryRow>(
            "SELECT id, player_id, item_template_id, quantity, slot_type, slot_index,
                    instance_id, durability, enhancement, sockets, rolled_effects
             FROM inventory WHERE player_id = $1 AND slot_type = 0
             ORDER BY slot_index",
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Get player equipment
    pub async fn get_equipment(&self, player_id: i64) -> Result<Vec<InventoryRow>, PostgresError> {
        let rows = sqlx::query_as::<_, InventoryRow>(
            "SELECT id, player_id, item_template_id, quantity, slot_type, slot_index,
                    instance_id, durability, enhancement, sockets, rolled_effects
             FROM inventory WHERE player_id = $1 AND slot_type = 1
             ORDER BY slot_index",
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Add item to player inventory
    pub async fn add_item(
        &self,
        player_id: i64,
        item_template_id: &str,
        quantity: i32,
        slot_type: i16,
    ) -> Result<i64, PostgresError> {
        // Try to stack with existing item first
        let existing = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM inventory
             WHERE player_id = $1 AND item_template_id = $2 AND slot_type = $3
             AND instance_id IS NULL
             LIMIT 1",
        )
        .bind(player_id)
        .bind(item_template_id)
        .bind(slot_type)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(slot_id) = existing {
            // Stack onto existing slot
            sqlx::query("UPDATE inventory SET quantity = quantity + $2 WHERE id = $1")
                .bind(slot_id)
                .bind(quantity)
                .execute(&self.pool)
                .await?;
            Ok(slot_id)
        } else {
            // Find next available slot
            let next_slot: i16 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(slot_index) + 1, 0) FROM inventory
                 WHERE player_id = $1 AND slot_type = $2",
            )
            .bind(player_id)
            .bind(slot_type)
            .fetch_one(&self.pool)
            .await?;

            let id: i64 = sqlx::query_scalar(
                "INSERT INTO inventory (player_id, item_template_id, quantity, slot_type, slot_index)
                 VALUES ($1, $2, $3, $4, $5) RETURNING id"
            )
            .bind(player_id)
            .bind(item_template_id)
            .bind(quantity)
            .bind(slot_type)
            .bind(next_slot)
            .fetch_one(&self.pool)
            .await?;

            Ok(id)
        }
    }

    /// Remove item quantity from a slot
    pub async fn remove_item(&self, slot_id: i64, quantity: i32) -> Result<(), PostgresError> {
        let current: i32 = sqlx::query_scalar("SELECT quantity FROM inventory WHERE id = $1")
            .bind(slot_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| PostgresError::NotFound(format!("Inventory slot {}", slot_id)))?;

        if quantity >= current {
            sqlx::query("DELETE FROM inventory WHERE id = $1")
                .bind(slot_id)
                .execute(&self.pool)
                .await?;
        } else {
            sqlx::query("UPDATE inventory SET quantity = quantity - $2 WHERE id = $1")
                .bind(slot_id)
                .bind(quantity)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Equip an item (move from bag to equipment slot)
    pub async fn equip_item(
        &self,
        player_id: i64,
        slot_id: i64,
        equipment_slot: i16,
    ) -> Result<(), PostgresError> {
        sqlx::query(
            "UPDATE inventory SET slot_type = 1, slot_index = $3
             WHERE id = $1 AND player_id = $2",
        )
        .bind(slot_id)
        .bind(player_id)
        .bind(equipment_slot)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ========================================================================
    // Wallet Operations
    // ========================================================================

    /// Get player wallet
    pub async fn get_wallet(&self, player_id: i64) -> Result<WalletRow, PostgresError> {
        let row = sqlx::query_as::<_, WalletRow>(
            "SELECT player_id, gold, premium_currency, honor_points, event_tokens
             FROM wallets WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| PostgresError::NotFound(format!("Wallet for player {}", player_id)))?;

        Ok(row)
    }

    /// Add gold to wallet
    pub async fn add_gold(&self, player_id: i64, amount: i64) -> Result<i64, PostgresError> {
        let new_gold: i64 = sqlx::query_scalar(
            "UPDATE wallets SET gold = gold + $2 WHERE player_id = $1 RETURNING gold",
        )
        .bind(player_id)
        .bind(amount)
        .fetch_one(&self.pool)
        .await?;

        Ok(new_gold)
    }

    /// Remove gold from wallet
    pub async fn remove_gold(&self, player_id: i64, amount: i64) -> Result<i64, PostgresError> {
        let current: i64 = sqlx::query_scalar("SELECT gold FROM wallets WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(&self.pool)
            .await?;

        if current < amount {
            return Err(PostgresError::InsufficientFunds {
                have: current as u64,
                need: amount as u64,
            });
        }

        let new_gold: i64 = sqlx::query_scalar(
            "UPDATE wallets SET gold = gold - $2 WHERE player_id = $1 RETURNING gold",
        )
        .bind(player_id)
        .bind(amount)
        .fetch_one(&self.pool)
        .await?;

        Ok(new_gold)
    }

    /// Transfer gold between players (atomic transaction)
    pub async fn transfer_gold(
        &self,
        from: i64,
        to: i64,
        amount: i64,
    ) -> Result<(), PostgresError> {
        let mut tx = self.pool.begin().await?;

        // Check sender balance
        let sender_gold: i64 =
            sqlx::query_scalar("SELECT gold FROM wallets WHERE player_id = $1 FOR UPDATE")
                .bind(from)
                .fetch_one(&mut *tx)
                .await?;

        if sender_gold < amount {
            return Err(PostgresError::InsufficientFunds {
                have: sender_gold as u64,
                need: amount as u64,
            });
        }

        // Deduct from sender
        sqlx::query("UPDATE wallets SET gold = gold - $2 WHERE player_id = $1")
            .bind(from)
            .bind(amount)
            .execute(&mut *tx)
            .await?;

        // Add to receiver
        sqlx::query("UPDATE wallets SET gold = gold + $2 WHERE player_id = $1")
            .bind(to)
            .bind(amount)
            .execute(&mut *tx)
            .await?;

        // Log transaction
        sqlx::query(
            "INSERT INTO transaction_log (tx_type, from_player, to_player, gold_amount)
             VALUES ('transfer', $1, $2, $3)",
        )
        .bind(from)
        .bind(to)
        .bind(amount)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    // ========================================================================
    // Guild Operations
    // ========================================================================

    /// Create a guild
    pub async fn create_guild(
        &self,
        name: &str,
        tag: &str,
        leader_id: i64,
        description: &str,
    ) -> Result<i64, PostgresError> {
        let mut tx = self.pool.begin().await?;

        let guild_id: i64 = sqlx::query_scalar(
            "INSERT INTO guilds (name, tag, leader_id, description)
             VALUES ($1, $2, $3, $4) RETURNING id",
        )
        .bind(name)
        .bind(tag)
        .bind(leader_id)
        .bind(description)
        .fetch_one(&mut *tx)
        .await?;

        // Add leader as member with rank 4
        sqlx::query("INSERT INTO guild_members (guild_id, player_id, rank) VALUES ($1, $2, 4)")
            .bind(guild_id)
            .bind(leader_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        info!("Created guild: {} [{}] (id={})", name, tag, guild_id);
        Ok(guild_id)
    }

    /// Get guild by ID
    pub async fn get_guild(&self, id: i64) -> Result<Option<GuildRow>, PostgresError> {
        let row = sqlx::query_as::<_, GuildRow>(
            "SELECT id, name, tag, leader_id, created_at, level, experience,
                    max_members, description, motd, is_recruiting, bank_gold
             FROM guilds WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// Add member to guild
    pub async fn add_guild_member(
        &self,
        guild_id: i64,
        player_id: i64,
        rank: i16,
    ) -> Result<(), PostgresError> {
        sqlx::query("INSERT INTO guild_members (guild_id, player_id, rank) VALUES ($1, $2, $3)")
            .bind(guild_id)
            .bind(player_id)
            .bind(rank)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Remove member from guild
    pub async fn remove_guild_member(
        &self,
        guild_id: i64,
        player_id: i64,
    ) -> Result<(), PostgresError> {
        sqlx::query("DELETE FROM guild_members WHERE guild_id = $1 AND player_id = $2")
            .bind(guild_id)
            .bind(player_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get guild members
    pub async fn get_guild_members(
        &self,
        guild_id: i64,
    ) -> Result<Vec<GuildMemberRow>, PostgresError> {
        let rows = sqlx::query_as::<_, GuildMemberRow>(
            "SELECT gm.guild_id, gm.player_id, gm.rank, gm.joined_at, gm.contribution, gm.note,
                    p.username
             FROM guild_members gm
             JOIN players p ON gm.player_id = p.id
             WHERE gm.guild_id = $1
             ORDER BY gm.rank DESC, gm.joined_at",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    // ========================================================================
    // Quest Progress Operations
    // ========================================================================

    /// Start a quest for a player
    pub async fn start_quest(
        &self,
        player_id: i64,
        quest_id: &str,
        num_objectives: usize,
    ) -> Result<(), PostgresError> {
        let objectives: Vec<serde_json::Value> = (0..num_objectives)
            .map(|i| {
                serde_json::json!({
                    "index": i,
                    "current": 0,
                    "complete": false
                })
            })
            .collect();

        sqlx::query(
            "INSERT INTO player_quests (player_id, quest_id, status, objectives, started_at)
             VALUES ($1, $2, 1, $3, NOW())
             ON CONFLICT (player_id, quest_id) DO UPDATE SET status = 1, started_at = NOW()",
        )
        .bind(player_id)
        .bind(quest_id)
        .bind(serde_json::Value::Array(objectives))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update quest objective progress
    pub async fn update_quest_objective(
        &self,
        player_id: i64,
        quest_id: &str,
        objective_idx: i32,
        current_count: i32,
    ) -> Result<(), PostgresError> {
        sqlx::query(
            "UPDATE player_quests SET objectives = jsonb_set(
                objectives,
                ('{' || $3 || ',current}')::text[],
                to_jsonb($4::int)
             ) WHERE player_id = $1 AND quest_id = $2",
        )
        .bind(player_id)
        .bind(quest_id)
        .bind(objective_idx)
        .bind(current_count)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Complete a quest
    pub async fn complete_quest(
        &self,
        player_id: i64,
        quest_id: &str,
    ) -> Result<(), PostgresError> {
        sqlx::query(
            "UPDATE player_quests SET status = 3, completed_at = NOW(),
                    times_completed = times_completed + 1
             WHERE player_id = $1 AND quest_id = $2",
        )
        .bind(player_id)
        .bind(quest_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get active quests for a player
    pub async fn get_active_quests(
        &self,
        player_id: i64,
    ) -> Result<Vec<QuestProgressRow>, PostgresError> {
        let rows = sqlx::query_as::<_, QuestProgressRow>(
            "SELECT player_id, quest_id, status, objectives, started_at, completed_at, times_completed
             FROM player_quests WHERE player_id = $1 AND status IN (1, 2)
             ORDER BY started_at"
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    // ========================================================================
    // Auction House Operations
    // ========================================================================

    /// Create an auction listing
    pub async fn create_auction(
        &self,
        seller_id: i64,
        item_template_id: &str,
        quantity: i32,
        buyout_price: i64,
        starting_bid: i64,
        duration_hours: i32,
    ) -> Result<i64, PostgresError> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO auctions (seller_id, item_template_id, quantity, buyout_price,
                    starting_bid, expires_at)
             VALUES ($1, $2, $3, $4, $5, NOW() + make_interval(hours => $6))
             RETURNING id",
        )
        .bind(seller_id)
        .bind(item_template_id)
        .bind(quantity)
        .bind(buyout_price)
        .bind(starting_bid)
        .bind(duration_hours)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get active auction listings
    pub async fn get_active_auctions(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<AuctionRow>, PostgresError> {
        let rows = sqlx::query_as::<_, AuctionRow>(
            "SELECT id, seller_id, item_template_id, quantity, buyout_price,
                    starting_bid, current_bid, highest_bidder,
                    created_at, expires_at, status, tax_rate
             FROM auctions WHERE status = 0 AND expires_at > NOW()
             ORDER BY created_at DESC
             LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Place a bid on an auction
    pub async fn place_bid(
        &self,
        auction_id: i64,
        bidder_id: i64,
        amount: i64,
    ) -> Result<(), PostgresError> {
        let mut tx = self.pool.begin().await?;

        // Get current auction state
        let (current_bid, status): (Option<i64>, i16) =
            sqlx::query_as("SELECT current_bid, status FROM auctions WHERE id = $1 FOR UPDATE")
                .bind(auction_id)
                .fetch_one(&mut *tx)
                .await?;

        if status != 0 {
            return Err(PostgresError::Constraint(
                "Auction is not active".to_string(),
            ));
        }

        let current = current_bid.unwrap_or(0);
        if amount <= current {
            return Err(PostgresError::Constraint(format!(
                "Bid {} must be higher than current bid {}",
                amount, current
            )));
        }

        // Update auction
        sqlx::query("UPDATE auctions SET current_bid = $2, highest_bidder = $3 WHERE id = $1")
            .bind(auction_id)
            .bind(amount)
            .bind(bidder_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Buyout an auction
    pub async fn buyout_auction(
        &self,
        auction_id: i64,
        buyer_id: i64,
    ) -> Result<(), PostgresError> {
        let mut tx = self.pool.begin().await?;

        // Get auction info
        let row: AuctionRow = sqlx::query_as(
            "SELECT id, seller_id, item_template_id, quantity, buyout_price,
                    starting_bid, current_bid, highest_bidder,
                    created_at, expires_at, status, tax_rate
             FROM auctions WHERE id = $1 AND status = 0 FOR UPDATE",
        )
        .bind(auction_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| PostgresError::NotFound(format!("Active auction {}", auction_id)))?;

        let tax = row.buyout_price * row.tax_rate as i64 / 100;
        let seller_receives = row.buyout_price - tax;

        // Deduct from buyer
        let buyer_gold: i64 =
            sqlx::query_scalar("SELECT gold FROM wallets WHERE player_id = $1 FOR UPDATE")
                .bind(buyer_id)
                .fetch_one(&mut *tx)
                .await?;

        if buyer_gold < row.buyout_price {
            return Err(PostgresError::InsufficientFunds {
                have: buyer_gold as u64,
                need: row.buyout_price as u64,
            });
        }

        sqlx::query("UPDATE wallets SET gold = gold - $2 WHERE player_id = $1")
            .bind(buyer_id)
            .bind(row.buyout_price)
            .execute(&mut *tx)
            .await?;

        // Pay seller (minus tax)
        sqlx::query("UPDATE wallets SET gold = gold + $2 WHERE player_id = $1")
            .bind(row.seller_id)
            .bind(seller_receives)
            .execute(&mut *tx)
            .await?;

        // Mark auction as sold
        sqlx::query("UPDATE auctions SET status = 1 WHERE id = $1")
            .bind(auction_id)
            .execute(&mut *tx)
            .await?;

        // Log transaction
        sqlx::query(
            "INSERT INTO transaction_log (tx_type, from_player, to_player, gold_amount, item_id, item_quantity)
             VALUES ('auction', $1, $2, $3, $4, $5)"
        )
        .bind(buyer_id)
        .bind(row.seller_id)
        .bind(row.buyout_price)
        .bind(&row.item_template_id)
        .bind(row.quantity)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Expire old auctions
    pub async fn expire_old_auctions(&self) -> Result<u64, PostgresError> {
        let result =
            sqlx::query("UPDATE auctions SET status = 2 WHERE status = 0 AND expires_at <= NOW()")
                .execute(&self.pool)
                .await?;

        let count = result.rows_affected();
        if count > 0 {
            info!("Expired {} auctions", count);
        }
        Ok(count)
    }

    // ========================================================================
    // Reputation Operations
    // ========================================================================

    /// Get player reputation with a faction
    pub async fn get_reputation(
        &self,
        player_id: i64,
        faction_id: &str,
    ) -> Result<Option<ReputationRow>, PostgresError> {
        let row = sqlx::query_as::<_, ReputationRow>(
            "SELECT player_id, faction_id, reputation, standing
             FROM player_reputation WHERE player_id = $1 AND faction_id = $2",
        )
        .bind(player_id)
        .bind(faction_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// Add reputation with a faction
    pub async fn add_reputation(
        &self,
        player_id: i64,
        faction_id: &str,
        amount: i32,
    ) -> Result<ReputationRow, PostgresError> {
        let row = sqlx::query_as::<_, ReputationRow>(
            "INSERT INTO player_reputation (player_id, faction_id, reputation, standing)
             VALUES ($1, $2, $3, CASE
                 WHEN $3 >= 21000 THEN 7  -- Exalted
                 WHEN $3 >= 12000 THEN 6  -- Revered
                 WHEN $3 >= 6000 THEN 5   -- Honored
                 WHEN $3 >= 3000 THEN 4   -- Friendly
                 WHEN $3 >= 0 THEN 3      -- Neutral
                 WHEN $3 >= -3000 THEN 2  -- Unfriendly
                 WHEN $3 >= -6000 THEN 1  -- Hostile
                 ELSE 0                   -- Hated
             END)
             ON CONFLICT (player_id, faction_id) DO UPDATE SET
                 reputation = player_reputation.reputation + $3,
                 standing = CASE
                     WHEN player_reputation.reputation + $3 >= 21000 THEN 7
                     WHEN player_reputation.reputation + $3 >= 12000 THEN 6
                     WHEN player_reputation.reputation + $3 >= 6000 THEN 5
                     WHEN player_reputation.reputation + $3 >= 3000 THEN 4
                     WHEN player_reputation.reputation + $3 >= 0 THEN 3
                     WHEN player_reputation.reputation + $3 >= -3000 THEN 2
                     WHEN player_reputation.reputation + $3 >= -6000 THEN 1
                     ELSE 0
                 END
             RETURNING player_id, faction_id, reputation, standing",
        )
        .bind(player_id)
        .bind(faction_id)
        .bind(amount)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    // ========================================================================
    // Friendship Operations
    // ========================================================================

    /// Send friend request
    pub async fn send_friend_request(
        &self,
        player_id: i64,
        friend_id: i64,
    ) -> Result<(), PostgresError> {
        sqlx::query(
            "INSERT INTO friendships (player_id, friend_id, status)
             VALUES ($1, $2, 0)
             ON CONFLICT (player_id, friend_id) DO NOTHING",
        )
        .bind(player_id)
        .bind(friend_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Accept friend request
    pub async fn accept_friend_request(
        &self,
        player_id: i64,
        friend_id: i64,
    ) -> Result<(), PostgresError> {
        let mut tx = self.pool.begin().await?;

        // Update original request
        sqlx::query(
            "UPDATE friendships SET status = 1
             WHERE player_id = $1 AND friend_id = $2 AND status = 0",
        )
        .bind(friend_id)
        .bind(player_id)
        .execute(&mut *tx)
        .await?;

        // Create reverse relationship
        sqlx::query(
            "INSERT INTO friendships (player_id, friend_id, status) VALUES ($1, $2, 1)
             ON CONFLICT (player_id, friend_id) DO UPDATE SET status = 1",
        )
        .bind(player_id)
        .bind(friend_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    // ========================================================================
    // Mail Operations
    // ========================================================================

    /// Send mail
    pub async fn send_mail(
        &self,
        sender_id: i64,
        sender_name: &str,
        recipient_id: i64,
        subject: &str,
        body: &str,
        gold: i64,
    ) -> Result<i64, PostgresError> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO mail (sender_id, sender_name, recipient_id, subject, body, gold_attached)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        )
        .bind(sender_id)
        .bind(sender_name)
        .bind(recipient_id)
        .bind(subject)
        .bind(body)
        .bind(gold)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get unread mail for player
    pub async fn get_unread_mail(&self, player_id: i64) -> Result<Vec<MailRow>, PostgresError> {
        let rows = sqlx::query_as::<_, MailRow>(
            "SELECT id, sender_id, sender_name, recipient_id, subject, body,
                    gold_attached, items_attached, status, sent_at, expires_at
             FROM mail WHERE recipient_id = $1 AND status IN (0, 1) AND expires_at > NOW()
             ORDER BY sent_at DESC",
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    // ========================================================================
    // Death Echo Operations (Tower Game unique)
    // ========================================================================

    /// Record a death echo
    #[allow(clippy::too_many_arguments)]
    pub async fn create_echo(
        &self,
        player_id: i64,
        player_name: &str,
        floor_id: i32,
        x: f32,
        y: f32,
        z: f32,
        cause: &str,
        echo_type: &str,
        message: Option<&str>,
        semantic_tags: &serde_json::Value,
    ) -> Result<i64, PostgresError> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO player_echoes (player_id, player_name, floor_id, pos_x, pos_y, pos_z,
                    cause_of_death, echo_type, echo_message, semantic_tags)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id",
        )
        .bind(player_id)
        .bind(player_name)
        .bind(floor_id)
        .bind(x)
        .bind(y)
        .bind(z)
        .bind(cause)
        .bind(echo_type)
        .bind(message)
        .bind(semantic_tags)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get active echoes on a floor
    pub async fn get_floor_echoes(&self, floor_id: i32) -> Result<Vec<EchoRow>, PostgresError> {
        let rows = sqlx::query_as::<_, EchoRow>(
            "SELECT id, player_id, player_name, floor_id, pos_x, pos_y, pos_z,
                    cause_of_death, last_action, echo_type, echo_message, echo_strength,
                    semantic_tags, created_at, expires_at
             FROM player_echoes WHERE floor_id = $1 AND expires_at > NOW()
             ORDER BY created_at DESC",
        )
        .bind(floor_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    // ========================================================================
    // Shadow Duel Operations
    // ========================================================================

    /// Record a shadow for PvP
    #[allow(clippy::too_many_arguments)]
    pub async fn create_shadow(
        &self,
        player_id: i64,
        player_name: &str,
        equipment: &serde_json::Value,
        abilities: &serde_json::Value,
        mastery: &serde_json::Value,
        actions: &[u8],
        pvp_rating: i32,
    ) -> Result<i64, PostgresError> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO shadow_recordings (player_id, player_name, equipment, abilities, mastery, actions, pvp_rating)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id"
        )
        .bind(player_id)
        .bind(player_name)
        .bind(equipment)
        .bind(abilities)
        .bind(mastery)
        .bind(actions)
        .bind(pvp_rating)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get shadows for matchmaking (find opponents near rating)
    pub async fn find_shadow_opponents(
        &self,
        rating: i32,
        range: i32,
        limit: i32,
    ) -> Result<Vec<ShadowRow>, PostgresError> {
        let rows = sqlx::query_as::<_, ShadowRow>(
            "SELECT id, player_id, player_name, equipment, abilities, mastery,
                    recorded_at, pvp_rating
             FROM shadow_recordings
             WHERE pvp_rating BETWEEN $1 - $2 AND $1 + $2
             ORDER BY RANDOM()
             LIMIT $3",
        )
        .bind(rating)
        .bind(range)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Record duel result
    pub async fn record_duel_result(
        &self,
        challenger_id: i64,
        shadow_owner_id: i64,
        winner_id: i64,
        rating_change_challenger: i32,
        rating_change_defender: i32,
        duration_ms: i32,
    ) -> Result<i64, PostgresError> {
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO duel_results (challenger_id, shadow_owner_id, winner_id,
                    rating_change_challenger, rating_change_defender, duration_ms)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
        )
        .bind(challenger_id)
        .bind(shadow_owner_id)
        .bind(winner_id)
        .bind(rating_change_challenger)
        .bind(rating_change_defender)
        .bind(duration_ms)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    // ========================================================================
    // Leaderboard Operations
    // ========================================================================

    /// Update leaderboard entry
    pub async fn update_leaderboard(
        &self,
        board_type: i16,
        rank: i32,
        player_id: i64,
        player_name: &str,
        score: i64,
    ) -> Result<(), PostgresError> {
        sqlx::query(
            "INSERT INTO leaderboards (board_type, rank, player_id, player_name, score, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW())
             ON CONFLICT (board_type, rank) DO UPDATE SET
                player_id = $3, player_name = $4, score = $5, updated_at = NOW()",
        )
        .bind(board_type)
        .bind(rank)
        .bind(player_id)
        .bind(player_name)
        .bind(score)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get top N from a leaderboard
    pub async fn get_leaderboard(
        &self,
        board_type: i16,
        limit: i32,
    ) -> Result<Vec<LeaderboardRow>, PostgresError> {
        let rows = sqlx::query_as::<_, LeaderboardRow>(
            "SELECT board_type, rank, player_id, player_name, score, extra_data, updated_at
             FROM leaderboards WHERE board_type = $1
             ORDER BY rank ASC
             LIMIT $2",
        )
        .bind(board_type)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    // ========================================================================
    // Season Pass Operations
    // ========================================================================

    /// Get season progress
    pub async fn get_season_progress(
        &self,
        player_id: i64,
        season_id: &str,
    ) -> Result<Option<SeasonProgressRow>, PostgresError> {
        let row = sqlx::query_as::<_, SeasonProgressRow>(
            "SELECT player_id, season_id, level, experience, has_premium, claimed_levels
             FROM player_seasons WHERE player_id = $1 AND season_id = $2",
        )
        .bind(player_id)
        .bind(season_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// Add season experience
    pub async fn add_season_experience(
        &self,
        player_id: i64,
        season_id: &str,
        exp: i64,
    ) -> Result<SeasonProgressRow, PostgresError> {
        let row = sqlx::query_as::<_, SeasonProgressRow>(
            "INSERT INTO player_seasons (player_id, season_id, experience)
             VALUES ($1, $2, $3)
             ON CONFLICT (player_id, season_id) DO UPDATE
                SET experience = player_seasons.experience + $3,
                    level = 1 + ((player_seasons.experience + $3) / 10000)::smallint
             RETURNING player_id, season_id, level, experience, has_premium, claimed_levels",
        )
        .bind(player_id)
        .bind(season_id)
        .bind(exp)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    // ========================================================================
    // Achievement Operations
    // ========================================================================

    /// Update achievement progress
    pub async fn update_achievement(
        &self,
        player_id: i64,
        achievement_id: &str,
        progress: &serde_json::Value,
        completed: bool,
    ) -> Result<(), PostgresError> {
        sqlx::query(
            "INSERT INTO player_achievements (player_id, achievement_id, progress, is_completed, completed_at)
             VALUES ($1, $2, $3, $4, CASE WHEN $4 THEN NOW() ELSE NULL END)
             ON CONFLICT (player_id, achievement_id) DO UPDATE SET
                progress = $3, is_completed = $4,
                completed_at = CASE WHEN $4 AND NOT player_achievements.is_completed THEN NOW()
                                    ELSE player_achievements.completed_at END"
        )
        .bind(player_id)
        .bind(achievement_id)
        .bind(progress)
        .bind(completed)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get player achievements
    pub async fn get_achievements(
        &self,
        player_id: i64,
    ) -> Result<Vec<AchievementRow>, PostgresError> {
        let rows = sqlx::query_as::<_, AchievementRow>(
            "SELECT player_id, achievement_id, is_completed, progress, completed_at
             FROM player_achievements WHERE player_id = $1
             ORDER BY achievement_id",
        )
        .bind(player_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

// ============================================================================
// Row types (for sqlx query_as mapping)
// ============================================================================

#[derive(Debug, Clone, FromRow)]
pub struct PlayerRow {
    pub id: i64,
    pub username: String,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
    pub playtime_seconds: i64,
    pub base_str: i16,
    pub base_dex: i16,
    pub base_int: i16,
    pub base_vit: i16,
    pub base_luk: i16,
    pub health: f32,
    pub max_health: f32,
    pub kinetic_energy: f32,
    pub thermal_energy: f32,
    pub semantic_energy: f32,
    pub floor_id: i32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub rot_pitch: f32,
    pub rot_yaw: f32,
    pub is_alive: bool,
    pub respawn_floor: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct MasteryRow {
    pub player_id: i64,
    pub domain: String,
    pub experience: i64,
    pub tier: i16,
    pub specialization: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
pub struct InventoryRow {
    pub id: i64,
    pub player_id: i64,
    pub item_template_id: String,
    pub quantity: i32,
    pub slot_type: i16,
    pub slot_index: i16,
    pub instance_id: Option<i64>,
    pub durability: Option<f32>,
    pub enhancement: Option<i16>,
    pub sockets: Option<serde_json::Value>,
    pub rolled_effects: Option<serde_json::Value>,
}

#[derive(Debug, Clone, FromRow)]
pub struct WalletRow {
    pub player_id: i64,
    pub gold: i64,
    pub premium_currency: i32,
    pub honor_points: i32,
    pub event_tokens: Option<serde_json::Value>,
}

#[derive(Debug, Clone, FromRow)]
pub struct GuildRow {
    pub id: i64,
    pub name: String,
    pub tag: String,
    pub leader_id: i64,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub level: i16,
    pub experience: i64,
    pub max_members: i16,
    pub description: Option<String>,
    pub motd: Option<String>,
    pub is_recruiting: bool,
    pub bank_gold: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct GuildMemberRow {
    pub guild_id: i64,
    pub player_id: i64,
    pub rank: i16,
    pub joined_at: Option<chrono::DateTime<chrono::Utc>>,
    pub contribution: i64,
    pub note: Option<String>,
    pub username: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct QuestProgressRow {
    pub player_id: i64,
    pub quest_id: String,
    pub status: i16,
    pub objectives: Option<serde_json::Value>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub times_completed: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct AuctionRow {
    pub id: i64,
    pub seller_id: i64,
    pub item_template_id: String,
    pub quantity: i32,
    pub buyout_price: i64,
    pub starting_bid: i64,
    pub current_bid: Option<i64>,
    pub highest_bidder: Option<i64>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub status: i16,
    pub tax_rate: i16,
}

#[derive(Debug, Clone, FromRow)]
pub struct ReputationRow {
    pub player_id: i64,
    pub faction_id: String,
    pub reputation: i32,
    pub standing: i16,
}

#[derive(Debug, Clone, FromRow)]
pub struct MailRow {
    pub id: i64,
    pub sender_id: Option<i64>,
    pub sender_name: String,
    pub recipient_id: i64,
    pub subject: String,
    pub body: Option<String>,
    pub gold_attached: i64,
    pub items_attached: Option<serde_json::Value>,
    pub status: i16,
    pub sent_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct EchoRow {
    pub id: i64,
    pub player_id: i64,
    pub player_name: String,
    pub floor_id: i32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub cause_of_death: String,
    pub last_action: Option<String>,
    pub echo_type: String,
    pub echo_message: Option<String>,
    pub echo_strength: f32,
    pub semantic_tags: Option<serde_json::Value>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ShadowRow {
    pub id: i64,
    pub player_id: i64,
    pub player_name: String,
    pub equipment: serde_json::Value,
    pub abilities: serde_json::Value,
    pub mastery: serde_json::Value,
    pub recorded_at: Option<chrono::DateTime<chrono::Utc>>,
    pub pvp_rating: i32,
}

#[derive(Debug, Clone, FromRow)]
pub struct LeaderboardRow {
    pub board_type: i16,
    pub rank: i32,
    pub player_id: i64,
    pub player_name: String,
    pub score: i64,
    pub extra_data: Option<serde_json::Value>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SeasonProgressRow {
    pub player_id: i64,
    pub season_id: String,
    pub level: i16,
    pub experience: i64,
    pub has_premium: bool,
    pub claimed_levels: Option<serde_json::Value>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AchievementRow {
    pub player_id: i64,
    pub achievement_id: String,
    pub is_completed: bool,
    pub progress: Option<serde_json::Value>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}
