# Tower Game - FFI API Reference
# Rust Procedural Core → Unreal Engine 5

**Version**: 0.5.0
**Last Updated**: Session 22 (2026-02-14)
**Total Exports**: 92 C-ABI functions

---

## Table of Contents

1. [Core](#1-core)
2. [Floor Generation](#2-floor-generation)
3. [Monsters](#3-monsters)
4. [Combat](#4-combat)
5. [Loot](#5-loot)
6. [World & Breath Cycle](#6-world--breath-cycle)
7. [Replication](#7-replication)
8. [Events](#8-events)
9. [Mastery](#9-mastery)
10. [Specialization](#10-specialization)
11. [Abilities](#11-abilities)
12. [Sockets](#12-sockets)
13. [Cosmetics](#13-cosmetics)
14. [Tutorial](#14-tutorial)
15. [Achievements](#15-achievements)
16. [Seasons](#16-seasons)
17. [Social](#17-social)
18. [Mutators](#18-mutators)
19. [Game Flow](#19-game-flow)
20. [Save Migration](#20-save-migration)
21. [Logging](#21-logging)
22. [Replay System](#22-replay-system)
23. [Tower Map](#23-tower-map)

---

## General Conventions

### Memory Management
- **All `*mut c_char` return values** must be freed using `free_string()`.
- **Never** free the same pointer twice.
- Null pointers (`nullptr`) indicate errors.

### JSON Encoding
- All data is serialized as JSON strings.
- Input: Pass JSON as `const char*` (UTF-8).
- Output: Returns JSON as `char*` (must free).

### Error Handling
- Functions return `null` on error (invalid JSON, missing data, etc.).
- Check for `null` before using returned pointers.

---

## 1. Core

### 1.1. `get_version`

**Signature**:
```c
char* get_version();
```

**Description**: Returns the version string of the procedural core library.

**Returns**: `"0.5.0"` (must free)

**Example**:
```cpp
char* version = get_version();
UE_LOG(LogTemp, Log, TEXT("Core Version: %s"), UTF8_TO_TCHAR(version));
free_string(version);
```

---

### 1.2. `free_string`

**Signature**:
```c
void free_string(char* ptr);
```

**Description**: Frees a string allocated by the Rust library. Call this for every `char*` returned by other FFI functions.

**Parameters**:
- `ptr`: Pointer to free (can be null)

**Example**:
```cpp
char* data = generate_floor(42, 1);
// ... use data ...
free_string(data);
```

---

## 2. Floor Generation

### 2.1. `generate_floor`

**Signature**:
```c
char* generate_floor(uint64_t seed, uint32_t floor_id);
```

**Description**: Generates a floor specification (ID, tier, hash, biome tags).

**Parameters**:
- `seed`: Tower seed (deterministic RNG)
- `floor_id`: Floor number (1-1000)

**Returns**: JSON `FloorResponse`:
```json
{
  "floor_id": 1,
  "tier": "Iron",
  "hash": 12345678901234,
  "biome_tags": [
    ["fire", 0.7],
    ["exploration", 0.9]
  ]
}
```

**Example**:
```cpp
char* floor_json = generate_floor(42, 10);
// Parse JSON, extract tier, hash
free_string(floor_json);
```

---

### 2.2. `generate_floor_layout`

**Signature**:
```c
char* generate_floor_layout(uint64_t seed, uint32_t floor_id);
```

**Description**: Generates WFC-based floor layout (tiles, rooms, spawn/exit points).

**Parameters**:
- `seed`: Tower seed
- `floor_id`: Floor number

**Returns**: JSON `FloorLayoutResponse`:
```json
{
  "width": 50,
  "height": 50,
  "tiles": [[0,1,2,...], ...],
  "rooms": [
    {"x": 10, "y": 10, "width": 5, "height": 5, "room_type": "Combat"}
  ],
  "spawn_points": [[5, 5], [10, 10]],
  "exit_point": [45, 45]
}
```

**Tile Types** (u8):
- `0`: Empty
- `1`: Floor
- `2`: Wall
- `3`: Door
- `4`: Chest

---

### 2.3. `get_floor_hash`

**Signature**:
```c
uint64_t get_floor_hash(uint64_t seed, uint32_t floor_id);
```

**Description**: Returns deterministic hash for a floor.

**Returns**: u64 hash value

---

### 2.4. `get_floor_tier`

**Signature**:
```c
uint32_t get_floor_tier(uint32_t floor_id);
```

**Description**: Returns tier ID for a floor.

**Returns**:
- `0`: Iron (floors 1-100)
- `1`: Bronze (floors 101-200)
- `2`: Silver (floors 201-400)
- `3`: Gold (floors 401-700)
- `4`: Platinum (floors 701-1000)

---

## 3. Monsters

### 3.1. `generate_monster`

**Signature**:
```c
char* generate_monster(uint64_t hash, uint32_t floor_level);
```

**Description**: Generates a single monster template from grammar.

**Parameters**:
- `hash`: Seed hash for determinism
- `floor_level`: Monster level scaling

**Returns**: JSON `MonsterInfo`:
```json
{
  "name": "Corrupted Flame Sentinel",
  "size": "Medium",
  "element": "Fire",
  "corruption": "Corrupted",
  "behavior": "Aggressive",
  "base_level": 10,
  "max_hp": 500.0,
  "damage": 25.0,
  "speed": 3.5,
  "armor": 10.0,
  "detection_range": 15.0,
  "xp_reward": 100,
  "semantic_tags": [["fire", 0.8], ["corruption", 0.5]]
}
```

---

### 3.2. `generate_floor_monsters`

**Signature**:
```c
char* generate_floor_monsters(uint64_t seed, uint32_t floor_id, uint32_t count);
```

**Description**: Generates multiple monsters for a floor.

**Returns**: JSON array of `MonsterInfo`

---

## 4. Combat

### 4.1. `get_angle_multiplier`

**Signature**:
```c
float get_angle_multiplier(uint32_t angle_id);
```

**Description**: Returns damage multiplier for attack angle.

**Parameters**:
- `angle_id`: `0` = Front (1.0x), `1` = Side (1.2x), `2` = Back (1.5x)

**Returns**: float multiplier

---

### 4.2. `calculate_combat`

**Signature**:
```c
char* calculate_combat(const char* request_json);
```

**Description**: Calculates combat damage with all bonuses.

**Input**: JSON `CombatCalcRequest`:
```json
{
  "base_damage": 100.0,
  "angle_id": 2,
  "combo_step": 3,
  "attacker_tags_json": "{\"tags\":[[\"fire\",0.9]]}",
  "defender_tags_json": "{\"tags\":[[\"ice\",0.8]]}"
}
```

**Returns**: JSON `CombatCalcResult`:
```json
{
  "final_damage": 195.0,
  "angle_multiplier": 1.5,
  "semantic_bonus": 0.3,
  "is_synergy": true
}
```

---

### 4.3. `semantic_similarity`

**Signature**:
```c
float semantic_similarity(const char* tags_a_json, const char* tags_b_json);
```

**Description**: Computes cosine similarity between two semantic tag vectors.

**Returns**: float `[0.0, 1.0]` (1.0 = identical)

---

## 5. Loot

### 5.1. `generate_loot`

**Signature**:
```c
char* generate_loot(
    uint64_t seed,
    uint32_t floor_level,
    const char* monster_tags_json,
    uint32_t count
);
```

**Description**: Generates loot items with semantic matching.

**Returns**: JSON array of `LootInfo`:
```json
[
  {
    "name": "Flame Shard",
    "category": "Material",
    "rarity": "Common",
    "quantity": 3,
    "semantic_tags": [["fire", 0.7]]
  }
]
```

---

## 6. World & Breath Cycle

### 6.1. `get_breath_state`

**Signature**:
```c
char* get_breath_state(float elapsed_seconds);
```

**Description**: Returns current Breath of the Tower cycle state.

**Parameters**:
- `elapsed_seconds`: Game time since session start

**Returns**: JSON `BreathState`:
```json
{
  "phase": "Exhale",
  "phase_progress": 0.45,
  "monster_spawn_mult": 1.2,
  "resource_mult": 0.8,
  "semantic_intensity": 1.5
}
```

**Phases**:
- `"Inhale"`: Calm, fewer monsters, more resources
- `"Hold"`: Transition, balanced
- `"Exhale"`: Intense, more monsters, fewer resources

---

## 7. Replication

### 7.1. `record_delta`

**Signature**:
```c
char* record_delta(
    const char* log_json,
    uint32_t tick,
    uint32_t delta_type_id,
    const char* payload_json
);
```

**Description**: Records a game state mutation to the delta log.

**Parameters**:
- `log_json`: Current `DeltaLog` (JSON)
- `tick`: Game tick number
- `delta_type_id`: `0` = FloorGenerated, `1` = MonsterSpawned, `2` = DamageTaken, `3` = ItemLooted
- `payload_json`: Event-specific JSON

**Returns**: Updated `DeltaLog` JSON

---

### 7.2. `create_floor_snapshot`

**Signature**:
```c
char* create_floor_snapshot(
    uint64_t seed,
    uint32_t floor_id,
    const char* delta_log_json
);
```

**Description**: Creates a snapshot of floor state from seed + deltas.

**Returns**: JSON `FloorSnapshot`

---

## 8. Events

### 8.1. `evaluate_event_trigger`

**Signature**:
```c
char* evaluate_event_trigger(
    uint32_t trigger_type_id,
    const char* context_json
);
```

**Description**: Evaluates if a procedural event should trigger.

**Parameters**:
- `trigger_type_id`: `0` = OnKill, `1` = OnEnter, `2` = OnInteract, `3` = OnTimer
- `context_json`: `TriggerContext` (current HP, floor, time, etc.)

**Returns**: JSON array of triggered events

---

## 9. Mastery

### 9.1. `mastery_create_profile`

**Signature**:
```c
char* mastery_create_profile();
```

**Description**: Creates a new mastery profile (all domains at tier 0).

**Returns**: JSON `MasteryProfile`

---

### 9.2. `mastery_gain_xp`

**Signature**:
```c
char* mastery_gain_xp(
    const char* profile_json,
    uint32_t domain_id,
    uint64_t amount
);
```

**Description**: Adds XP to a mastery domain.

**Parameters**:
- `domain_id`: See `mastery_get_all_domains()`
- `amount`: XP to add

**Returns**: Updated `MasteryProfile` (with tier-ups if applicable)

---

### 9.3. `mastery_get_tier`

**Signature**:
```c
int32_t mastery_get_tier(const char* profile_json, uint32_t domain_id);
```

**Description**: Returns current tier for a domain.

**Returns**: `0` = Novice, `1` = Apprentice, `2` = Journeyman, `3` = Adept, `4` = Expert, `5` = Master

---

### 9.4. `mastery_xp_for_action`

**Signature**:
```c
uint64_t mastery_xp_for_action(const char* action_name);
```

**Description**: Returns XP reward for an action (e.g., "kill", "parry", "dodge").

---

### 9.5. `mastery_get_all_domains`

**Signature**:
```c
char* mastery_get_all_domains();
```

**Description**: Returns JSON array of all 21 mastery domains with IDs and names.

**Example Output**:
```json
[
  {"id": 0, "name": "Sword"},
  {"id": 1, "name": "Bow"},
  ...
  {"id": 20, "name": "Aerial"}
]
```

---

## 10. Specialization

### 10.1. `spec_get_all_branches`

**Signature**:
```c
char* spec_get_all_branches();
```

**Description**: Returns all specialization branches (3 per weapon type).

---

### 10.2. `spec_create_profile`

**Signature**:
```c
char* spec_create_profile();
```

**Description**: Creates new specialization profile (no branches chosen).

---

### 10.3. `spec_choose_branch`

**Signature**:
```c
char* spec_choose_branch(
    const char* profile_json,
    uint32_t domain_id,
    uint32_t branch_index
);
```

**Description**: Chooses a specialization branch (Expert tier required).

**Parameters**:
- `branch_index`: `0`, `1`, or `2` (e.g., Berserker, Duelist, Guardian for Sword)

---

### 10.4. `spec_find_synergies`

**Signature**:
```c
char* spec_find_synergies(const char* branch_ids_json);
```

**Description**: Finds cross-spec synergies.

**Input**: JSON array of branch IDs `[0, 5, 10]`

**Returns**: JSON array of synergy descriptions

---

## 11. Abilities

### 11.1. `ability_get_defaults`

**Signature**:
```c
char* ability_get_defaults();
```

**Description**: Returns all default abilities (22 total).

---

### 11.2. `ability_create_loadout`

**Signature**:
```c
char* ability_create_loadout();
```

**Description**: Creates empty ability loadout (6 hotbar slots).

---

### 11.3. `ability_learn`

**Signature**:
```c
char* ability_learn(const char* loadout_json, const char* ability_id);
```

**Description**: Learns a new ability.

---

### 11.4. `ability_equip`

**Signature**:
```c
char* ability_equip(
    const char* loadout_json,
    uint32_t slot,
    const char* ability_id
);
```

**Description**: Equips an ability to a hotbar slot (0-5).

---

## 12. Sockets

### 12.1. `socket_get_starter_gems`

**Signature**:
```c
char* socket_get_starter_gems();
```

**Description**: Returns 6 starter gems (2 per color).

---

### 12.2. `socket_get_starter_runes`

**Signature**:
```c
char* socket_get_starter_runes();
```

**Description**: Returns 3 starter runes.

---

### 12.3. `socket_create_equipment`

**Signature**:
```c
char* socket_create_equipment(
    uint32_t num_sockets,
    const char* socket_colors_json
);
```

**Description**: Creates socketed equipment.

**Parameters**:
- `num_sockets`: Number of sockets (0-6)
- `socket_colors_json`: JSON array of color IDs `[0, 1, 2]` (0=Red, 1=Blue, 2=Yellow)

---

### 12.4. `socket_insert_gem`

**Signature**:
```c
char* socket_insert_gem(
    const char* equipment_json,
    uint32_t socket_index,
    const char* gem_id
);
```

**Description**: Inserts a gem into a socket.

---

### 12.5. `socket_insert_rune`

**Signature**:
```c
char* socket_insert_rune(
    const char* equipment_json,
    uint32_t socket_index,
    const char* rune_id
);
```

**Description**: Inserts a rune into a socket.

---

### 12.6. `socket_combine_gems`

**Signature**:
```c
char* socket_combine_gems(const char* gems_json);
```

**Description**: Combines 3 gems of same color into higher tier.

**Input**: JSON array of 3 gem IDs

**Returns**: New `Gem` JSON (or null if invalid)

---

## 13. Cosmetics

### 13.1. `cosmetic_get_all`

**Signature**:
```c
char* cosmetic_get_all();
```

**Description**: Returns all cosmetic items (60 total).

---

### 13.2. `cosmetic_get_all_dyes`

**Signature**:
```c
char* cosmetic_get_all_dyes();
```

**Description**: Returns all dye items.

---

### 13.3. `cosmetic_create_profile`

**Signature**:
```c
char* cosmetic_create_profile();
```

**Description**: Creates new cosmetic profile.

---

### 13.4. `cosmetic_unlock`

**Signature**:
```c
char* cosmetic_unlock(const char* profile_json, const char* cosmetic_id);
```

**Description**: Unlocks a cosmetic item.

---

### 13.5. `cosmetic_apply_transmog`

**Signature**:
```c
char* cosmetic_apply_transmog(
    const char* profile_json,
    uint32_t slot_id,
    const char* cosmetic_id
);
```

**Description**: Applies transmog to a slot.

**Parameters**:
- `slot_id`: `0` = Helmet, `1` = Chest, `2` = Gloves, `3` = Boots, `4` = Weapon, `5` = Back

---

### 13.6. `cosmetic_apply_dye`

**Signature**:
```c
char* cosmetic_apply_dye(
    const char* profile_json,
    uint32_t slot_id,
    uint32_t channel_id,
    const char* dye_id
);
```

**Description**: Applies dye to a slot channel.

**Parameters**:
- `channel_id`: `0` = Primary, `1` = Secondary, `2` = Accent

---

## 14. Tutorial

### 14.1. `tutorial_get_steps`

**Signature**:
```c
char* tutorial_get_steps();
```

**Description**: Returns all tutorial steps (10 total).

---

### 14.2. `tutorial_get_hints`

**Signature**:
```c
char* tutorial_get_hints();
```

**Description**: Returns all context hints (15 total).

---

### 14.3. `tutorial_create_progress`

**Signature**:
```c
char* tutorial_create_progress();
```

**Description**: Creates new tutorial progress tracker.

---

### 14.4. `tutorial_complete_step`

**Signature**:
```c
char* tutorial_complete_step(const char* progress_json, const char* step_id);
```

**Description**: Marks a tutorial step as completed.

---

### 14.5. `tutorial_completion_percent`

**Signature**:
```c
float tutorial_completion_percent(const char* progress_json);
```

**Description**: Returns tutorial completion percentage (0.0 - 1.0).

---

## 15. Achievements

### 15.1. `achievement_create_tracker`

**Signature**:
```c
char* achievement_create_tracker();
```

**Description**: Creates new achievement tracker (45 achievements).

---

### 15.2. `achievement_increment`

**Signature**:
```c
char* achievement_increment(
    const char* tracker_json,
    const char* achievement_id,
    uint32_t amount
);
```

**Description**: Increments progress for an achievement.

---

### 15.3. `achievement_check_all`

**Signature**:
```c
char* achievement_check_all(const char* tracker_json);
```

**Description**: Returns all newly unlocked achievements.

---

### 15.4. `achievement_completion_percent`

**Signature**:
```c
float achievement_completion_percent(const char* tracker_json);
```

**Description**: Returns achievement completion percentage.

---

## 16. Seasons

### 16.1. `season_create_pass`

**Signature**:
```c
char* season_create_pass(uint32_t season_number, const char* name);
```

**Description**: Creates a new season pass (100 tiers).

---

### 16.2. `season_add_xp`

**Signature**:
```c
char* season_add_xp(const char* pass_json, uint64_t amount);
```

**Description**: Adds XP to season pass (auto-levels).

---

### 16.3. `season_generate_dailies`

**Signature**:
```c
char* season_generate_dailies(uint64_t day_seed);
```

**Description**: Generates 3 daily quests.

---

### 16.4. `season_generate_weeklies`

**Signature**:
```c
char* season_generate_weeklies(uint64_t week_seed);
```

**Description**: Generates 3 weekly quests.

---

### 16.5. `season_get_rewards`

**Signature**:
```c
char* season_get_rewards(uint32_t season_number);
```

**Description**: Returns season pass reward table (100 tiers).

---

## 17. Social

### 17.1. `social_create_guild`

**Signature**:
```c
char* social_create_guild(
    const char* guild_id,
    const char* name,
    const char* leader_id
);
```

**Description**: Creates a new guild.

---

### 17.2. `social_guild_add_member`

**Signature**:
```c
char* social_guild_add_member(
    const char* guild_json,
    const char* player_id,
    const char* player_name
);
```

**Description**: Adds member to guild.

---

### 17.3. `social_create_party`

**Signature**:
```c
char* social_create_party(const char* party_id, const char* leader_id);
```

**Description**: Creates a new party (max 4 players).

---

### 17.4. `social_party_add_member`

**Signature**:
```c
char* social_party_add_member(
    const char* party_json,
    const char* player_id,
    uint32_t role_id
);
```

**Description**: Adds member to party with role.

**Parameters**:
- `role_id`: `0` = Tank, `1` = DPS, `2` = Support, `3` = Flex

---

### 17.5. `social_create_trade`

**Signature**:
```c
char* social_create_trade(
    const char* trade_id,
    const char* player_a_id,
    const char* player_b_id
);
```

**Description**: Creates a new trade session.

---

### 17.6. `social_trade_add_item`

**Signature**:
```c
char* social_trade_add_item(
    const char* trade_json,
    uint32_t player_index,
    const char* item_id,
    uint32_t quantity,
    uint64_t gold
);
```

**Description**: Adds item/gold to trade.

**Parameters**:
- `player_index`: `0` = Player A, `1` = Player B

---

### 17.7. `social_trade_lock`

**Signature**:
```c
char* social_trade_lock(const char* trade_json, uint32_t player_index);
```

**Description**: Locks trade for a player.

---

### 17.8. `social_trade_confirm`

**Signature**:
```c
char* social_trade_confirm(const char* trade_json, uint32_t player_index);
```

**Description**: Confirms trade for a player.

---

### 17.9. `social_trade_execute`

**Signature**:
```c
char* social_trade_execute(const char* trade_json);
```

**Description**: Executes trade if both players confirmed.

**Returns**: Trade result JSON (success/failure)

---

## 18. Mutators

### 18.1. `generate_floor_mutators`

**Signature**:
```c
char* generate_floor_mutators(uint64_t seed, uint32_t floor_id);
```

**Description**: Generates floor mutators (1-4 depending on tier).

**Returns**: JSON array of mutators with types, difficulty, rewards

---

### 18.2. `get_all_mutator_types`

**Signature**:
```c
char* get_all_mutator_types();
```

**Description**: Returns all 28 mutator types in 5 categories.

---

### 18.3. `compute_mutator_effects`

**Signature**:
```c
char* compute_mutator_effects(const char* mutators_json);
```

**Description**: Computes combined stat effects from active mutators.

**Returns**: JSON with total difficulty, reward bonus, stat modifiers

---

## 19. Game Flow

### 19.1. `get_all_game_states`

**Signature**:
```c
char* get_all_game_states();
```

**Description**: Returns all 7 game states (Loading, MainMenu, CharacterSelect, etc.).

---

### 19.2. `get_all_sub_states`

**Signature**:
```c
char* get_all_sub_states();
```

**Description**: Returns all 7 in-game sub-states (Exploring, Combat, Paused, etc.).

---

## 20. Save Migration

### 20.1. `migrate_save`

**Signature**:
```c
char* migrate_save(const char* save_json);
```

**Description**: Migrates save file to current version.

**Returns**: Migrated save JSON with `MigrationResult`

---

### 20.2. `get_save_version`

**Signature**:
```c
uint32_t get_save_version(const char* save_json);
```

**Description**: Extracts version number from save file.

---

### 20.3. `create_new_save`

**Signature**:
```c
char* create_new_save(const char* player_name);
```

**Description**: Creates a new save file at current version.

---

### 20.4. `get_current_save_version`

**Signature**:
```c
uint32_t get_current_save_version();
```

**Description**: Returns current save format version (3).

---

### 20.5. `validate_save`

**Signature**:
```c
uint32_t validate_save(const char* save_json);
```

**Description**: Validates save file structure.

**Returns**: `1` = valid, `0` = invalid

---

## 21. Logging

### 21.1. `logging_get_default_config`

**Signature**:
```c
char* logging_get_default_config();
```

**Description**: Returns default tracing configuration.

---

### 21.2. `logging_init`

**Signature**:
```c
void logging_init(const char* config_json);
```

**Description**: Initializes tracing with custom config (idempotent).

---

### 21.3. `logging_get_snapshot`

**Signature**:
```c
char* logging_get_snapshot();
```

**Description**: Returns current logging status and configuration.

---

### 21.4. `logging_log_message`

**Signature**:
```c
void logging_log_message(
    uint32_t level,
    const char* target,
    const char* message
);
```

**Description**: Logs a message at specified level.

**Parameters**:
- `level`: `0` = Trace, `1` = Debug, `2` = Info, `3` = Warn, `4` = Error

---

## 22. Replay System

### 22.1. `replay_start_recording`

**Signature**:
```c
void replay_start_recording(
    uint64_t seed,
    uint32_t floor_id,
    const char* player_id,
    const char* metadata_json,
    uint64_t start_tick
);
```

**Description**: Starts replay recording.

---

### 22.2. `replay_record_frame`

**Signature**:
```c
void replay_record_frame(
    uint64_t tick,
    uint32_t input_type,
    const char* payload_json
);
```

**Description**: Records an input frame.

**Parameters**:
- `input_type`: `0` = Move, `1` = Attack, `2` = Parry, `3` = Dodge, `4` = UseAbility, `5` = Interact, `6` = Jump, `7` = ChangeWeapon

---

### 22.3. `replay_stop_recording`

**Signature**:
```c
char* replay_stop_recording(uint32_t outcome, uint64_t current_tick);
```

**Description**: Stops recording and returns `ReplayRecording` JSON.

**Parameters**:
- `outcome`: `0` = Victory, `1` = Defeat, `2` = Timeout

---

### 22.4. `replay_create_playback`

**Signature**:
```c
char* replay_create_playback(const char* recording_json);
```

**Description**: Creates playback controller from recording.

---

### 22.5. `replay_get_snapshot`

**Signature**:
```c
char* replay_get_snapshot();
```

**Description**: Returns current replay recorder status.

---

### 22.6. `replay_get_input_types`

**Signature**:
```c
char* replay_get_input_types();
```

**Description**: Returns all input type IDs and names.

---

## 23. Tower Map

### 23.1. `towermap_create`

**Signature**:
```c
char* towermap_create();
```

**Description**: Creates new tower map (empty).

---

### 23.2. `towermap_discover_floor`

**Signature**:
```c
char* towermap_discover_floor(
    const char* map_json,
    uint32_t floor_id,
    uint8_t tier,
    uint32_t total_rooms,
    uint32_t total_monsters,
    uint32_t total_chests
);
```

**Description**: Discovers a new floor (marks as visited, sets totals).

---

### 23.3. `towermap_clear_floor`

**Signature**:
```c
char* towermap_clear_floor(
    const char* map_json,
    uint32_t floor_id,
    float clear_time_secs
);
```

**Description**: Marks floor as cleared, records time.

---

### 23.4. `towermap_record_death`

**Signature**:
```c
char* towermap_record_death(const char* map_json, uint32_t floor_id);
```

**Description**: Records a death on a floor.

---

### 23.5. `towermap_get_floor`

**Signature**:
```c
char* towermap_get_floor(const char* map_json, uint32_t floor_id);
```

**Description**: Returns `FloorMapEntry` for a specific floor.

---

### 23.6. `towermap_get_overview`

**Signature**:
```c
char* towermap_get_overview(const char* map_json);
```

**Description**: Returns `TowerMapOverview` (stats, highest floor, tier counts).

---

### 23.7. `towermap_discover_room`

**Signature**:
```c
char* towermap_discover_room(const char* map_json, uint32_t floor_id);
```

**Description**: Increments discovered rooms counter.

---

### 23.8. `towermap_kill_monster`

**Signature**:
```c
char* towermap_kill_monster(const char* map_json, uint32_t floor_id);
```

**Description**: Increments monsters killed counter.

---

## Error Codes

All functions return `null` on error. Common error scenarios:

1. **Invalid JSON**: Malformed input
2. **Missing field**: Required field not present
3. **Out of range**: Invalid ID or index
4. **Logic error**: Invalid state transition

**Best Practice**: Always check for `null` before dereferencing.

```cpp
char* result = generate_floor(42, 1);
if (result == nullptr) {
    UE_LOG(LogTemp, Error, TEXT("Floor generation failed"));
    return;
}
// Use result...
free_string(result);
```

---

## Performance Notes

- **Caching**: Floor specs are deterministic — cache by `(seed, floor_id)`.
- **Batch**: Use batch functions (e.g., `generate_floor_monsters`) instead of loops.
- **Memory**: Call `free_string()` promptly to avoid leaks.
- **Threading**: FFI calls are thread-safe (no global mutable state).

---

## Version History

- **0.1.0** (Session 1-14): Initial 16 exports
- **0.2.0** (Session 15): Expanded to 46 exports (mastery, abilities, sockets, etc.)
- **0.3.0** (Session 18): Added CI/CD, edge case tests
- **0.4.0** (Session 20): Added mutators, game flow, save migration (74 exports)
- **0.5.0** (Session 21-22): Added logging, replay, tower map, hot-reload, analytics (92 exports)

---

**End of API Reference**
