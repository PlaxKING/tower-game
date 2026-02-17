# Tower Game - Complete Entity Model

**Version**: 1.0 (Production-ready)
**Date**: 2026-02-16
**Status**: Design specification for full implementation

---

## Overview

This document defines **ALL** game entities for Tower Game, designed for production deployment with LMDB (templates) + PostgreSQL (player data) + FoundationDB (distributed state, future).

---

## Entity Categories

### 1. **Player & Character** (PostgreSQL)
### 2. **Monsters & NPCs** (LMDB templates + PostgreSQL instances)
### 3. **Items & Equipment** (LMDB templates + PostgreSQL instances)
### 4. **Abilities & Skills** (LMDB templates + PostgreSQL progress)
### 5. **World & Floors** (LMDB cached + procedural)
### 6. **Economy** (PostgreSQL transactions)
### 7. **Social Systems** (PostgreSQL relationships)
### 8. **Quests & Events** (LMDB templates + PostgreSQL progress)
### 9. **Factions** (LMDB templates + PostgreSQL reputation)
### 10. **Seasonal Content** (PostgreSQL progress)

---

## 1. Player & Character Entities

### **1.1 Player Core**
```rust
pub struct Player {
    pub id: u64,                    // Unique player ID
    pub username: String,           // Unique username (3-20 chars)
    pub created_at: i64,            // Unix timestamp
    pub last_login: i64,            // Unix timestamp
    pub playtime_seconds: u64,      // Total playtime

    // Base stats (set ONCE at creation, never changed - CLAUDE.md)
    pub base_str: u8,               // 1-10, total 20 points distributed
    pub base_dex: u8,               // 1-10
    pub base_int: u8,               // 1-10
    pub base_vit: u8,               // 1-10
    pub base_luk: u8,               // 1-10 (luck for loot)

    // Current state
    pub health: f32,
    pub max_health: f32,            // Calculated from base_vit + equipment
    pub kinetic_energy: f32,        // Combat resource (melee)
    pub thermal_energy: f32,        // Combat resource (ranged/magic)
    pub semantic_energy: f32,       // Combat resource (special abilities)

    // Position
    pub floor_id: u32,              // Current floor (1-1000+)
    pub position: Vec3,             // World position
    pub rotation: Rotation,         // Camera/character rotation

    // State flags
    pub is_alive: bool,
    pub is_in_combat: bool,
    pub is_trading: bool,
    pub respawn_floor: u32,         // Respawn point (last safe floor)
}
```

**Storage**: PostgreSQL `players` table

### **1.2 Mastery Progress** (21 domains)
```rust
pub struct MasteryProgress {
    pub player_id: u64,
    pub domain: MasteryDomain,      // One of 21 domains
    pub experience: u64,            // XP in this domain
    pub tier: MasteryTier,          // 1-6: Novice → Grandmaster
    pub specialization: Option<Specialization>, // Unlocked at Expert tier
}

pub enum MasteryTier {
    Novice = 1,       // 0-1000 XP
    Apprentice = 2,   // 1000-5000 XP
    Adept = 3,        // 5000-20000 XP
    Expert = 4,       // 20000-100000 XP (unlock specialization)
    Master = 5,       // 100000-500000 XP
    Grandmaster = 6,  // 500000+ XP
}

pub struct Specialization {
    pub domain: MasteryDomain,
    pub branch: SpecializationBranch, // e.g., SwordMastery → Duelist/Defender/Berserker
    pub passive_bonuses: Vec<PassiveBonus>,
}
```

**Storage**: PostgreSQL `mastery` table (player_id, domain, experience, tier)

