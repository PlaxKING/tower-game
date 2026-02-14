//! Centralized game constants for tower procedural core.
//!
//! Eliminates magic numbers duplicated across engine services and FFI bridge.
//! Per-module constants (monster stats, loot tables, mastery XP) remain in
//! their respective modules as the single source of truth.

// =====================================================
// Combat
// =====================================================

/// Per-combo-step damage multiplier: final = 1.0 + step * COMBO_STEP_MULT
pub const COMBO_STEP_MULT: f32 = 0.15;

/// Base critical hit chance (5%)
pub const BASE_CRIT_CHANCE: f32 = 0.05;

/// Critical damage multiplier (1.5x)
pub const CRIT_DAMAGE_MULT: f32 = 1.5;

/// Semantic similarity threshold above which attacker gets a bonus
pub const SEMANTIC_HIGH_THRESHOLD: f32 = 0.7;

/// Semantic similarity threshold below which attacker gets a penalty
pub const SEMANTIC_LOW_THRESHOLD: f32 = 0.3;

/// Damage multiplier when semantic similarity > HIGH_THRESHOLD
pub const SEMANTIC_HIGH_MULT: f32 = 1.2;

/// Damage multiplier when semantic similarity < LOW_THRESHOLD
pub const SEMANTIC_LOW_MULT: f32 = 0.9;

/// Flat semantic bonus from synergy (similarity > HIGH_THRESHOLD) in bridge combat calc
pub const SEMANTIC_SYNERGY_BONUS: f32 = 0.2;

/// Flat semantic penalty from conflict (similarity < LOW_THRESHOLD) in bridge combat calc
pub const SEMANTIC_CONFLICT_PENALTY: f32 = -0.1;

// =====================================================
// Breath Cycle (Tower Breath)
// =====================================================

/// Inhale phase duration in seconds (6 dev-mode hours)
pub const BREATH_INHALE_SECS: f32 = 360.0;

/// Hold phase duration in seconds (4 dev-mode hours)
pub const BREATH_HOLD_SECS: f32 = 240.0;

/// Exhale phase duration in seconds (6 dev-mode hours)
pub const BREATH_EXHALE_SECS: f32 = 360.0;

/// Pause phase duration in seconds (2 dev-mode hours)
pub const BREATH_PAUSE_SECS: f32 = 120.0;

/// Total breath cycle duration in seconds (18 dev-mode hours)
pub const BREATH_CYCLE_TOTAL: f32 =
    BREATH_INHALE_SECS + BREATH_HOLD_SECS + BREATH_EXHALE_SECS + BREATH_PAUSE_SECS;

// =====================================================
// Procedural Generation
// =====================================================

/// Prime number used as hash stride for monster spawning
pub const MONSTER_HASH_PRIME: u64 = 7919;

/// Base number of monsters per floor (before floor_id modifier)
pub const BASE_MONSTER_COUNT: u64 = 3;

/// Floor ID modulus for additional monster count
pub const MONSTER_COUNT_MOD: u32 = 5;
