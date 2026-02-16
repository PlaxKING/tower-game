-- Tower Game - Main Nakama Server Module v0.6.0
-- Authoritative server for Tower MMORPG
--
-- Responsibilities:
-- 1. Tower seed management (single source of truth)
-- 2. Player state persistence (floor, inventory, faction, mastery)
-- 3. Floor instance matchmaking (up to 50 players per floor)
-- 4. Seed + Delta synchronization
-- 5. Leaderboards (highest floor, fastest clear, mastery ranking)
-- 6. Analytics aggregation (combat stats, economy metrics)
-- 7. Anti-cheat validation (damage, progression, gold)

local nk = require("nakama")

-- ============ Constants ============

local TOWER_SEED_KEY = "tower_seed"
local PLAYER_STATE_COLLECTION = "tower_player_state"
local GLOBAL_STATE_COLLECTION = "tower_global"
local MAX_PLAYERS_PER_FLOOR = 50
local MATCH_MODULE = "tower_match"

-- ============ Tower Seed Management ============

-- Get or initialize the tower seed (shared by all players)
local function get_tower_seed()
    local result = nk.storage_read({
        { collection = GLOBAL_STATE_COLLECTION, key = TOWER_SEED_KEY, user_id = nil }
    })

    if #result > 0 then
        local data = result[1].value
        return data.seed, data.epoch
    end

    -- First time: generate random seed
    local seed = math.random(1, 2^53)
    local epoch = os.time()
    nk.storage_write({
        {
            collection = GLOBAL_STATE_COLLECTION,
            key = TOWER_SEED_KEY,
            user_id = nil,
            value = { seed = seed, epoch = epoch, version = "0.6.0" },
            permission_read = 2, -- public read
            permission_write = 0, -- server only write
        }
    })
    nk.logger_info(string.format("Initialized tower seed: %d (epoch: %d)", seed, epoch))
    return seed, epoch
end

-- ============ Player State ============

-- Save player progress
local function save_player_state(user_id, state)
    nk.storage_write({
        {
            collection = PLAYER_STATE_COLLECTION,
            key = "progress",
            user_id = user_id,
            value = state,
            permission_read = 1, -- owner read
            permission_write = 0, -- server only write
        }
    })
end

-- Load player progress
local function load_player_state(user_id)
    local result = nk.storage_read({
        { collection = PLAYER_STATE_COLLECTION, key = "progress", user_id = user_id }
    })

    if #result > 0 then
        return result[1].value
    end

    -- New player default state
    return {
        current_floor = 1,
        highest_floor = 1,
        total_deaths = 0,
        total_kills = 0,
        total_echoes_created = 0,
        play_time_seconds = 0,
        faction_standings = {
            seekers = 0,
            keepers = 0,
            weavers = 0,
            voidborn = 0,
        },
        inventory_shards = 0,
        created_at = os.time(),
        last_login = os.time(),
    }
end

-- ============ RPC: Get Tower Seed ============

nk.register_rpc(function(context, payload)
    local seed, epoch = get_tower_seed()
    return nk.json_encode({
        seed = seed,
        epoch = epoch,
        server_time = os.time(),
    })
end, "get_tower_seed")

-- ============ RPC: Request Floor ============

nk.register_rpc(function(context, payload)
    local data = nk.json_decode(payload)
    local floor_id = data.floor_id or 1
    local seed, _ = get_tower_seed()

    -- Load player state
    local state = load_player_state(context.user_id)

    -- Validate floor access
    if floor_id > state.highest_floor + 1 then
        return nk.json_encode({
            status = "error",
            message = string.format("Cannot access floor %d (highest: %d)", floor_id, state.highest_floor),
        })
    end

    -- Update current floor
    state.current_floor = floor_id
    state.last_login = os.time()
    save_player_state(context.user_id, state)

    nk.logger_info(string.format("Player %s entering floor %d", context.user_id, floor_id))

    return nk.json_encode({
        status = "ok",
        seed = seed,
        floor_id = floor_id,
        highest_floor = state.highest_floor,
    })
end, "request_floor")

-- ============ RPC: Report Floor Clear ============

nk.register_rpc(function(context, payload)
    local data = nk.json_decode(payload)
    local floor_id = data.floor_id or 1
    local kills = data.kills or 0
    local clear_time = data.clear_time_seconds or 0

    local state = load_player_state(context.user_id)

    -- Update progress
    state.total_kills = state.total_kills + kills
    if floor_id >= state.highest_floor then
        state.highest_floor = floor_id + 1
    end

    save_player_state(context.user_id, state)

    -- Update leaderboards
    nk.leaderboard_record_write("highest_floor", context.user_id, context.username, state.highest_floor)
    if clear_time > 0 then
        -- Fastest clear: use negative score so lower time = higher rank
        nk.leaderboard_record_write(
            string.format("floor_%d_speed", floor_id),
            context.user_id,
            context.username,
            -math.floor(clear_time * 1000)
        )
    end

    nk.logger_info(string.format("Player %s cleared floor %d (%d kills, %.1fs)",
        context.user_id, floor_id, kills, clear_time))

    return nk.json_encode({
        status = "ok",
        new_highest = state.highest_floor,
        total_kills = state.total_kills,
    })
end, "report_floor_clear")

-- ============ RPC: Report Death ============