### **1.3 Inventory**
```rust
pub struct InventorySlot {
    pub id: u64,                    // Unique slot ID
    pub player_id: u64,
    pub item_template_id: String,   // Reference to ItemTemplate in LMDB
    pub quantity: u32,              // Stack size (1 for unique items)
    pub slot_type: SlotType,        // Bag, Equipment, Bank
    pub slot_index: u16,            // Position in bag/equipment

    // Instance data (for non-stackable items)
    pub item_instance: Option<ItemInstance>,
}

pub enum SlotType {
    Bag,            // Inventory bag (100 slots default)
    Equipment,      // Equipped gear (16 slots: head, chest, hands, legs, feet, weapon, etc.)
    Bank,           // Bank storage (200 slots)
    GuildBank,      // Guild shared storage
}

pub struct ItemInstance {
    pub instance_id: u64,           // Unique instance for this specific item
    pub durability: f32,            // Current/max (1.0 = 100%)
    pub sockets: Vec<SocketSlot>,   // Gems/runes inserted
    pub enchantments: Vec<Enchantment>,
    pub custom_effects: Vec<ItemEffect>, // Random rolls
}
```

**Storage**: PostgreSQL `inventory` table

### **1.4 Equipment Loadout**
```rust
pub struct EquipmentSlot {
    Head = 0,
    Chest = 1,
    Hands = 2,
    Legs = 3,
    Feet = 4,
    MainHand = 5,       // Weapon
    OffHand = 6,        // Shield, dual wield, or empty
    Accessory1 = 7,     // Ring
    Accessory2 = 8,     // Ring
    Accessory3 = 9,     // Amulet
    Accessory4 = 10,    // Earring
    Accessory5 = 11,    // Bracelet
    Artifact1 = 12,     // Special endgame items
    Artifact2 = 13,
    Artifact3 = 14,
    Cosmetic = 15,      // Transmog/skin
}
```

**Storage**: Part of `inventory` table with `slot_type = Equipment`

---

## 2. Monsters & NPCs

### **2.1 Monster Template** (Static definition)
```rust
pub struct MonsterTemplate {
    pub id: String,                 // "goblin_scout", "fire_elemental_boss"
    pub name: String,               // "Goblin Scout"
    pub monster_type: MonsterType,  // Normal, Elite, Rare, Boss, WorldBoss
    pub tier: u8,                   // 1-10 difficulty tier

    // Base stats (scaled by floor)
    pub base_health: f32,
    pub base_damage: f32,
    pub base_defense: f32,
    pub base_speed: f32,

    // AI behavior
    pub ai_behavior: AIBehavior,    // Passive, Aggressive, Patrol, Boss
    pub aggro_range: f32,
    pub leash_range: f32,           // Max distance from spawn before reset

    // Combat
    pub abilities: Vec<String>,     // List of ability IDs
    pub attack_pattern: AttackPattern, // Melee, Ranged, Mixed

    // Semantic tags (base, will be blended with floor tags)
    pub semantic_tags: SemanticTags,

    // Loot
    pub loot_table_id: String,      // Reference to LootTable
    pub gold_min: u32,
    pub gold_max: u32,

    // Visuals
    pub model_id: String,           // 3D model reference
    pub scale: f32,                 // Size multiplier
}

pub enum MonsterType {
    Normal,         // Regular mobs
    Elite,          // 3x stats, better loot
    Rare,           // Rare spawn, unique loot
    MiniBoss,       // Room boss
    FloorBoss,      // End of floor boss
    WorldBoss,      // Special event boss
}

pub enum AIBehavior {
    Passive,        // Doesn't attack unless hit
    Defensive,      // Attacks if player approaches
    Aggressive,     // Attacks on sight
    Patrol,         // Walks a path, aggressive
    Boss,           // Special scripted behavior
}
```

**Storage**: LMDB `monster_templates` database

### **2.2 Monster Instance** (Active in world)
```rust
pub struct MonsterInstance {
    pub instance_id: u64,           // Unique runtime ID
    pub template_id: String,        // Reference to MonsterTemplate
    pub floor_id: u32,              // Floor where spawned

    // Current state
    pub position: Vec3,
    pub rotation: Rotation,
    pub velocity: Velocity,
    pub health: f32,
    pub max_health: f32,            // Scaled by floor + bonuses

    // Combat state
    pub target_player: Option<u64>, // Currently attacking
    pub is_in_combat: bool,
    pub last_damaged_by: Option<u64>, // For loot rights

    // AI state
    pub ai_state: AIState,
    pub spawn_position: Vec3,       // For leashing
    pub patrol_waypoints: Vec<Vec3>,

    // Modifiers
    pub buffs: Vec<Buff>,
    pub debuffs: Vec<Debuff>,

    // Semantic tags (floor 70% + template 30%)
    pub semantic_tags: SemanticTags,
}

pub enum AIState {
    Idle,
    Patrol,
    Chasing,
    Attacking,
    Fleeing,
    Returning,      // Leashing back to spawn
    Dead,
}
```

