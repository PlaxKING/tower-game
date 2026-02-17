//! Combat System — Skill-based non-target action combat
//!
//! Core pillars (from CLAUDE.md):
//! - Positioning > stats, timing > cooldowns
//! - Angular hitboxes, parry windows (80-120ms), spatial tactics
//! - Combat is a "dance" — dynamic, skill-dependent
//!
//! ## Architecture
//! ```text
//! Input → CombatAction → validate_action() → apply state transition
//!   └→ CombatStateMachine tracks: Idle/Attacking/Blocking/Dodging/Parrying/Staggered/Dead
//!   └→ WeaponMoveset defines: combo chains, damage, timing, hitbox shapes
//!   └→ DamagePipeline: base × angle × combo × semantic × resistance = final
//! ```

use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ============================================================================
// Combat State Machine
// ============================================================================

/// Current combat state of an entity
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct CombatState {
    pub phase: CombatPhase,
    /// Timer for current phase (counts down to 0)
    pub phase_timer: f32,
    /// Current combo step (0 = no combo)
    pub combo_step: u8,
    /// Max combo chain length for current weapon
    pub max_combo: u8,
    /// Time remaining to input next combo (0 = combo dropped)
    pub combo_window: f32,
    /// Active i-frames (invincibility during dodge)
    pub i_frames: f32,
    /// Parry window remaining (successful parry if hit during this)
    pub parry_window: f32,
    /// Stagger resistance (prevents stagger if > 0, decremented by attacks)
    pub poise: f32,
    pub max_poise: f32,
    /// Poise regen per second
    pub poise_regen: f32,
    /// Currently facing direction (yaw in radians)
    pub facing: f32,
    /// Target entity (for auto-aim assist at close range)
    pub target: Option<u64>,
}