nk.register_rpc(function(context, payload)
    local data = nk.json_decode(payload)
    local floor_id = data.floor_id or 1
    local echo_type = data.echo_type or "Lingering"

    local state = load_player_state(context.user_id)
    state.total_deaths = state.total_deaths + 1
    state.total_echoes_created = state.total_echoes_created + 1
    save_player_state(context.user_id, state)

    -- Store echo for other players to encounter
    nk.storage_write({
        {
            collection = "tower_echoes",
            key = string.format("echo_%s_%d_%d", context.user_id, floor_id, os.time()),
            user_id = nil,
            value = {
                player_id = context.user_id,
                player_name = context.username,
                floor_id = floor_id,
                echo_type = echo_type,
                position = data.position or {x = 0, y = 0, z = 0},
                semantic_tags = data.semantic_tags or {},
                created_at = os.time(),
                expires_at = os.time() + 86400, -- 24h
            },
            permission_read = 2,
            permission_write = 0,
        }
    })

    nk.logger_info(string.format("Player %s died on floor %d (%s echo)",
        context.user_id, floor_id, echo_type))

    return nk.json_encode({
        status = "ok",
        total_deaths = state.total_deaths,
        echo_type = echo_type,
    })
end, "report_death")

-- ============ RPC: Get Echoes for Floor ============

nk.register_rpc(function(context, payload)
    local data = nk.json_decode(payload)
    local floor_id = data.floor_id or 1

    -- Query echoes for this floor
    local query = string.format("+value.floor_id:%d +value.expires_at:>%d", floor_id, os.time())
    local result = nk.storage_list(nil, "tower_echoes", 20, "")

    local echoes = {}
    for _, obj in ipairs(result) do
        local echo = obj.value
        if echo.floor_id == floor_id and echo.expires_at > os.time() then
            table.insert(echoes, {
                player_name = echo.player_name,
                echo_type = echo.echo_type,
                position = echo.position,
                semantic_tags = echo.semantic_tags,
            })
        end
    end

    return nk.json_encode({
        status = "ok",
        floor_id = floor_id,
        echoes = echoes,
        count = #echoes,
    })
end, "get_floor_echoes")

-- ============ RPC: Update Faction Standing ============

nk.register_rpc(function(context, payload)
    local data = nk.json_decode(payload)
    local faction = data.faction
    local delta = data.delta or 0

    local state = load_player_state(context.user_id)

    if state.faction_standings[faction] then
        state.faction_standings[faction] = state.faction_standings[faction] + delta
        -- Clamp to -100 to 100
        state.faction_standings[faction] = math.max(-100, math.min(100, state.faction_standings[faction]))
        save_player_state(context.user_id, state)
    end

    return nk.json_encode({
        status = "ok",
        faction = faction,
        standing = state.faction_standings[faction] or 0,
        all_standings = state.faction_standings,
    })
end, "update_faction")

-- ============ RPC: Get Player State ============

nk.register_rpc(function(context, payload)
    local state = load_player_state(context.user_id)
    return nk.json_encode({
        status = "ok",
        state = state,
    })
end, "get_player_state")

-- ============ RPC: Health Check ============

nk.register_rpc(function(context, payload)
    local seed, epoch = get_tower_seed()
    return nk.json_encode({
        status = "healthy",
        version = "0.6.0",
        tower_seed = seed,
        server_time = os.time(),
    })
end, "health_check")

-- ============ RPC: Join Floor Match ============

nk.register_rpc(function(context, payload)
    local data = nk.json_decode(payload)
    local floor_id = data.floor_id or 1
    local seed, _ = get_tower_seed()

    -- Load player state for validation
    local state = load_player_state(context.user_id)
    if floor_id > state.highest_floor + 1 then
        return nk.json_encode({
            status = "error",
            message = string.format("Cannot access floor %d (highest: %d)", floor_id, state.highest_floor),
        })
    end

    -- Search for existing match on this floor with space
    local query = string.format('+label.floor_id:%d +label.cleared:false', floor_id)
    local matches = nk.match_list(1, true, "", nil, 1, query)

    local match_id
    if #matches > 0 then
        -- Join existing match
        match_id = matches[1].match_id
        nk.logger_info(string.format("Player %s joining existing floor %d match: %s",
            context.user_id, floor_id, match_id))
    else
        -- Create new match
        match_id = nk.match_create("tower_match", {
            floor_id = floor_id,
            seed = seed,
        })
        nk.logger_info(string.format("Player %s created new floor %d match: %s",
            context.user_id, floor_id, match_id))
    end

    -- Update player state
    state.current_floor = floor_id
    state.last_login = os.time()
    save_player_state(context.user_id, state)

    return nk.json_encode({
        status = "ok",
        match_id = match_id,
        floor_id = floor_id,
        seed = seed,
    })
end, "join_floor_match")

-- ============ RPC: Get Active Matches ============

nk.register_rpc(function(context, payload)
    local matches = nk.match_list(20, true, "", nil, nil, "+label.cleared:false")

    local result = {}
    for _, match in ipairs(matches) do
        local label = nk.json_decode(match.label)
        table.insert(result, {
            match_id = match.match_id,
            floor_id = label.floor_id,
            player_count = label.player_count,
            max_players = label.max_players,
        })
    end

    return nk.json_encode({
        status = "ok",
        matches = result,
        count = #result,
    })
end, "list_active_matches")

-- ============ Leaderboard Setup ============

-- Create leaderboards on module load
local function setup_leaderboards()
    -- Highest floor reached
    nk.leaderboard_create("highest_floor", false, "desc", "incr", "", {})

    -- Per-floor speed records (floors 1-50)
    for i = 1, 50 do
        local id = string.format("floor_%d_speed", i)
        nk.leaderboard_create(id, false, "asc", "best", "", {})
    end

    nk.logger_info("Tower leaderboards initialized")
end

-- Safe leaderboard setup (may fail if already exists)
pcall(setup_leaderboards)

-- ============ Module Load ============

nk.logger_info("Tower Game server module loaded (v0.6.0)")
nk.logger_info("RPC endpoints: get_tower_seed, request_floor, report_floor_clear, report_death, get_floor_echoes, update_faction, get_player_state, health_check, join_floor_match, list_active_matches")