**Storage**: PostgreSQL `monster_instances` (ephemeral, cleared on floor reset)

---

## 3. Items & Equipment

### **3.1 Item Template** (Static definition)
```rust
pub struct ItemTemplate {
    pub id: String,                 // "iron_sword", "health_potion_minor"
    pub name: String,               // "Iron Sword"
    pub description: String,
    pub item_type: ItemType,
    pub rarity: Rarity,
    pub tier: u8,                   // 1-10 item level

    // Requirements
    pub required_mastery: Option<(MasteryDomain, MasteryTier)>,
    pub required_level: Option<u8>, // If using levels (Tower Game doesn't)

    // Stats (MINIMAL - Tower Game focuses on effects)
    pub base_damage: Option<f32>,   // For weapons
    pub base_defense: Option<f32>,  // For armor
    pub base_speed: Option<f32>,    // Attack speed modifier

    // Effects (CORE SYSTEM - trigger→action)
    pub effects: Vec<ItemEffect>,

    // Sockets
    pub socket_count: u8,           // 0-3 sockets for gems/runes
    pub socket_types: Vec<SocketType>, // Color restrictions

    // Set bonus
    pub set_id: Option<String>,     // "inferno_set"

    // Semantic tags
    pub semantic_tags: SemanticTags, // For synergy calculation

    // Economy
    pub vendor_value: u32,          // Sell price
    pub max_stack: u32,             // 1 for equipment, 99 for consumables

    // Crafting
    pub is_craftable: bool,
    pub recipe_id: Option<String>,

    // Visuals
    pub icon_id: String,
    pub model_id: Option<String>,   // For 3D equipment
}

pub enum ItemType {
    // Weapons (7 types matching mastery domains)
    Sword,
    Axe,
    Spear,
    Bow,
    Staff,
    Fist,           // Knuckles/gauntlets
    DualWield,      // Paired weapons

    // Armor
    Helmet,
    Chest,
    Gloves,
    Pants,
    Boots,
    Shield,

    // Accessories
    Ring,
    Amulet,
    Earring,
    Bracelet,

    // Consumables
    Potion,         // Health/mana/energy restore
    Food,           // Temporary buffs
    Scroll,         // One-time effect (teleport, buff)

    // Materials
    Ore,            // Mining materials
    Herb,           // Herbalism materials
    Wood,           // Logging materials
    Essence,        // Monster drops for crafting
    Reagent,        // Alchemy ingredients

    // Quest items
    QuestItem,
    KeyItem,

    // Special
    Artifact,       // Endgame unique items
    Cosmetic,       // Transmog skins
}

pub enum Rarity {
    Common = 0,      // White
    Uncommon = 1,    // Green
    Rare = 2,        // Blue
    Epic = 3,        // Purple
    Legendary = 4,   // Orange
    Mythic = 5,      // Red
    Ancient = 6,     // Gold (ultra rare)
}
```

**Storage**: LMDB `item_templates` database

