-- Tower Match Handler — Authoritative floor instance multiplayer
--
-- Each floor is a match with up to 50 concurrent players.
-- Players join via matchmaking or direct floor request.
--
-- State synchronized:
-- - Player positions (tick-based)
-- - Monster HP / alive status
-- - Floor clear progress
-- - Breath of Tower phase
-- - Echo spawns (real-time death echoes)
--
-- OpCodes:
--   1 = Player position update
--   2 = Player attack (target, damage, angle)
--   3 = Monster damage taken
--   4 = Monster defeated
--   5 = Player death + echo spawn
--   6 = Floor clear (stairs unlock)
--   7 = Breath phase change
--   8 = Player joined notification
--   9 = Player left notification
--  10 = Chat message
--  11 = Loot dropped
--  12 = Player interact (shrine, chest, NPC)

local nk = require("nakama")

local MAX_PLAYERS = 50
local TICK_RATE = 10 -- 10 updates/second
local EMPTY_TIMEOUT = 300 -- 5 minutes before empty match closes

-- ============ Match Init ============

local function match_init(context, setupstate)
    local state = {
        -- Floor info
        floor_id = setupstate.floor_id or 1,
        seed = setupstate.seed or 42,

        -- Players: { [user_id] = { position, hp, name, ... } }
        players = {},
        player_count = 0,

        -- Monsters: { [monster_id] = { hp, max_hp, position, alive } }
        monsters = {},
        monsters_alive = 0,
        monsters_total = 0,

        -- Floor state
        floor_cleared = false,
        stairs_unlocked = false,

        -- Breath of Tower
        breath_phase = "Pause",
        breath_progress = 0.0,
        elapsed_time = 0.0,

        -- Timing
        empty_timer = 0.0,
        tick_count = 0,
    }

    -- Generate monsters for this floor
    local monster_count = 5 + math.floor(state.floor_id * 0.5)
    for i = 1, monster_count do
        local hp = 100 + state.floor_id * 20
        state.monsters[tostring(i)] = {
            id = i,
            hp = hp,
            max_hp = hp,
            position = { x = math.random(-20, 20), y = 0, z = math.random(-20, 20) },
            alive = true,
        }
    end
    state.monsters_alive = monster_count
    state.monsters_total = monster_count

    local label = nk.json_encode({
        floor_id = state.floor_id,
        player_count = 0,
        max_players = MAX_PLAYERS,
        cleared = false,
    })

    nk.logger_info(string.format("Match created for floor %d (seed: %d, %d monsters)",
        state.floor_id, state.seed, monster_count))

    return state, TICK_RATE, label
end

-- ============ Match Join Attempt ============

local function match_join_attempt(context, dispatcher, tick, state, presence, metadata)
    -- Reject if full
    if state.player_count >= MAX_PLAYERS then
        return state, false, "Floor instance full (max " .. MAX_PLAYERS .. " players)"
    end

    -- Reject if floor already cleared
    if state.floor_cleared then
        return state, false, "Floor already cleared"
    end

    return state, true
end

-- ============ Match Join ============

local function match_join(context, dispatcher, tick, state, presences)
    for _, presence in ipairs(presences) do
        state.players[presence.user_id] = {
            user_id = presence.user_id,
            username = presence.username,
            position = { x = 0, y = 0, z = 0 },
            hp = 100,
            max_hp = 100,
            alive = true,
            joined_at = os.time(),
            kills = 0,
            damage_dealt = 0,
        }
        state.player_count = state.player_count + 1

        nk.logger_info(string.format("Player %s joined floor %d (%d/%d)",
            presence.username, state.floor_id, state.player_count, MAX_PLAYERS))

        -- Notify all players about new join
        local join_data = nk.json_encode({
            user_id = presence.user_id,
            username = presence.username,
            player_count = state.player_count,
        })
        dispatcher.broadcast_message(8, join_data)

        -- Send current state to joining player
        local sync_data = nk.json_encode({
            floor_id = state.floor_id,
            seed = state.seed,
            monsters = state.monsters,
            monsters_alive = state.monsters_alive,
            breath_phase = state.breath_phase,
            breath_progress = state.breath_progress,
            floor_cleared = state.floor_cleared,
            player_count = state.player_count,
        })
        dispatcher.broadcast_message(7, sync_data, { presence })
    end

    -- Update label
    local label = nk.json_encode({
        floor_id = state.floor_id,
        player_count = state.player_count,
        max_players = MAX_PLAYERS,
        cleared = state.floor_cleared,
    })
    dispatcher.match_label_update(label)

    return state
