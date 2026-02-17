//! Database Migrations - PostgreSQL schema for Tower Game
//!
//! Production-ready schema for all player-mutable data.
//! Static templates live in LMDB, not here.

/// SQL migration for creating all tables
pub const MIGRATION_V1: &str = r#"
-- ============================================================================
-- Tower Game Database Schema v1
-- Production-ready PostgreSQL schema
-- ============================================================================

-- Enable extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================================
-- 1. Players & Characters
-- ============================================================================

CREATE TABLE IF NOT EXISTS players (
    id              BIGSERIAL PRIMARY KEY,
    username        VARCHAR(50) UNIQUE NOT NULL,
    password_hash   VARCHAR(255) NOT NULL,
    created_at      TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    last_login      TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    playtime_seconds BIGINT DEFAULT 0,

    -- Base stats (immutable after creation, total = 20 points)
    base_str        SMALLINT NOT NULL DEFAULT 4 CHECK (base_str BETWEEN 1 AND 10),
    base_dex        SMALLINT NOT NULL DEFAULT 4 CHECK (base_dex BETWEEN 1 AND 10),
    base_int        SMALLINT NOT NULL DEFAULT 4 CHECK (base_int BETWEEN 1 AND 10),
    base_vit        SMALLINT NOT NULL DEFAULT 4 CHECK (base_vit BETWEEN 1 AND 10),
    base_luk        SMALLINT NOT NULL DEFAULT 4 CHECK (base_luk BETWEEN 1 AND 10),

    -- Current state
    health          REAL NOT NULL DEFAULT 100.0,
    max_health      REAL NOT NULL DEFAULT 100.0,
    kinetic_energy  REAL NOT NULL DEFAULT 100.0,
    thermal_energy  REAL NOT NULL DEFAULT 100.0,
    semantic_energy REAL NOT NULL DEFAULT 100.0,

    -- Position
    floor_id        INTEGER NOT NULL DEFAULT 1,
    pos_x           REAL NOT NULL DEFAULT 0.0,
    pos_y           REAL NOT NULL DEFAULT 0.0,
    pos_z           REAL NOT NULL DEFAULT 0.0,
    rot_pitch       REAL NOT NULL DEFAULT 0.0,
    rot_yaw         REAL NOT NULL DEFAULT 0.0,

    -- State flags
    is_alive        BOOLEAN NOT NULL DEFAULT TRUE,
    respawn_floor   INTEGER NOT NULL DEFAULT 1,

    -- Constraints
    CONSTRAINT check_stats_total CHECK (base_str + base_dex + base_int + base_vit + base_luk = 20)
);

CREATE INDEX idx_players_username ON players(username);
CREATE INDEX idx_players_floor ON players(floor_id);

-- ============================================================================
-- 2. Mastery Progress (21 domains per player)
-- ============================================================================

CREATE TABLE IF NOT EXISTS mastery (
    player_id       BIGINT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    domain          VARCHAR(50) NOT NULL,  -- 'SwordMastery', 'ParryMastery', etc.
    experience      BIGINT NOT NULL DEFAULT 0,
    tier            SMALLINT NOT NULL DEFAULT 0,  -- 0=Novice ... 5=Grandmaster
    specialization  VARCHAR(100),  -- Unlocked at Expert tier

    PRIMARY KEY (player_id, domain)
);

CREATE INDEX idx_mastery_player ON mastery(player_id);

-- ============================================================================
-- 3. Inventory System
-- ============================================================================