### **3.2 Item Effect System** (Trigger→Action)
```rust
pub struct ItemEffect {
    pub trigger: EffectTrigger,
    pub action: EffectAction,
    pub chance: f32,                // 0.0-1.0 (1.0 = 100%)
    pub cooldown_ms: u32,           // Internal cooldown
}

pub enum EffectTrigger {
    OnHit,                          // When you hit enemy
    OnCrit,                         // On critical hit
    OnParry,                        // When you parry
    OnDodge,                        // When you dodge
    OnKill,                         // When you kill enemy
    OnDamaged,                      // When you take damage
    OnLowHealth(f32),               // When health < threshold
    OnAbilityUse(String),           // When specific ability used
    Passive,                        // Always active
    OnCombatStart,
    OnCombatEnd,
}

pub enum EffectAction {
    DealDamage {
        damage_type: DamageType,
        amount: f32,
        is_percent: bool,           // % of base damage or flat
    },
    Heal {
        amount: f32,
        is_percent: bool,
    },
    ApplyBuff {
        buff_id: String,
        duration_ms: u32,
    },
    ApplyDebuff {
        debuff_id: String,
        duration_ms: u32,
    },
    RestoreResource {
        resource: ResourceType,
        amount: f32,
    },
    IncreaseStats {
        stat: StatType,
        amount: f32,
        duration_ms: Option<u32>,   // None = permanent (while equipped)
    },
    Lifesteal {
        percent: f32,               // % of damage dealt
    },
    Shield {
        amount: f32,
        duration_ms: u32,
    },
    Teleport {
        distance: f32,
        direction: TeleportDirection,
    },
    SummonCreature {
        creature_id: String,
        duration_ms: u32,
    },
}

pub enum DamageType {
    Physical,
    Fire,
    Ice,
    Lightning,
    Poison,
    Holy,
    Dark,
    Semantic,       // Special damage type using semantic tags
}

pub enum ResourceType {
    Health,
    Kinetic,
    Thermal,
    Semantic,
}

pub enum StatType {
    Damage,
    Defense,
    AttackSpeed,
    MovementSpeed,
    CritChance,
    CritDamage,
    Lifesteal,
}
```

### **3.3 Set Bonuses**
```rust
pub struct ItemSet {
    pub id: String,                 // "inferno_set"
    pub name: String,               // "Inferno's Wrath"
    pub items: Vec<String>,         // List of item IDs in set
    pub bonuses: Vec<SetBonus>,
}

pub struct SetBonus {
    pub pieces_required: u8,        // 2, 3, 4, or 5 pieces
    pub effects: Vec<ItemEffect>,   // Activate when equipped
    pub description: String,
}
```

**Storage**: LMDB `item_sets` database

### **3.4 Socket System**
```rust
pub struct Socket {
    pub socket_type: SocketType,
    pub gem: Option<GemTemplate>,
}

pub enum SocketType {
    Red,        // Strength gems
    Blue,       // Intelligence gems
    Yellow,     // Dexterity gems
    Green,      // Vitality gems
    Prismatic,  // Any gem
}

pub struct GemTemplate {
    pub id: String,
    pub name: String,
    pub tier: u8,               // 1-5
    pub socket_type: SocketType,
    pub bonus: ItemEffect,      // What it adds when socketed
}
```

**Storage**: LMDB `gem_templates` database

---

## 4. Abilities & Skills

### **4.1 Ability Template**
```rust
pub struct AbilityTemplate {
    pub id: String,                 // "fireball", "parry_stance"
    pub name: String,
    pub description: String,
    pub ability_type: AbilityType,

    // Requirements
    pub required_mastery: (MasteryDomain, MasteryTier),
    pub required_weapon: Option<ItemType>,

    // Resource costs
    pub kinetic_cost: f32,
    pub thermal_cost: f32,
    pub semantic_cost: f32,

    // Timing
    pub cooldown_ms: u32,
    pub cast_time_ms: u32,
    pub global_cooldown_ms: u32,

    // Charges
    pub max_charges: u8,            // 0 = unlimited, 1+ = charge system
    pub charge_recharge_ms: u32,

    // Effects
    pub effects: Vec<AbilityEffect>,

    // Targeting
    pub target_type: TargetType,
    pub range: f32,
    pub aoe_radius: Option<f32>,

    // Semantic tags
    pub semantic_tags: SemanticTags,

    // Visuals
    pub animation_id: String,
    pub vfx_id: String,
    pub sfx_id: String,
}

pub enum AbilityType {
    Active,         // Manually activated
    Passive,        // Always active
    Toggle,         // On/off stance
    Channeled,      // Hold to cast
}

pub struct AbilityEffect {
    pub effect_type: EffectType,
    pub value: f32,
    pub duration_ms: Option<u32>,
    pub scaling: Option<AbilityScaling>,
}

pub enum EffectType {
    Damage(DamageType),
    Heal,
    Buff(String),
    Debuff(String),
    Teleport,
    Knockback,
    Stun,
    Silence,
    Root,
    Slow(f32),      // % reduction
}

pub struct AbilityScaling {
    pub stat: StatType,
    pub ratio: f32,             // Damage = base + (stat × ratio)
}

pub enum TargetType {
    Self_,
    SingleTarget,
    AOE,
    Cone,
    Line,
    Ground,         // Click location
}
```