end

-- ============ Match Leave ============

local function match_leave(context, dispatcher, tick, state, presences)
    for _, presence in ipairs(presences) do
        state.players[presence.user_id] = nil
        state.player_count = state.player_count - 1

        nk.logger_info(string.format("Player %s left floor %d (%d remaining)",
            presence.username, state.floor_id, state.player_count))

        -- Notify remaining players
        local leave_data = nk.json_encode({
            user_id = presence.user_id,
            username = presence.username,
            player_count = state.player_count,
        })
        dispatcher.broadcast_message(9, leave_data)
    end

    -- Update label
    local label = nk.json_encode({
        floor_id = state.floor_id,
        player_count = state.player_count,
        max_players = MAX_PLAYERS,
        cleared = state.floor_cleared,
    })
    dispatcher.match_label_update(label)

    return state
end

-- ============ Match Loop (Tick) ============

local function match_loop(context, dispatcher, tick, state, messages)
    -- Empty match timeout
    if state.player_count == 0 then
        state.empty_timer = state.empty_timer + (1.0 / TICK_RATE)
        if state.empty_timer >= EMPTY_TIMEOUT then
            nk.logger_info(string.format("Floor %d match closing (empty timeout)", state.floor_id))
            return nil -- terminates match
        end
    else
        state.empty_timer = 0
    end

    state.tick_count = tick
    state.elapsed_time = tick / TICK_RATE

    -- Update Breath of Tower
    update_breath(state)

    -- Process incoming messages
    for _, message in ipairs(messages) do
        local op_code = message.op_code
        local data = {}
        if message.data and #message.data > 0 then
            data = nk.json_decode(message.data)
        end
        local sender = message.sender

        if op_code == 1 then
            -- Player position update
            handle_position_update(state, sender, data)

        elseif op_code == 2 then
            -- Player attack
            handle_player_attack(state, dispatcher, sender, data)

        elseif op_code == 3 then
            -- Direct monster damage (validated server-side)
            handle_monster_damage(state, dispatcher, sender, data)

        elseif op_code == 5 then
            -- Player death
            handle_player_death(state, dispatcher, sender, data)

        elseif op_code == 10 then
            -- Chat message (relay to all)
            dispatcher.broadcast_message(10, message.data)

        elseif op_code == 12 then
            -- Player interact
            handle_interact(state, dispatcher, sender, data)
        end
    end

    -- Broadcast positions every 2 ticks (5 times/sec)
    if tick % 2 == 0 then
        broadcast_positions(state, dispatcher)
    end

    return state
end

-- ============ Match Terminate ============

local function match_terminate(context, dispatcher, tick, state, grace_seconds)
    nk.logger_info(string.format("Floor %d match terminating (%d players, %d ticks)",
        state.floor_id, state.player_count, tick))
    return nil
end

-- ============ Match Signal ============

local function match_signal(context, dispatcher, tick, state, data)
    -- External signals (admin commands, etc.)
    return state, data
end

-- ============ Helper Functions ============