CREATE TABLE IF NOT EXISTS inventory (
    id              BIGSERIAL PRIMARY KEY,
    player_id       BIGINT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    item_template_id VARCHAR(100) NOT NULL,  -- Reference to LMDB ItemTemplate
    quantity        INTEGER NOT NULL DEFAULT 1 CHECK (quantity > 0),
    slot_type       SMALLINT NOT NULL DEFAULT 0,  -- 0=Bag, 1=Equipment, 2=Bank, 3=GuildBank
    slot_index      SMALLINT NOT NULL DEFAULT 0,

    -- Item instance data (for unique/non-stackable items)
    instance_id     BIGINT,
    durability      REAL DEFAULT 1.0,
    enhancement     SMALLINT DEFAULT 0,  -- +0 to +15
    sockets         JSONB DEFAULT '[]',   -- [{type: "red", gem_id: "ruby_t3"}]
    rolled_effects  JSONB DEFAULT '[]',   -- [{trigger: "on_hit", action: {...}}]

    CONSTRAINT unique_equipment_slot UNIQUE (player_id, slot_type, slot_index)
        DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX idx_inventory_player ON inventory(player_id);
CREATE INDEX idx_inventory_template ON inventory(item_template_id);
CREATE INDEX idx_inventory_slot ON inventory(player_id, slot_type);

-- ============================================================================
-- 4. Player Abilities
-- ============================================================================

CREATE TABLE IF NOT EXISTS player_abilities (
    player_id       BIGINT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    ability_id      VARCHAR(100) NOT NULL,
    is_unlocked     BOOLEAN NOT NULL DEFAULT TRUE,
    hotbar_slot     SMALLINT,  -- NULL = not on hotbar, 0-9 = slot
    is_passive      BOOLEAN NOT NULL DEFAULT FALSE,

    PRIMARY KEY (player_id, ability_id)
);

CREATE INDEX idx_abilities_player ON player_abilities(player_id);

-- ============================================================================
-- 5. Economy - Wallets
-- ============================================================================

CREATE TABLE IF NOT EXISTS wallets (
    player_id       BIGINT PRIMARY KEY REFERENCES players(id) ON DELETE CASCADE,
    gold            BIGINT NOT NULL DEFAULT 0 CHECK (gold >= 0),
    premium_currency INTEGER NOT NULL DEFAULT 0 CHECK (premium_currency >= 0),
    honor_points    INTEGER NOT NULL DEFAULT 0 CHECK (honor_points >= 0),
    event_tokens    JSONB DEFAULT '{}'  -- {"season_01": 150, "lunar_festival": 30}
);

-- ============================================================================
-- 6. Economy - Trading
-- ============================================================================

CREATE TABLE IF NOT EXISTS trades (
    id              BIGSERIAL PRIMARY KEY,
    player1_id      BIGINT NOT NULL REFERENCES players(id),
    player2_id      BIGINT NOT NULL REFERENCES players(id),
    player1_gold    BIGINT NOT NULL DEFAULT 0,
    player2_gold    BIGINT NOT NULL DEFAULT 0,
    player1_items   JSONB DEFAULT '[]',
    player2_items   JSONB DEFAULT '[]',
    player1_confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    player2_confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    status          SMALLINT NOT NULL DEFAULT 0,  -- 0=Pending, 1=Completed, 2=Cancelled
    created_at      TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- ============================================================================
-- 7. Economy - Auction House
-- ============================================================================

CREATE TABLE IF NOT EXISTS auctions (
    id              BIGSERIAL PRIMARY KEY,
    seller_id       BIGINT NOT NULL REFERENCES players(id),
    item_template_id VARCHAR(100) NOT NULL,
    item_instance_id BIGINT,
    quantity        INTEGER NOT NULL DEFAULT 1,

    buyout_price    BIGINT NOT NULL,
    starting_bid    BIGINT NOT NULL DEFAULT 0,
    current_bid     BIGINT DEFAULT 0,
    highest_bidder  BIGINT REFERENCES players(id),

    created_at      TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at      TIMESTAMP WITH TIME ZONE NOT NULL,
    status          SMALLINT NOT NULL DEFAULT 0,  -- 0=Active, 1=Sold, 2=Expired, 3=Cancelled

    tax_rate        SMALLINT NOT NULL DEFAULT 5   -- 5% default tax
);

CREATE INDEX idx_auctions_seller ON auctions(seller_id);
CREATE INDEX idx_auctions_status ON auctions(status);
CREATE INDEX idx_auctions_item ON auctions(item_template_id);
CREATE INDEX idx_auctions_expires ON auctions(expires_at) WHERE status = 0;

-- ============================================================================
-- 8. Economy - Transaction Log (Audit Trail)
-- ============================================================================

CREATE TABLE IF NOT EXISTS transaction_log (
    id              BIGSERIAL PRIMARY KEY,
    tx_type         VARCHAR(20) NOT NULL,  -- 'trade', 'auction', 'vendor', 'craft', 'tax', 'loot'
    from_player     BIGINT REFERENCES players(id),
    to_player       BIGINT REFERENCES players(id),
    gold_amount     BIGINT DEFAULT 0,
    item_id         VARCHAR(100),
    item_quantity   INTEGER DEFAULT 0,
    created_at      TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_tx_log_from ON transaction_log(from_player);
CREATE INDEX idx_tx_log_to ON transaction_log(to_player);
CREATE INDEX idx_tx_log_type ON transaction_log(tx_type);
CREATE INDEX idx_tx_log_time ON transaction_log(created_at);

-- ============================================================================
-- 9. Social - Guilds
-- ============================================================================

CREATE TABLE IF NOT EXISTS guilds (
    id              BIGSERIAL PRIMARY KEY,
    name            VARCHAR(100) UNIQUE NOT NULL,
    tag             VARCHAR(5) UNIQUE NOT NULL,
    leader_id       BIGINT NOT NULL REFERENCES players(id),
    created_at      TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    level           SMALLINT NOT NULL DEFAULT 1,
    experience      BIGINT NOT NULL DEFAULT 0,
    max_members     SMALLINT NOT NULL DEFAULT 20,

    description     TEXT DEFAULT '',
    motd            TEXT DEFAULT '',
    is_recruiting   BOOLEAN NOT NULL DEFAULT TRUE,

    bank_gold       BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_guilds_leader ON guilds(leader_id);

CREATE TABLE IF NOT EXISTS guild_members (
    guild_id        BIGINT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    player_id       BIGINT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    rank            SMALLINT NOT NULL DEFAULT 1,  -- 0=Recruit, 1=Member, 2=Veteran, 3=Officer, 4=Leader
    joined_at       TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    contribution    BIGINT NOT NULL DEFAULT 0,
    note            TEXT DEFAULT '',

    PRIMARY KEY (guild_id, player_id)
);

CREATE INDEX idx_gm_player ON guild_members(player_id);

-- ============================================================================
-- 10. Social - Friends
-- ============================================================================

CREATE TABLE IF NOT EXISTS friendships (
    player_id       BIGINT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    friend_id       BIGINT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    status          SMALLINT NOT NULL DEFAULT 0,  -- 0=Pending, 1=Accepted, 2=Blocked
    created_at      TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    note            TEXT DEFAULT '',

    PRIMARY KEY (player_id, friend_id),
    CONSTRAINT no_self_friend CHECK (player_id != friend_id)
);

-- ============================================================================
-- 11. Social - Mail
-- ============================================================================

CREATE TABLE IF NOT EXISTS mail (
    id              BIGSERIAL PRIMARY KEY,
    sender_id       BIGINT REFERENCES players(id),
    sender_name     VARCHAR(50) NOT NULL,
    recipient_id    BIGINT NOT NULL REFERENCES players(id),

    subject         VARCHAR(200) NOT NULL,
    body            TEXT DEFAULT '',

    gold_attached   BIGINT NOT NULL DEFAULT 0,
    items_attached  JSONB DEFAULT '[]',

    status          SMALLINT NOT NULL DEFAULT 0,  -- 0=Unread, 1=Read, 2=Claimed, 3=Expired
    sent_at         TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at      TIMESTAMP WITH TIME ZONE DEFAULT NOW() + INTERVAL '30 days'
);

CREATE INDEX idx_mail_recipient ON mail(recipient_id, status);

-- ============================================================================
-- 12. Quest Progress
-- ============================================================================

CREATE TABLE IF NOT EXISTS player_quests (
    player_id       BIGINT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    quest_id        VARCHAR(100) NOT NULL,
    status          SMALLINT NOT NULL DEFAULT 0,  -- 0=NotStarted, 1=InProgress, 2=Ready, 3=Completed, 4=Failed
    objectives      JSONB DEFAULT '[]',  -- [{index: 0, current: 5, required: 10, complete: false}]
    started_at      TIMESTAMP WITH TIME ZONE,
    completed_at    TIMESTAMP WITH TIME ZONE,
    times_completed INTEGER NOT NULL DEFAULT 0,

    PRIMARY KEY (player_id, quest_id)
);

CREATE INDEX idx_quests_player ON player_quests(player_id, status);

-- ============================================================================
-- 13. Faction Reputation
-- ============================================================================

CREATE TABLE IF NOT EXISTS player_reputation (
    player_id       BIGINT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    faction_id      VARCHAR(100) NOT NULL,
    reputation      INTEGER NOT NULL DEFAULT 0,  -- -42000 to +42000
    standing        SMALLINT NOT NULL DEFAULT 3,  -- 0=Hated ... 7=Exalted

    PRIMARY KEY (player_id, faction_id)
);

CREATE INDEX idx_reputation_player ON player_reputation(player_id);

-- ============================================================================
-- 14. Season Pass Progress
-- ============================================================================

CREATE TABLE IF NOT EXISTS player_seasons (
    player_id       BIGINT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    season_id       VARCHAR(50) NOT NULL,
    level           SMALLINT NOT NULL DEFAULT 1,
    experience      BIGINT NOT NULL DEFAULT 0,
    has_premium     BOOLEAN NOT NULL DEFAULT FALSE,
    claimed_levels  JSONB DEFAULT '[]',  -- [1, 2, 3, 5] (claimed reward levels)

    PRIMARY KEY (player_id, season_id)
);

-- ============================================================================
-- 15. Achievements
-- ============================================================================

CREATE TABLE IF NOT EXISTS player_achievements (
    player_id       BIGINT NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    achievement_id  VARCHAR(100) NOT NULL,
    is_completed    BOOLEAN NOT NULL DEFAULT FALSE,
    progress        JSONB DEFAULT '[]',  -- [5, 10] (progress per criteria)
    completed_at    TIMESTAMP WITH TIME ZONE,

    PRIMARY KEY (player_id, achievement_id)
);

-- ============================================================================
-- 16. PvP - Shadow Duels
-- ============================================================================

CREATE TABLE IF NOT EXISTS shadow_recordings (
    id              BIGSERIAL PRIMARY KEY,
    player_id       BIGINT NOT NULL REFERENCES players(id),
    player_name     VARCHAR(50) NOT NULL,
    equipment       JSONB NOT NULL,  -- Equipment snapshot
    abilities       JSONB NOT NULL,  -- Ability loadout
    mastery         JSONB NOT NULL,  -- Mastery snapshot
    actions         BYTEA NOT NULL,  -- Binary replay data (compressed)
    recorded_at     TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    pvp_rating      INTEGER NOT NULL DEFAULT 1000
);

CREATE INDEX idx_shadows_player ON shadow_recordings(player_id);
CREATE INDEX idx_shadows_rating ON shadow_recordings(pvp_rating);

CREATE TABLE IF NOT EXISTS duel_results (
    id              BIGSERIAL PRIMARY KEY,
    challenger_id   BIGINT NOT NULL REFERENCES players(id),
    shadow_owner_id BIGINT NOT NULL REFERENCES players(id),
    winner_id       BIGINT NOT NULL REFERENCES players(id),
    rating_change_challenger INTEGER NOT NULL DEFAULT 0,
    rating_change_defender   INTEGER NOT NULL DEFAULT 0,
    duration_ms     INTEGER NOT NULL,
    created_at      TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- ============================================================================
-- 17. Death Echoes (Tower Game unique)
-- ============================================================================

CREATE TABLE IF NOT EXISTS player_echoes (
    id              BIGSERIAL PRIMARY KEY,
    player_id       BIGINT NOT NULL REFERENCES players(id),
    player_name     VARCHAR(50) NOT NULL,
    floor_id        INTEGER NOT NULL,
    pos_x           REAL NOT NULL,
    pos_y           REAL NOT NULL,
    pos_z           REAL NOT NULL,

    cause_of_death  VARCHAR(100) NOT NULL,
    last_action     VARCHAR(100),
    echo_type       VARCHAR(20) NOT NULL DEFAULT 'warning',  -- warning, buff, loot, hint
    echo_message    TEXT,
    echo_strength   REAL NOT NULL DEFAULT 1.0,

    semantic_tags   JSONB DEFAULT '{}',  -- Semantic trace left on floor

    created_at      TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    expires_at      TIMESTAMP WITH TIME ZONE DEFAULT NOW() + INTERVAL '7 days'
);

CREATE INDEX idx_echoes_floor ON player_echoes(floor_id);
CREATE INDEX idx_echoes_expires ON player_echoes(expires_at);

-- ============================================================================
-- 18. Leaderboards (materialized for performance)
-- ============================================================================

CREATE TABLE IF NOT EXISTS leaderboards (
    board_type      SMALLINT NOT NULL,  -- 0=Floor, 1=Mastery, 2=PvP, 3=Wealth, 4=Entropy, 5=Guild
    rank            INTEGER NOT NULL,
    player_id       BIGINT NOT NULL REFERENCES players(id),
    player_name     VARCHAR(50) NOT NULL,
    score           BIGINT NOT NULL,
    extra_data      JSONB DEFAULT '{}',
    updated_at      TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    PRIMARY KEY (board_type, rank)
);

-- ============================================================================
-- Seed Data
-- ============================================================================

-- Create default mastery domains trigger
CREATE OR REPLACE FUNCTION create_default_mastery() RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO mastery (player_id, domain, experience, tier) VALUES
        (NEW.id, 'SwordMastery', 0, 0),
        (NEW.id, 'AxeMastery', 0, 0),
        (NEW.id, 'SpearMastery', 0, 0),
        (NEW.id, 'BowMastery', 0, 0),
        (NEW.id, 'StaffMastery', 0, 0),
        (NEW.id, 'FistMastery', 0, 0),
        (NEW.id, 'DualWieldMastery', 0, 0),
        (NEW.id, 'ParryMastery', 0, 0),
        (NEW.id, 'DodgeMastery', 0, 0),
        (NEW.id, 'CounterMastery', 0, 0),
        (NEW.id, 'ComboMastery', 0, 0),
        (NEW.id, 'PositioningMastery', 0, 0),
        (NEW.id, 'SmithingMastery', 0, 0),
        (NEW.id, 'AlchemyMastery', 0, 0),
        (NEW.id, 'CookingMastery', 0, 0),
        (NEW.id, 'MiningMastery', 0, 0),
        (NEW.id, 'HerbalismMastery', 0, 0),
        (NEW.id, 'LoggingMastery', 0, 0),
        (NEW.id, 'ExplorationMastery', 0, 0),
        (NEW.id, 'CorruptionResistance', 0, 0),
        (NEW.id, 'SocialMastery', 0, 0);

    -- Create default wallet
    INSERT INTO wallets (player_id, gold, premium_currency, honor_points, event_tokens)
    VALUES (NEW.id, 100, 0, 0, '{}');

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_player_created
    AFTER INSERT ON players
    FOR EACH ROW EXECUTE FUNCTION create_default_mastery();
"#;

/// Get all migration SQL statements in order
pub fn get_migrations() -> Vec<(&'static str, &'static str)> {
    vec![
        ("v1_initial_schema", MIGRATION_V1),
    ]
}