**Storage**: LMDB `ability_templates` database

### **4.2 Player Abilities** (Unlocked/equipped)
```rust
pub struct PlayerAbilities {
    pub player_id: u64,
    pub unlocked: Vec<String>,      // All unlocked ability IDs
    pub hotbar: [Option<String>; 10], // 10 hotbar slots (0-9)
    pub passive_abilities: Vec<String>, // Auto-active passives
}
```

**Storage**: PostgreSQL `player_abilities` table

---

## 5. World & Floors

### **5.1 Floor (Procedural)**
Already implemented as `ChunkData` with semantic tags ✅

### **5.2 Floor Modifiers**
```rust
pub struct FloorModifiers {
    pub floor_id: u32,
    pub modifiers: Vec<FloorModifier>,
}

pub enum FloorModifier {
    BreathPhase(BreathPhase),       // Current Breath of Tower phase
    Event(EventType),               // Active event
    Corruption(f32),                // Corruption level (0.0-1.0)
    DangerLevel(u8),                // 1-10
}

pub enum BreathPhase {
    Inhale,     // 6h - Passive monsters, +20% recovery
    Hold,       // 4h - Monster swarms, +30% damage, -40% recovery
    Exhale,     // 6h - Aggressive, -30% recovery
    Pause,      // 2h - Reality cracks, portals to hidden floors
}

pub enum EventType {
    TreasureFloor,      // Extra loot
    BossRush,           // Multiple bosses
    MonsterHorde,       // Waves of enemies
    MerchantVisit,      // Rare trader appears
    Corruption Surge,   // Increased corruption, harder enemies
}
```

**Storage**: LMDB `floor_modifiers` (cached, expires after 18h cycle)

---

## 6. Economy

### **6.1 Wallet**
```rust
pub struct Wallet {
    pub player_id: u64,
    pub gold: u64,                  // Primary currency
    pub premium_currency: u32,      // Donatable currency (optional)
    pub event_tokens: HashMap<String, u32>, // Seasonal tokens
    pub honor_points: u32,          // PvP currency
}
```

**Storage**: PostgreSQL `wallets` table

### **6.2 Trade**
```rust
pub struct Trade {
    pub id: u64,
    pub player1_id: u64,
    pub player2_id: u64,
    pub player1_items: Vec<TradeItem>,
    pub player2_items: Vec<TradeItem>,
    pub player1_gold: u64,
    pub player2_gold: u64,
    pub player1_confirmed: bool,
    pub player2_confirmed: bool,
    pub status: TradeStatus,
    pub created_at: i64,
}

pub struct TradeItem {
    pub item_template_id: String,
    pub quantity: u32,
    pub item_instance_id: Option<u64>, // For unique items
}

pub enum TradeStatus {
    Pending,
    Completed,
    Cancelled,
}
```

**Storage**: PostgreSQL `trades` table (audit log)

### **6.3 Auction House**
```rust
pub struct AuctionListing {
    pub id: u64,
    pub seller_id: u64,
    pub item_template_id: String,
    pub item_instance_id: Option<u64>,
    pub quantity: u32,
    pub buyout_price: u64,
    pub bid_price: Option<u64>,         // Starting bid (optional)
    pub current_bid: Option<u64>,
    pub highest_bidder: Option<u64>,
    pub created_at: i64,
    pub expires_at: i64,
    pub status: AuctionStatus,
}

pub enum AuctionStatus {
    Active,
    Sold,
    Expired,
    Cancelled,
}
```

**Storage**: PostgreSQL `auctions` table