function update_breath(state)
    -- Breath of Tower cycle: 4 phases
    -- Inhale (0-300s), Hold (300-420s), Exhale (420-600s), Pause (600-720s)
    local cycle_time = state.elapsed_time % 720
    local phase, progress

    if cycle_time < 300 then
        phase = "Inhale"
        progress = cycle_time / 300
    elseif cycle_time < 420 then
        phase = "Hold"
        progress = (cycle_time - 300) / 120
    elseif cycle_time < 600 then
        phase = "Exhale"
        progress = (cycle_time - 420) / 180
    else
        phase = "Pause"
        progress = (cycle_time - 600) / 120
    end

    if phase ~= state.breath_phase then
        state.breath_phase = phase
        nk.logger_info(string.format("Floor %d: Breath phase -> %s", state.floor_id, phase))
    end
    state.breath_progress = progress
end

function handle_position_update(state, sender, data)
    local player = state.players[sender.user_id]
    if not player then return end

    -- Anti-cheat: validate movement speed
    if data.position then
        local dx = data.position.x - player.position.x
        local dz = data.position.z - player.position.z
        local dist = math.sqrt(dx * dx + dz * dz)
        local max_speed = 20.0 -- units per tick (generous for network jitter)

        if dist > max_speed then
            nk.logger_warn(string.format("Player %s moved too fast: %.1f units/tick (max: %.1f)",
                sender.username, dist, max_speed))
            return -- reject teleport
        end

        player.position = data.position
    end
end

function handle_player_attack(state, dispatcher, sender, data)
    local player = state.players[sender.user_id]
    if not player or not player.alive then return end

    local target_id = tostring(data.target_id)
    local monster = state.monsters[target_id]
    if not monster or not monster.alive then return end

    -- Server-side damage validation
    local base_damage = data.damage or 30
    local angle_id = data.angle_id or 0
    local combo_step = data.combo_step or 0

    -- Anti-cheat: clamp damage to reasonable range
    local max_damage = 200 -- absolute cap per hit
    if base_damage > max_damage then
        nk.logger_warn(string.format("Player %s damage clamped: %.0f -> %.0f",
            sender.username, base_damage, max_damage))
        base_damage = max_damage
    end

    -- Apply angle multiplier
    local angle_mult = 1.0
    if angle_id == 1 then angle_mult = 0.7       -- side
    elseif angle_id == 2 then angle_mult = 1.5    -- back
    end

    -- Apply combo multiplier
    local combo_mult = 1.0 + combo_step * 0.15

    -- Breath phase multiplier
    local breath_mult = 1.0
    if state.breath_phase == "Hold" then breath_mult = 1.3 end

    local final_damage = base_damage * angle_mult * combo_mult * breath_mult

    -- Apply damage to monster
    monster.hp = monster.hp - final_damage
    player.damage_dealt = player.damage_dealt + final_damage

    if monster.hp <= 0 then
        monster.hp = 0
        monster.alive = false
        state.monsters_alive = state.monsters_alive - 1
        player.kills = player.kills + 1

        -- Broadcast monster defeat
        local defeat_data = nk.json_encode({
            monster_id = data.target_id,
            killer = sender.user_id,
            killer_name = sender.username,
            monsters_remaining = state.monsters_alive,
        })
        dispatcher.broadcast_message(4, defeat_data)

        nk.logger_info(string.format("Monster %s defeated by %s on floor %d (%d remaining)",
            target_id, sender.username, state.floor_id, state.monsters_alive))

        -- Generate loot drop
        local loot_data = nk.json_encode({
            monster_id = data.target_id,
            position = monster.position,
            floor_id = state.floor_id,
            killer = sender.user_id,
        })
        dispatcher.broadcast_message(11, loot_data)

        -- Check floor clear
        if state.monsters_alive <= 0 and not state.floor_cleared then
            state.floor_cleared = true
            state.stairs_unlocked = true

            local clear_data = nk.json_encode({
                floor_id = state.floor_id,
                clear_time = state.elapsed_time,
                player_count = state.player_count,
            })
            dispatcher.broadcast_message(6, clear_data)

            nk.logger_info(string.format("Floor %d CLEARED! (%.1fs, %d players)",
                state.floor_id, state.elapsed_time, state.player_count))
        end
    else
        -- Broadcast damage taken
        local damage_data = nk.json_encode({
            monster_id = data.target_id,
            damage = final_damage,
            remaining_hp = monster.hp,
            attacker = sender.user_id,
        })
        dispatcher.broadcast_message(3, damage_data)
    end