impl Default for CombatState {
    fn default() -> Self {
        Self {
            phase: CombatPhase::Idle,
            phase_timer: 0.0,
            combo_step: 0,
            max_combo: 3,
            combo_window: 0.0,
            i_frames: 0.0,
            parry_window: 0.0,
            poise: 100.0,
            max_poise: 100.0,
            poise_regen: 20.0,
            facing: 0.0,
            target: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatPhase {
    /// Can act freely
    Idle,
    /// Executing an attack (locked into animation)
    Attacking,
    /// Holding block (reduces damage, drains stamina)
    Blocking,
    /// Short dodge roll with i-frames
    Dodging,
    /// Active parry window (reflect damage on success)
    Parrying,
    /// Hit stagger (cannot act)
    Staggered,
    /// Dead (awaiting respawn)
    Dead,
}

// ============================================================================
// Weapon System
// ============================================================================

/// Equipped weapon component
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct EquippedWeapon {
    pub weapon_type: WeaponType,
    pub weapon_id: String,
    /// Base damage per hit
    pub base_damage: f32,
    /// Attack speed multiplier (1.0 = normal)
    pub attack_speed: f32,
    /// Range in world units
    pub range: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponType {
    Sword,
    Spear,
    Hammer,
}

/// Defines timing and hitbox properties for each attack in a combo chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackData {
    /// Which combo step this is (0-indexed)
    pub combo_step: u8,
    /// Duration of the windup before damage frame (seconds)
    pub windup: f32,
    /// Duration of the active damage frame (seconds)
    pub active: f32,
    /// Duration of recovery after attack (seconds)
    pub recovery: f32,
    /// Damage multiplier for this combo step
    pub damage_mult: f32,
    /// Hitbox shape: (half_width, half_height, depth) local to attacker
    pub hitbox_half_extents: [f32; 3],
    /// Hitbox offset from attacker center (forward)
    pub hitbox_offset: f32,
    /// Poise damage dealt to target
    pub poise_damage: f32,
    /// Knockback force
    pub knockback: f32,
    /// Angular sweep in radians (e.g., sword = wide, spear = narrow)
    pub sweep_angle: f32,
}

/// All weapon movesets (loaded once, read by systems)
#[derive(Resource, Debug)]
pub struct WeaponMovesets {
    pub movesets: HashMap<WeaponType, Vec<AttackData>>,
}

impl Default for WeaponMovesets {
    fn default() -> Self {
        let mut movesets = HashMap::new();

        // === SWORD: Fast, wide sweeps, 4-hit combo ===
        movesets.insert(WeaponType::Sword, vec![
            AttackData {
                combo_step: 0,
                windup: 0.08,
                active: 0.10,
                recovery: 0.15,
                damage_mult: 1.0,
                hitbox_half_extents: [1.2, 0.8, 1.5],
                hitbox_offset: 1.5,
                poise_damage: 15.0,
                knockback: 1.0,
                sweep_angle: std::f32::consts::FRAC_PI_2, // 90°
            },
            AttackData {
                combo_step: 1,
                windup: 0.06,
                active: 0.10,
                recovery: 0.12,
                damage_mult: 1.1,
                hitbox_half_extents: [1.4, 0.8, 1.5],
                hitbox_offset: 1.5,
                poise_damage: 15.0,
                knockback: 1.2,
                sweep_angle: std::f32::consts::FRAC_PI_2,
            },
            AttackData {
                combo_step: 2,
                windup: 0.10,
                active: 0.12,
                recovery: 0.18,
                damage_mult: 1.3,
                hitbox_half_extents: [1.6, 1.0, 1.8],
                hitbox_offset: 1.8,
                poise_damage: 25.0,
                knockback: 2.0,
                sweep_angle: std::f32::consts::PI, // 180° finisher
            },
            AttackData {
                combo_step: 3,
                windup: 0.15,
                active: 0.15,
                recovery: 0.30,
                damage_mult: 1.8,
                hitbox_half_extents: [2.0, 1.2, 2.0],
                hitbox_offset: 2.0,
                poise_damage: 40.0,
                knockback: 3.5,
                sweep_angle: std::f32::consts::PI,
            },
        ]);

        // === SPEAR: Long range, narrow, 3-hit combo ===
        movesets.insert(WeaponType::Spear, vec![
            AttackData {
                combo_step: 0,
                windup: 0.10,
                active: 0.08,
                recovery: 0.14,
                damage_mult: 1.0,
                hitbox_half_extents: [0.4, 0.4, 2.5],
                hitbox_offset: 2.5,
                poise_damage: 20.0,
                knockback: 1.5,
                sweep_angle: 0.4, // ~23° narrow thrust
            },
            AttackData {
                combo_step: 1,
                windup: 0.08,
                active: 0.10,
                recovery: 0.12,
                damage_mult: 1.15,
                hitbox_half_extents: [0.6, 0.6, 2.8],
                hitbox_offset: 2.8,
                poise_damage: 22.0,
                knockback: 1.8,
                sweep_angle: 0.5,
            },
            AttackData {
                combo_step: 2,
                windup: 0.18,
                active: 0.12,
                recovery: 0.25,
                damage_mult: 1.6,
                hitbox_half_extents: [0.5, 0.5, 3.5],
                hitbox_offset: 3.2,
                poise_damage: 35.0,
                knockback: 4.0,
                sweep_angle: 0.3, // Very narrow piercing finisher
            },
        ]);

        // === HAMMER: Slow, devastating, 2-hit combo ===
        movesets.insert(WeaponType::Hammer, vec![
            AttackData {
                combo_step: 0,
                windup: 0.20,
                active: 0.12,
                recovery: 0.25,
                damage_mult: 1.5,
                hitbox_half_extents: [1.0, 1.0, 1.8],
                hitbox_offset: 1.8,
                poise_damage: 45.0,
                knockback: 3.0,
                sweep_angle: std::f32::consts::FRAC_PI_3, // 60°
            },
            AttackData {
                combo_step: 1,
                windup: 0.30,
                active: 0.15,
                recovery: 0.40,
                damage_mult: 2.5,
                hitbox_half_extents: [1.5, 1.5, 2.0],
                hitbox_offset: 1.5,
                poise_damage: 80.0,
                knockback: 5.0,
                sweep_angle: std::f32::consts::TAU, // 360° ground slam
            },
        ]);

        Self { movesets }
    }
}

// ============================================================================
// Combat Timing Constants
// ============================================================================

/// Parry window duration in seconds (80-120ms from design doc)
pub const PARRY_WINDOW_SECS: f32 = 0.100; // 100ms center of 80-120ms range
/// Parry success bonus: incoming damage is reflected as this multiplier
pub const PARRY_REFLECT_MULT: f32 = 0.5;
/// Dodge i-frame duration
pub const DODGE_IFRAMES_SECS: f32 = 0.200; // 200ms of invincibility
/// Dodge roll total duration
pub const DODGE_DURATION_SECS: f32 = 0.400;
/// Dodge roll distance (world units)
pub const DODGE_DISTANCE: f32 = 4.0;
/// Time window to input next combo after recovery ends
pub const COMBO_WINDOW_SECS: f32 = 0.300; // 300ms to continue combo
/// Block damage reduction multiplier (0.3 = 70% reduction)
pub const BLOCK_DAMAGE_MULT: f32 = 0.3;
/// Block poise cost per hit received
pub const BLOCK_POISE_COST: f32 = 10.0;
/// Stagger duration when poise breaks
pub const STAGGER_DURATION_SECS: f32 = 0.800;
/// Minimum time between attacks (spam prevention)
pub const ATTACK_COOLDOWN_SECS: f32 = 0.050;

// ============================================================================
// Combat Actions (input events)
// ============================================================================

/// Combat action request from player input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatAction {
    pub player_id: u64,
    pub action: ActionType,
    /// World position of the action (for validation)
    pub position: [f32; 3],
    /// Direction the player is facing (radians)
    pub facing: f32,
    /// Server timestamp when action was received
    pub timestamp: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    /// Light attack (combo chain)
    Attack,
    /// Block/guard
    Block,
    /// Release block
    BlockRelease,
    /// Parry (block + timing)
    Parry,
    /// Dodge roll in facing direction
    Dodge,
    /// Heavy attack (charged, no combo)
    HeavyAttack,
}

// ============================================================================
// Damage Pipeline
// ============================================================================

/// Attack angle relative to the target
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackAngle {
    Front,     // 0° ± 45°
    Side,      // 45°–135°
    Back,      // 135°–180°
}

impl AttackAngle {
    /// Calculate attack angle from attacker position/facing to target position/facing
    pub fn calculate(
        attacker_pos: Vec3,
        _attacker_facing: f32,
        target_pos: Vec3,
        target_facing: f32,
    ) -> Self {
        // Direction from target to attacker
        let to_attacker = (attacker_pos - target_pos).normalize_or_zero();
        // Target's forward direction
        let target_forward = Vec3::new(target_facing.cos(), 0.0, target_facing.sin());

        let dot = to_attacker.dot(target_forward);

        if dot > 0.707 {
            AttackAngle::Front  // Within ~45° of facing
        } else if dot < -0.707 {
            AttackAngle::Back   // Behind target
        } else {
            AttackAngle::Side   // Flanking
        }
    }

    /// Damage multiplier for this angle
    pub fn damage_multiplier(&self) -> f32 {
        match self {
            AttackAngle::Front => 1.0,
            AttackAngle::Side => 1.15,
            AttackAngle::Back => 1.4,
        }
    }
}

/// Full damage calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DamageCalcResult {
    pub base_damage: f32,
    pub angle_mult: f32,
    pub combo_mult: f32,
    pub semantic_mult: f32,
    pub final_damage: f32,
    pub poise_damage: f32,
    pub knockback: f32,
    pub angle: AttackAngle,
    pub was_critical: bool,
    pub was_blocked: bool,
    pub was_parried: bool,
}

/// Calculate damage through the full pipeline
pub fn calculate_damage(
    base_damage: f32,
    attack_data: &AttackData,
    angle: AttackAngle,
    combo_step: u8,
    semantic_affinity: f32,  // 0.0 = neutral, positive = bonus, negative = resistance
    target_blocking: bool,
    target_parrying: bool,
) -> DamageCalcResult {
    let angle_mult = angle.damage_multiplier();
    let combo_mult = 1.0 + combo_step as f32 * 0.15; // +15% per combo step
    let semantic_mult = 1.0 + semantic_affinity.clamp(-0.3, 0.5); // -30% to +50%
    let step_mult = attack_data.damage_mult;

    let mut final_damage = base_damage * step_mult * angle_mult * combo_mult * semantic_mult;
    let poise_damage = attack_data.poise_damage;
    let knockback = attack_data.knockback;

    let was_parried = target_parrying;
    let was_blocked = target_blocking && !was_parried;

    if was_parried {
        // Parried: no damage dealt, reflected stagger
        final_damage = 0.0;
    } else if was_blocked {
        final_damage *= BLOCK_DAMAGE_MULT;
    }

    DamageCalcResult {
        base_damage,
        angle_mult,
        combo_mult,
        semantic_mult,
        final_damage,
        poise_damage,
        knockback,
        angle,
        was_critical: false, // TODO: crit system
        was_blocked,
        was_parried,
    }
}

// ============================================================================
// Combat Energy System
// ============================================================================

/// Energy resources used for abilities and special attacks
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct CombatEnergy {
    /// Kinetic energy: generated by movement and attacks
    pub kinetic: f32,
    pub kinetic_max: f32,
    /// Thermal energy: generated by elemental interactions
    pub thermal: f32,
    pub thermal_max: f32,
    /// Semantic energy: generated by semantic tag interactions
    pub semantic: f32,
    pub semantic_max: f32,
}

impl Default for CombatEnergy {
    fn default() -> Self {
        Self {
            kinetic: 0.0,
            kinetic_max: 100.0,
            thermal: 0.0,
            thermal_max: 100.0,
            semantic: 0.0,
            semantic_max: 100.0,
        }
    }
}

impl CombatEnergy {
    /// Generate kinetic energy from dealing damage
    pub fn gain_kinetic(&mut self, amount: f32) {
        self.kinetic = (self.kinetic + amount).min(self.kinetic_max);
    }