### **6.4 Crafting Recipe**
```rust
pub struct CraftingRecipe {
    pub id: String,
    pub name: String,
    pub profession: MasteryDomain,  // Smithing, Alchemy, Cooking
    pub required_tier: MasteryTier,

    // Ingredients
    pub ingredients: Vec<RecipeIngredient>,

    // Results
    pub result_item: String,        // ItemTemplate ID
    pub result_quantity: u32,
    pub success_rate: f32,          // 0.0-1.0 (based on mastery)

    // Requirements
    pub crafting_station: Option<CraftingStation>,
    pub gold_cost: u32,

    // Learning
    pub is_learned_by_default: bool,
    pub recipe_item_id: Option<String>, // Recipe scroll to learn
}

pub struct RecipeIngredient {
    pub item_template_id: String,
    pub quantity: u32,
}

pub enum CraftingStation {
    Forge,          // Smithing
    AlchemyLab,     // Alchemy
    Kitchen,        // Cooking
    Workbench,      // General crafting
}
```

**Storage**: LMDB `crafting_recipes` database

---

## 7. Social Systems

### **7.1 Guild**
```rust
pub struct Guild {
    pub id: u64,
    pub name: String,               // Unique guild name
    pub tag: String,                // 3-5 char tag [TAG]
    pub leader_id: u64,
    pub created_at: i64,
    pub level: u8,                  // 1-10 guild level
    pub experience: u64,            // Guild XP
    pub max_members: u16,           // Scales with level
    pub description: String,
    pub is_recruiting: bool,
}

pub struct GuildMember {
    pub guild_id: u64,
    pub player_id: u64,
    pub rank: GuildRank,
    pub joined_at: i64,
    pub contribution_points: u64,   // For guild shop
}

pub enum GuildRank {
    Leader,
    Officer,
    Veteran,
    Member,
    Recruit,
}
```

**Storage**: PostgreSQL `guilds`, `guild_members` tables

### **7.2 Friends**
```rust
pub struct Friendship {
    pub player_id: u64,
    pub friend_id: u64,
    pub status: FriendshipStatus,
    pub created_at: i64,
}

pub enum FriendshipStatus {
    Pending,        // Friend request sent
    Accepted,       // Friends
    Blocked,        // Blocked user
}
```

**Storage**: PostgreSQL `friendships` table

### **7.3 Party**
```rust
pub struct Party {
    pub id: u64,
    pub leader_id: u64,
    pub members: Vec<u64>,          // Player IDs
    pub max_size: u8,               // 4-8 depending on content
    pub loot_mode: LootMode,
    pub created_at: i64,
}

pub enum LootMode {
    FreeForAll,     // Anyone can loot
    RoundRobin,     // Takes turns
    MasterLooter,   // Leader distributes
    NeedGreed,      // Roll system
}
```

**Storage**: PostgreSQL `parties` table (ephemeral, cleared on disband)

---

## 8. Quests & Events

### **8.1 Quest Template**
```rust
pub struct QuestTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub quest_type: QuestType,
    pub quest_tier: QuestTier,

    // Requirements
    pub required_floor_min: Option<u32>,
    pub required_mastery: Option<(MasteryDomain, MasteryTier)>,
    pub required_quest: Option<String>, // Prerequisite quest

    // Objectives
    pub objectives: Vec<QuestObjective>,

    // Rewards
    pub rewards: QuestRewards,

    // Repeatability
    pub is_repeatable: bool,
    pub reset_interval: Option<QuestResetInterval>,
}

pub enum QuestType {
    MainStory,
    Side,
    Daily,
    Weekly,
    Seasonal,
    Chain,          // Part of quest chain
}

pub enum QuestTier {
    Easy,
    Normal,
    Hard,
    Elite,
    Legendary,
}

pub struct QuestObjective {
    pub objective_type: ObjectiveType,
    pub target: String,         // Monster ID, item ID, floor ID
    pub required_count: u32,
    pub current_count: u32,     // Progress (stored in PlayerQuest)
}

pub enum ObjectiveType {
    KillMonster,
    CollectItem,
    ReachFloor,
    CraftItem,
    TalkToNPC,
    Escort,
    Discover,       // Discover location
    UseAbility,
}

pub struct QuestRewards {
    pub gold: u64,
    pub mastery_exp: HashMap<MasteryDomain, u64>,
    pub items: Vec<(String, u32)>,      // (ItemTemplate ID, quantity)
    pub unlock_ability: Option<String>,
    pub unlock_recipe: Option<String>,
}

pub enum QuestResetInterval {
    Daily,
    Weekly,
    Monthly,
    Never,
}
```