end

function handle_monster_damage(state, dispatcher, sender, data)
    -- Direct damage from server-validated source (traps, environmental)
    local target_id = tostring(data.monster_id)
    local monster = state.monsters[target_id]
    if not monster or not monster.alive then return end

    local damage = math.min(data.damage or 0, 500) -- cap environmental damage
    monster.hp = monster.hp - damage

    if monster.hp <= 0 then
        monster.hp = 0
        monster.alive = false
        state.monsters_alive = state.monsters_alive - 1

        local defeat_data = nk.json_encode({
            monster_id = data.monster_id,
            killer = "environment",
            monsters_remaining = state.monsters_alive,
        })
        dispatcher.broadcast_message(4, defeat_data)
    end
end

function handle_player_death(state, dispatcher, sender, data)
    local player = state.players[sender.user_id]
    if not player then return end

    player.alive = false
    player.hp = 0

    -- Create echo at death position
    local echo_data = nk.json_encode({
        user_id = sender.user_id,
        username = sender.username,
        position = player.position,
        echo_type = data.echo_type or "Lingering",
        floor_id = state.floor_id,
        kills = player.kills,
        damage_dealt = player.damage_dealt,
    })
    dispatcher.broadcast_message(5, echo_data)

    nk.logger_info(string.format("Player %s died on floor %d (%s echo, %d kills)",
        sender.username, state.floor_id, data.echo_type or "Lingering", player.kills))

    -- Respawn after 5 seconds
    player.alive = true
    player.hp = player.max_hp
end

function handle_interact(state, dispatcher, sender, data)
    local interact_type = data.type or "unknown"

    if interact_type == "stairs" and state.stairs_unlocked then
        -- Player wants to go to next floor
        local next_data = nk.json_encode({
            user_id = sender.user_id,
            username = sender.username,
            next_floor = state.floor_id + 1,
        })
        dispatcher.broadcast_message(12, next_data, { sender })

    elseif interact_type == "shrine" then
        local shrine_data = nk.json_encode({
            user_id = sender.user_id,
            faction = data.faction or "seekers",
            position = data.position,
        })
        dispatcher.broadcast_message(12, shrine_data)

    elseif interact_type == "chest" then
        local chest_data = nk.json_encode({
            user_id = sender.user_id,
            chest_id = data.chest_id,
            position = data.position,
            floor_id = state.floor_id,
        })
        dispatcher.broadcast_message(12, chest_data, { sender })
    end
end

function broadcast_positions(state, dispatcher)
    local positions = {}
    for user_id, player in pairs(state.players) do
        if player.alive then
            positions[user_id] = {
                position = player.position,
                hp = player.hp,
                username = player.username,
            }
        end
    end

    if next(positions) then
        local data = nk.json_encode({
            players = positions,
            breath_phase = state.breath_phase,
            breath_progress = state.breath_progress,
            monsters_alive = state.monsters_alive,
        })
        dispatcher.broadcast_message(1, data)
    end
end

-- ============ Register Match Handler ============

nk.register_match(
    "tower_match",
    {
        match_init = match_init,
        match_join_attempt = match_join_attempt,
        match_join = match_join,
        match_leave = match_leave,
        match_loop = match_loop,
        match_terminate = match_terminate,
        match_signal = match_signal,
    }
)

nk.logger_info("Tower Match handler registered (max " .. MAX_PLAYERS .. " players, " .. TICK_RATE .. " tick/s)")