    /// Generate thermal energy from elemental attacks
    pub fn gain_thermal(&mut self, amount: f32) {
        self.thermal = (self.thermal + amount).min(self.thermal_max);
    }

    /// Generate semantic energy from semantic interactions
    pub fn gain_semantic(&mut self, amount: f32) {
        self.semantic = (self.semantic + amount).min(self.semantic_max);
    }

    /// Try to spend energy, returns true if sufficient
    pub fn spend(&mut self, kinetic: f32, thermal: f32, semantic: f32) -> bool {
        if self.kinetic >= kinetic && self.thermal >= thermal && self.semantic >= semantic {
            self.kinetic -= kinetic;
            self.thermal -= thermal;
            self.semantic -= semantic;
            true
        } else {
            false
        }
    }
}

// ============================================================================
// Hitbox Validation (server-side, no physics engine required)
// ============================================================================

/// Check if a point is within the attack hitbox (oriented bounding box)
pub fn point_in_hitbox(
    attacker_pos: Vec3,
    attacker_facing: f32,
    attack_data: &AttackData,
    target_pos: Vec3,
) -> bool {
    // Transform target position into attacker's local space
    let dir = target_pos - attacker_pos;
    let forward = Vec3::new(attacker_facing.cos(), 0.0, attacker_facing.sin());
    let right = Vec3::new(-attacker_facing.sin(), 0.0, attacker_facing.cos());

    // Local coordinates relative to attacker
    let local_z = dir.dot(forward) - attack_data.hitbox_offset;
    let local_x = dir.dot(right);
    let local_y = dir.y;

    // Check if within hitbox extents
    let hx = attack_data.hitbox_half_extents[0];
    let hy = attack_data.hitbox_half_extents[1];
    let hz = attack_data.hitbox_half_extents[2];

    if local_x.abs() > hx || local_y.abs() > hy || local_z.abs() > hz {
        return false;
    }

    // Check angular sweep
    if attack_data.sweep_angle < std::f32::consts::TAU - 0.01 {
        let angle_to_target = local_x.atan2(local_z + attack_data.hitbox_offset);
        if angle_to_target.abs() > attack_data.sweep_angle / 2.0 {
            return false;
        }
    }

    true
}

// ============================================================================
// Bevy Systems
// ============================================================================

/// System: Update combat state timers every tick
pub fn update_combat_timers(
    time: Res<Time>,
    mut combatants: Query<&mut CombatState>,
) {
    let dt = time.delta_secs();

    for mut state in &mut combatants {
        // Tick down phase timer
        if state.phase_timer > 0.0 {
            state.phase_timer -= dt;
            if state.phase_timer <= 0.0 {
                state.phase_timer = 0.0;
                // Phase ended — transition based on current phase
                match state.phase {
                    CombatPhase::Attacking => {
                        // Attack finished → open combo window
                        state.phase = CombatPhase::Idle;
                        state.combo_window = COMBO_WINDOW_SECS;
                    }
                    CombatPhase::Dodging => {
                        state.phase = CombatPhase::Idle;
                    }
                    CombatPhase::Parrying => {
                        // Parry window closed → back to blocking or idle
                        state.phase = CombatPhase::Idle;
                    }
                    CombatPhase::Staggered => {
                        state.phase = CombatPhase::Idle;
                    }
                    _ => {}
                }
            }
        }

        // Tick down combo window
        if state.combo_window > 0.0 {
            state.combo_window -= dt;
            if state.combo_window <= 0.0 {
                state.combo_window = 0.0;
                state.combo_step = 0; // Combo dropped
            }
        }

        // Tick down i-frames
        if state.i_frames > 0.0 {
            state.i_frames -= dt;
            if state.i_frames <= 0.0 {
                state.i_frames = 0.0;
            }
        }

        // Tick down parry window
        if state.parry_window > 0.0 {
            state.parry_window -= dt;
            if state.parry_window <= 0.0 {
                state.parry_window = 0.0;
            }
        }

        // Regenerate poise (only when idle or blocking)
        if matches!(state.phase, CombatPhase::Idle | CombatPhase::Blocking) {
            state.poise = (state.poise + state.poise_regen * dt).min(state.max_poise);
        }
    }
}

/// Validate and apply a combat action to an entity's CombatState.
/// Returns a description of what happened.
pub fn try_combat_action(
    state: &mut CombatState,
    action: ActionType,
    weapon: &EquippedWeapon,
    movesets: &WeaponMovesets,
) -> CombatActionResult {
    match action {
        ActionType::Attack => try_attack(state, weapon, movesets),
        ActionType::Block => try_block(state),
        ActionType::BlockRelease => try_block_release(state),
        ActionType::Parry => try_parry(state),
        ActionType::Dodge => try_dodge(state),
        ActionType::HeavyAttack => try_heavy_attack(state, weapon, movesets),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatActionResult {
    pub success: bool,
    pub action: ActionType,
    pub new_phase: CombatPhase,
    pub combo_step: u8,
    /// Attack data if an attack was started
    pub attack_data: Option<AttackData>,
    pub message: String,
}

fn try_attack(
    state: &mut CombatState,
    weapon: &EquippedWeapon,
    movesets: &WeaponMovesets,
) -> CombatActionResult {
    // Can only attack from Idle
    if !matches!(state.phase, CombatPhase::Idle) {
        return CombatActionResult {
            success: false,
            action: ActionType::Attack,
            new_phase: state.phase,
            combo_step: state.combo_step,
            attack_data: None,
            message: "Cannot attack in current state".into(),
        };
    }

    let moveset = match movesets.movesets.get(&weapon.weapon_type) {
        Some(m) => m,
        None => return CombatActionResult {
            success: false,
            action: ActionType::Attack,
            new_phase: state.phase,
            combo_step: state.combo_step,
            attack_data: None,
            message: "No moveset for weapon".into(),
        },
    };

    // Determine combo step
    let step = if state.combo_window > 0.0 {
        // Continuing combo
        let next = state.combo_step + 1;
        if next as usize >= moveset.len() {
            0 // Wrap around to first attack
        } else {
            next
        }
    } else {
        0 // Fresh attack
    };

    let attack = &moveset[step as usize];
    let total_duration = (attack.windup + attack.active + attack.recovery) / weapon.attack_speed;

    state.phase = CombatPhase::Attacking;
    state.phase_timer = total_duration;
    state.combo_step = step;
    state.combo_window = 0.0; // Will be set when attack ends

    CombatActionResult {
        success: true,
        action: ActionType::Attack,
        new_phase: CombatPhase::Attacking,
        combo_step: step,
        attack_data: Some(attack.clone()),
        message: format!("Attack combo step {}", step),
    }
}

fn try_block(state: &mut CombatState) -> CombatActionResult {
    if !matches!(state.phase, CombatPhase::Idle) {
        return CombatActionResult {
            success: false,
            action: ActionType::Block,
            new_phase: state.phase,
            combo_step: state.combo_step,
            attack_data: None,
            message: "Cannot block in current state".into(),
        };
    }

    state.phase = CombatPhase::Blocking;
    state.combo_step = 0;
    state.combo_window = 0.0;

    CombatActionResult {
        success: true,
        action: ActionType::Block,
        new_phase: CombatPhase::Blocking,
        combo_step: 0,
        attack_data: None,
        message: "Blocking".into(),
    }
}

fn try_block_release(state: &mut CombatState) -> CombatActionResult {
    if state.phase == CombatPhase::Blocking {
        state.phase = CombatPhase::Idle;
    }

    CombatActionResult {
        success: true,
        action: ActionType::BlockRelease,
        new_phase: state.phase,
        combo_step: state.combo_step,
        attack_data: None,
        message: "Block released".into(),
    }
}

fn try_parry(state: &mut CombatState) -> CombatActionResult {
    // Parry can be initiated from Idle or Blocking
    if !matches!(state.phase, CombatPhase::Idle | CombatPhase::Blocking) {
        return CombatActionResult {
            success: false,
            action: ActionType::Parry,
            new_phase: state.phase,
            combo_step: state.combo_step,
            attack_data: None,
            message: "Cannot parry in current state".into(),
        };
    }

    state.phase = CombatPhase::Parrying;
    state.phase_timer = PARRY_WINDOW_SECS;
    state.parry_window = PARRY_WINDOW_SECS;
    state.combo_step = 0;
    state.combo_window = 0.0;

    CombatActionResult {
        success: true,
        action: ActionType::Parry,
        new_phase: CombatPhase::Parrying,
        combo_step: 0,
        attack_data: None,
        message: "Parry window open".into(),
    }
}

fn try_dodge(state: &mut CombatState) -> CombatActionResult {
    // Can dodge from Idle, Blocking, or during combo window
    if !matches!(state.phase, CombatPhase::Idle | CombatPhase::Blocking) {
        return CombatActionResult {
            success: false,
            action: ActionType::Dodge,
            new_phase: state.phase,
            combo_step: state.combo_step,
            attack_data: None,
            message: "Cannot dodge in current state".into(),
        };
    }

    state.phase = CombatPhase::Dodging;
    state.phase_timer = DODGE_DURATION_SECS;
    state.i_frames = DODGE_IFRAMES_SECS;
    state.combo_step = 0;
    state.combo_window = 0.0;

    CombatActionResult {
        success: true,
        action: ActionType::Dodge,
        new_phase: CombatPhase::Dodging,
        combo_step: 0,
        attack_data: None,
        message: "Dodge roll".into(),
    }
}

fn try_heavy_attack(
    state: &mut CombatState,
    weapon: &EquippedWeapon,
    movesets: &WeaponMovesets,
) -> CombatActionResult {
    if !matches!(state.phase, CombatPhase::Idle) {
        return CombatActionResult {
            success: false,
            action: ActionType::HeavyAttack,
            new_phase: state.phase,
            combo_step: state.combo_step,
            attack_data: None,
            message: "Cannot heavy attack in current state".into(),
        };
    }

    let moveset = match movesets.movesets.get(&weapon.weapon_type) {
        Some(m) => m,
        None => return CombatActionResult {
            success: false,
            action: ActionType::HeavyAttack,
            new_phase: state.phase,
            combo_step: state.combo_step,
            attack_data: None,
            message: "No moveset for weapon".into(),
        },
    };

    // Heavy attack uses the last (finisher) attack with extended windup
    let finisher = &moveset[moveset.len() - 1];
    let mut heavy = finisher.clone();
    heavy.windup *= 1.5; // Longer windup
    heavy.damage_mult *= 1.3; // More damage
    heavy.poise_damage *= 1.5; // More stagger

    let total_duration = (heavy.windup + heavy.active + heavy.recovery) / weapon.attack_speed;

    state.phase = CombatPhase::Attacking;
    state.phase_timer = total_duration;
    state.combo_step = 0; // Heavy attack resets combo
    state.combo_window = 0.0;

    CombatActionResult {
        success: true,
        action: ActionType::HeavyAttack,
        new_phase: CombatPhase::Attacking,
        combo_step: 0,
        attack_data: Some(heavy),
        message: "Heavy attack".into(),
    }
}

/// Apply damage to a target's combat state (poise, stagger, death)
pub fn apply_damage_to_target(
    target_state: &mut CombatState,
    target_health: &mut f32,
    damage_result: &DamageCalcResult,
) -> DamageOutcome {
    if damage_result.was_parried {
        return DamageOutcome::Parried;
    }

    // Apply damage
    *target_health -= damage_result.final_damage;

    // Check death
    if *target_health <= 0.0 {
        *target_health = 0.0;
        target_state.phase = CombatPhase::Dead;
        target_state.phase_timer = 0.0;
        return DamageOutcome::Killed;
    }

    // Apply poise damage
    let poise_damage = if damage_result.was_blocked {
        BLOCK_POISE_COST
    } else {
        damage_result.poise_damage
    };

    target_state.poise -= poise_damage;

    // Check stagger
    if target_state.poise <= 0.0 {
        target_state.poise = 0.0;
        target_state.phase = CombatPhase::Staggered;
        target_state.phase_timer = STAGGER_DURATION_SECS;
        return DamageOutcome::Staggered;
    }

    if damage_result.was_blocked {
        DamageOutcome::Blocked
    } else {
        DamageOutcome::Hit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageOutcome {
    Hit,
    Blocked,
    Parried,
    Staggered,
    Killed,
}

// ============================================================================
// Mastery XP Rewards
// ============================================================================

/// Calculate mastery XP reward for a combat action
pub fn mastery_xp_for_action(action: ActionType, outcome: DamageOutcome) -> (String, f32) {
    match (action, outcome) {
        (ActionType::Attack, DamageOutcome::Hit) => ("WeaponMastery".into(), 10.0),
        (ActionType::Attack, DamageOutcome::Killed) => ("WeaponMastery".into(), 50.0),
        (ActionType::HeavyAttack, DamageOutcome::Hit) => ("WeaponMastery".into(), 15.0),
        (ActionType::HeavyAttack, DamageOutcome::Staggered) => ("WeaponMastery".into(), 30.0),
        (ActionType::HeavyAttack, DamageOutcome::Killed) => ("WeaponMastery".into(), 75.0),
        (ActionType::Parry, DamageOutcome::Parried) => ("ParryMastery".into(), 50.0),
        (ActionType::Dodge, _) => ("DodgeMastery".into(), 30.0),
        (ActionType::Block, DamageOutcome::Blocked) => ("BlockMastery".into(), 15.0),
        _ => ("CombatMastery".into(), 5.0),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_sword() -> EquippedWeapon {
        EquippedWeapon {
            weapon_type: WeaponType::Sword,
            weapon_id: "test_sword".into(),
            base_damage: 50.0,
            attack_speed: 1.0,
            range: 2.0,
        }
    }

    #[test]
    fn test_combat_state_default() {
        let state = CombatState::default();
        assert_eq!(state.phase, CombatPhase::Idle);
        assert_eq!(state.combo_step, 0);
        assert_eq!(state.poise, 100.0);
    }

    #[test]
    fn test_attack_from_idle() {
        let mut state = CombatState::default();
        let weapon = test_sword();
        let movesets = WeaponMovesets::default();

        let result = try_combat_action(&mut state, ActionType::Attack, &weapon, &movesets);
        assert!(result.success);
        assert_eq!(result.new_phase, CombatPhase::Attacking);
        assert_eq!(result.combo_step, 0);
        assert!(result.attack_data.is_some());
    }

    #[test]
    fn test_cannot_attack_while_attacking() {
        let mut state = CombatState::default();
        let weapon = test_sword();
        let movesets = WeaponMovesets::default();

        // First attack succeeds
        let _ = try_combat_action(&mut state, ActionType::Attack, &weapon, &movesets);
        assert_eq!(state.phase, CombatPhase::Attacking);

        // Second attack fails (still in attacking phase)
        let result = try_combat_action(&mut state, ActionType::Attack, &weapon, &movesets);
        assert!(!result.success);
    }

    #[test]
    fn test_combo_chain() {
        let mut state = CombatState::default();
        let weapon = test_sword();
        let movesets = WeaponMovesets::default();

        // Attack step 0
        let r1 = try_combat_action(&mut state, ActionType::Attack, &weapon, &movesets);
        assert_eq!(r1.combo_step, 0);

        // Simulate attack finishing → idle with combo window
        state.phase = CombatPhase::Idle;
        state.combo_window = COMBO_WINDOW_SECS;

        // Attack step 1
        let r2 = try_combat_action(&mut state, ActionType::Attack, &weapon, &movesets);
        assert!(r2.success);
        assert_eq!(r2.combo_step, 1);

        // Step 2
        state.phase = CombatPhase::Idle;
        state.combo_window = COMBO_WINDOW_SECS;
        let r3 = try_combat_action(&mut state, ActionType::Attack, &weapon, &movesets);
        assert_eq!(r3.combo_step, 2);
    }

    #[test]
    fn test_combo_drops_without_window() {
        let mut state = CombatState::default();
        let weapon = test_sword();
        let movesets = WeaponMovesets::default();

        // Attack step 0
        let _ = try_combat_action(&mut state, ActionType::Attack, &weapon, &movesets);

        // Simulate attack ending but combo window expired
        state.phase = CombatPhase::Idle;
        state.combo_window = 0.0;
        state.combo_step = 2; // Was at step 2

        // Should reset to step 0
        let result = try_combat_action(&mut state, ActionType::Attack, &weapon, &movesets);
        assert_eq!(result.combo_step, 0);
    }

    #[test]
    fn test_parry_timing_window() {
        let mut state = CombatState::default();
        let weapon = test_sword();
        let movesets = WeaponMovesets::default();

        let result = try_combat_action(&mut state, ActionType::Parry, &weapon, &movesets);
        assert!(result.success);
        assert_eq!(state.phase, CombatPhase::Parrying);
        assert!((state.parry_window - PARRY_WINDOW_SECS).abs() < 0.001);
        assert!((state.phase_timer - PARRY_WINDOW_SECS).abs() < 0.001);
    }

    #[test]
    fn test_dodge_iframes() {
        let mut state = CombatState::default();
        let weapon = test_sword();
        let movesets = WeaponMovesets::default();

        let result = try_combat_action(&mut state, ActionType::Dodge, &weapon, &movesets);
        assert!(result.success);
        assert_eq!(state.phase, CombatPhase::Dodging);
        assert!((state.i_frames - DODGE_IFRAMES_SECS).abs() < 0.001);
        assert!((state.phase_timer - DODGE_DURATION_SECS).abs() < 0.001);
    }

    #[test]
    fn test_block() {
        let mut state = CombatState::default();
        let weapon = test_sword();
        let movesets = WeaponMovesets::default();

        let result = try_combat_action(&mut state, ActionType::Block, &weapon, &movesets);
        assert!(result.success);
        assert_eq!(state.phase, CombatPhase::Blocking);

        // Release block
        let result = try_combat_action(&mut state, ActionType::BlockRelease, &weapon, &movesets);
        assert!(result.success);
        assert_eq!(state.phase, CombatPhase::Idle);
    }

    #[test]
    fn test_damage_calculation_front() {
        let movesets = WeaponMovesets::default();
        let attack = &movesets.movesets[&WeaponType::Sword][0];

        let result = calculate_damage(50.0, attack, AttackAngle::Front, 0, 0.0, false, false);
        assert_eq!(result.angle_mult, 1.0);
        assert_eq!(result.combo_mult, 1.0);
        assert_eq!(result.semantic_mult, 1.0);
        // 50 * 1.0 (step) * 1.0 (angle) * 1.0 (combo) * 1.0 (semantic) = 50
        assert!((result.final_damage - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_damage_calculation_back_combo() {
        let movesets = WeaponMovesets::default();
        let attack = &movesets.movesets[&WeaponType::Sword][2]; // combo step 2

        let result = calculate_damage(50.0, attack, AttackAngle::Back, 2, 0.2, false, false);
        // 50 * 1.3 (step 2 mult) * 1.4 (back) * 1.3 (combo) * 1.2 (semantic +0.2)
        let expected = 50.0 * 1.3 * 1.4 * 1.3 * 1.2;
        assert!((result.final_damage - expected).abs() < 0.1,
            "Expected {}, got {}", expected, result.final_damage);
    }

    #[test]
    fn test_damage_blocked() {
        let movesets = WeaponMovesets::default();
        let attack = &movesets.movesets[&WeaponType::Sword][0];

        let result = calculate_damage(50.0, attack, AttackAngle::Front, 0, 0.0, true, false);
        assert!(result.was_blocked);
        assert!(!result.was_parried);
        assert!((result.final_damage - 50.0 * BLOCK_DAMAGE_MULT).abs() < 0.01);
    }

    #[test]
    fn test_damage_parried() {
        let movesets = WeaponMovesets::default();
        let attack = &movesets.movesets[&WeaponType::Sword][0];

        let result = calculate_damage(50.0, attack, AttackAngle::Front, 0, 0.0, false, true);
        assert!(result.was_parried);
        assert_eq!(result.final_damage, 0.0);
    }

    #[test]
    fn test_apply_damage_kill() {
        let mut state = CombatState::default();
        let mut health = 20.0;

        let movesets = WeaponMovesets::default();
        let attack = &movesets.movesets[&WeaponType::Hammer][1];
        let damage = calculate_damage(100.0, attack, AttackAngle::Back, 0, 0.0, false, false);

        let outcome = apply_damage_to_target(&mut state, &mut health, &damage);
        assert_eq!(outcome, DamageOutcome::Killed);
        assert_eq!(state.phase, CombatPhase::Dead);
        assert_eq!(health, 0.0);
    }

    #[test]
    fn test_poise_break_stagger() {
        let mut state = CombatState { poise: 10.0, ..Default::default() };
        let mut health = 500.0;

        let movesets = WeaponMovesets::default();
        let attack = &movesets.movesets[&WeaponType::Hammer][0]; // 45 poise damage

        let damage = calculate_damage(50.0, attack, AttackAngle::Front, 0, 0.0, false, false);
        let outcome = apply_damage_to_target(&mut state, &mut health, &damage);
        assert_eq!(outcome, DamageOutcome::Staggered);
        assert_eq!(state.phase, CombatPhase::Staggered);
    }

    #[test]
    fn test_attack_angle_calculation() {
        // Attacker in front of target (target facing attacker)
        let angle = AttackAngle::calculate(
            Vec3::new(0.0, 0.0, 5.0),  // attacker
            0.0,                         // attacker facing
            Vec3::ZERO,                  // target
            std::f32::consts::FRAC_PI_2, // target facing +Z
        );
        assert_eq!(angle, AttackAngle::Front);

        // Attacker behind target
        let angle = AttackAngle::calculate(
            Vec3::new(0.0, 0.0, -5.0), // attacker behind
            0.0,
            Vec3::ZERO,
            std::f32::consts::FRAC_PI_2, // target facing +Z
        );
        assert_eq!(angle, AttackAngle::Back);
    }

    #[test]
    fn test_hitbox_validation() {
        let movesets = WeaponMovesets::default();
        let attack = &movesets.movesets[&WeaponType::Sword][0];

        // Target directly in front, within range
        assert!(point_in_hitbox(
            Vec3::ZERO,
            0.0, // facing +X
            attack,
            Vec3::new(2.0, 0.0, 0.0),
        ));

        // Target behind attacker
        assert!(!point_in_hitbox(
            Vec3::ZERO,
            0.0,
            attack,
            Vec3::new(-5.0, 0.0, 0.0),
        ));

        // Target too far
        assert!(!point_in_hitbox(
            Vec3::ZERO,
            0.0,
            attack,
            Vec3::new(20.0, 0.0, 0.0),
        ));
    }

    #[test]
    fn test_weapon_movesets_all_present() {
        let movesets = WeaponMovesets::default();
        assert_eq!(movesets.movesets[&WeaponType::Sword].len(), 4);
        assert_eq!(movesets.movesets[&WeaponType::Spear].len(), 3);
        assert_eq!(movesets.movesets[&WeaponType::Hammer].len(), 2);
    }

    #[test]
    fn test_combat_energy() {
        let mut energy = CombatEnergy::default();
        energy.gain_kinetic(30.0);
        assert_eq!(energy.kinetic, 30.0);

        // Exceed max
        energy.gain_kinetic(200.0);
        assert_eq!(energy.kinetic, 100.0);

        // Spend
        assert!(energy.spend(50.0, 0.0, 0.0));
        assert_eq!(energy.kinetic, 50.0);

        // Insufficient
        assert!(!energy.spend(60.0, 0.0, 0.0));
        assert_eq!(energy.kinetic, 50.0); // Unchanged
    }

    #[test]
    fn test_mastery_xp_rewards() {
        let (domain, xp) = mastery_xp_for_action(ActionType::Parry, DamageOutcome::Parried);
        assert_eq!(domain, "ParryMastery");
        assert_eq!(xp, 50.0);

        let (domain, xp) = mastery_xp_for_action(ActionType::Attack, DamageOutcome::Killed);
        assert_eq!(domain, "WeaponMastery");
        assert_eq!(xp, 50.0);
    }

    #[test]
    fn test_heavy_attack() {
        let mut state = CombatState::default();
        let weapon = test_sword();
        let movesets = WeaponMovesets::default();

        let result = try_combat_action(&mut state, ActionType::HeavyAttack, &weapon, &movesets);
        assert!(result.success);
        assert_eq!(result.new_phase, CombatPhase::Attacking);
        assert_eq!(result.combo_step, 0); // Heavy resets combo

        // Heavy attack uses boosted finisher
        let attack = result.attack_data.unwrap();
        let finisher = &movesets.movesets[&WeaponType::Sword][3];
        assert!(attack.damage_mult > finisher.damage_mult); // Should be 1.3x more
    }

    #[test]
    fn test_spear_narrow_hitbox() {
        let movesets = WeaponMovesets::default();
        let attack = &movesets.movesets[&WeaponType::Spear][0];

        // Spear should hit straight ahead at long range
        assert!(point_in_hitbox(
            Vec3::ZERO,
            0.0,
            attack,
            Vec3::new(3.0, 0.0, 0.0),
        ));

        // Spear should NOT hit at wide angles
        assert!(!point_in_hitbox(
            Vec3::ZERO,
            0.0,
            attack,
            Vec3::new(1.0, 0.0, 3.0), // Far to the side
        ));
    }

    #[test]
    fn test_parry_from_blocking() {
        let mut state = CombatState::default();
        let weapon = test_sword();
        let movesets = WeaponMovesets::default();

        // Enter blocking
        let _ = try_combat_action(&mut state, ActionType::Block, &weapon, &movesets);
        assert_eq!(state.phase, CombatPhase::Blocking);

        // Parry from blocking should work
        let result = try_combat_action(&mut state, ActionType::Parry, &weapon, &movesets);
        assert!(result.success);
        assert_eq!(state.phase, CombatPhase::Parrying);
    }
}