**Storage**: LMDB `quest_templates` database

### **8.2 Player Quest Progress**
```rust
pub struct PlayerQuest {
    pub player_id: u64,
    pub quest_id: String,
    pub status: QuestStatus,
    pub objectives: Vec<ObjectiveProgress>,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub times_completed: u32,       // For repeatable quests
}

pub struct ObjectiveProgress {
    pub objective_index: u8,
    pub current_count: u32,
    pub is_complete: bool,
}

pub enum QuestStatus {
    NotStarted,
    InProgress,
    ReadyToComplete,    // All objectives done, need to turn in
    Completed,
    Failed,
}
```

**Storage**: PostgreSQL `player_quests` table

---

## 9. Factions

### **9.1 Faction Template**
```rust
pub struct FactionTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub alignment: FactionAlignment,

    // Relations (4-component system from CLAUDE.md)
    pub relations: HashMap<String, FactionRelation>,
}

pub enum FactionAlignment {
    Order,
    Chaos,
    Neutral,
}

pub struct FactionRelation {
    pub target_faction: String,
    pub political: i8,      // -100 to +100
    pub economic: i8,
    pub military: i8,
    pub cultural: i8,
}
```

**Storage**: LMDB `faction_templates` database

### **9.2 Player Faction Reputation**
```rust
pub struct PlayerReputation {
    pub player_id: u64,
    pub faction_id: String,
    pub reputation: i32,        // -42000 to +42000
    pub standing: ReputationStanding,
}

pub enum ReputationStanding {
    Hated,          // -42000 to -21000
    Hostile,        // -21000 to -3000
    Unfriendly,     // -3000 to -1
    Neutral,        // 0 to 2999
    Friendly,       // 3000 to 8999
    Honored,        // 9000 to 20999
    Revered,        // 21000 to 41999
    Exalted,        // 42000
}
```

**Storage**: PostgreSQL `player_reputation` table

---

## 10. Seasonal Content

### **10.1 Season Pass**
```rust
pub struct SeasonPass {
    pub id: String,
    pub name: String,
    pub season_number: u16,
    pub start_date: i64,
    pub end_date: i64,
    pub max_level: u8,          // e.g., 50 levels
    pub rewards: Vec<SeasonReward>,
}

pub struct SeasonReward {
    pub level: u8,
    pub free_reward: Option<String>,    // ItemTemplate ID
    pub premium_reward: Option<String>, // Requires pass purchase
}

pub struct PlayerSeasonProgress {
    pub player_id: u64,
    pub season_id: String,
    pub level: u8,
    pub experience: u64,
    pub has_premium: bool,
    pub claimed_rewards: Vec<u8>,   // List of claimed reward levels
}
```

**Storage**: LMDB `season_passes`, PostgreSQL `player_season_progress`

---

## Summary

### **Total Entity Count**:
- **Core Player**: 4 entities (Player, Mastery, Inventory, Equipment)
- **Monsters**: 2 entities (Template, Instance)
- **Items**: 5 entities (Template, Effects, Sets, Sockets, Gems)
- **Abilities**: 2 entities (Template, Player unlocked)
- **World**: 2 entities (Floors, Modifiers)
- **Economy**: 4 entities (Wallet, Trade, Auction, Recipes)
- **Social**: 3 entities (Guild, Friends, Party)
- **Quests**: 2 entities (Template, Player progress)
- **Factions**: 2 entities (Template, Player reputation)
- **Seasonal**: 2 entities (Season pass, Player progress)

**Total**: 28 entity types

### **Storage Distribution**:
- **LMDB** (10 templates): Monsters, Items, Sets, Gems, Abilities, Recipes, Quests, Factions, Season passes, Floor modifiers
- **PostgreSQL** (18 tables): Players, Mastery, Inventory, Abilities unlocked, Monster instances, Wallets, Trades, Auctions, Guilds, Members, Friends, Parties, Player quests, Reputation, Season progress, etc.

---

**Next Step**: Implement Protobuf schemas for all entities →
