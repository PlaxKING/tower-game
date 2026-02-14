//! Status effects and buff/debuff system.
//!
//! Effects are driven by semantic tags — fire attacks apply Burning,
//! corruption attacks apply Corruption, etc.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Status effect types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatusType {
    // Damage over time
    Burning,  // fire: damage per tick
    Poisoned, // corruption: damage + reduced healing
    Bleeding, // physical: damage on movement

    // Crowd control
    Stunned,  // can't act
    Frozen,   // water: can't move, take extra damage
    Silenced, // can't use abilities

    // Debuffs
    Weakened,  // reduced damage output
    Slowed,    // reduced move speed
    Exposed,   // increased damage taken
    Corrupted, // semantic tags distorted

    // Buffs
    Empowered,     // increased damage
    Hastened,      // increased move speed
    Shielded,      // damage absorption
    Regenerating,  // heal over time
    SemanticFocus, // enhanced semantic abilities
}

/// A single status effect instance
#[derive(Debug, Clone)]
pub struct StatusEffect {
    pub effect_type: StatusType,
    pub remaining: f32, // seconds remaining
    pub strength: f32,  // effect intensity (0.0 - 1.0)
    pub source: Option<Entity>,
    pub stacks: u32, // stackable effects
    pub max_stacks: u32,
}

impl StatusEffect {
    pub fn new(effect_type: StatusType, duration: f32, strength: f32) -> Self {
        let max_stacks = match effect_type {
            StatusType::Burning | StatusType::Bleeding | StatusType::Poisoned => 5,
            StatusType::Corrupted => 3,
            _ => 1,
        };

        Self {
            effect_type,
            remaining: duration,
            strength,
            source: None,
            stacks: 1,
            max_stacks,
        }
    }

    pub fn with_source(mut self, source: Entity) -> Self {
        self.source = Some(source);
        self
    }

    pub fn is_expired(&self) -> bool {
        self.remaining <= 0.0
    }

    pub fn is_debuff(&self) -> bool {
        matches!(
            self.effect_type,
            StatusType::Burning
                | StatusType::Poisoned
                | StatusType::Bleeding
                | StatusType::Stunned
                | StatusType::Frozen
                | StatusType::Silenced
                | StatusType::Weakened
                | StatusType::Slowed
                | StatusType::Exposed
                | StatusType::Corrupted
        )
    }

    /// Damage per second for DoT effects
    pub fn dps(&self) -> f32 {
        match self.effect_type {
            StatusType::Burning => 8.0 * self.strength * self.stacks as f32,
            StatusType::Poisoned => 5.0 * self.strength * self.stacks as f32,
            StatusType::Bleeding => 3.0 * self.strength * self.stacks as f32,
            _ => 0.0,
        }
    }

    /// Movement speed modifier (1.0 = normal)
    pub fn speed_modifier(&self) -> f32 {
        match self.effect_type {
            StatusType::Slowed => 1.0 - 0.3 * self.strength,
            StatusType::Frozen => 0.0,
            StatusType::Hastened => 1.0 + 0.3 * self.strength,
            _ => 1.0,
        }
    }

    /// Damage dealt modifier
    pub fn damage_dealt_modifier(&self) -> f32 {
        match self.effect_type {
            StatusType::Weakened => 1.0 - 0.25 * self.strength,
            StatusType::Empowered => 1.0 + 0.3 * self.strength,
            _ => 1.0,
        }
    }

    /// Damage taken modifier
    pub fn damage_taken_modifier(&self) -> f32 {
        match self.effect_type {
            StatusType::Exposed => 1.0 + 0.3 * self.strength,
            StatusType::Frozen => 1.0 + 0.2 * self.strength,
            StatusType::Shielded => 1.0 - 0.5 * self.strength,
            _ => 1.0,
        }
    }
}

/// Component: list of active status effects on an entity
#[derive(Component, Debug, Default)]
pub struct StatusEffects {
    pub effects: Vec<StatusEffect>,
}

impl StatusEffects {
    /// Apply a new status effect, stacking if applicable
    pub fn apply(&mut self, effect: StatusEffect) {
        if let Some(existing) = self
            .effects
            .iter_mut()
            .find(|e| e.effect_type == effect.effect_type)
        {
            // Refresh duration
            existing.remaining = existing.remaining.max(effect.remaining);
            // Add stacks
            if existing.stacks < existing.max_stacks {
                existing.stacks += 1;
                existing.strength = existing.strength.max(effect.strength);
            }
        } else {
            self.effects.push(effect);
        }
    }

    /// Remove all effects of a type
    pub fn cleanse(&mut self, effect_type: StatusType) {
        self.effects.retain(|e| e.effect_type != effect_type);
    }

    /// Remove all debuffs
    pub fn cleanse_debuffs(&mut self) {
        self.effects.retain(|e| !e.is_debuff());
    }

    /// Check if entity has a specific effect
    pub fn has(&self, effect_type: StatusType) -> bool {
        self.effects.iter().any(|e| e.effect_type == effect_type)
    }

    /// Can the entity act? (not stunned/frozen)
    pub fn can_act(&self) -> bool {
        !self.has(StatusType::Stunned) && !self.has(StatusType::Frozen)
    }

    /// Can the entity use abilities?
    pub fn can_use_abilities(&self) -> bool {
        self.can_act() && !self.has(StatusType::Silenced)
    }

    /// Aggregate speed modifier from all effects
    pub fn total_speed_modifier(&self) -> f32 {
        self.effects.iter().map(|e| e.speed_modifier()).product()
    }

    /// Aggregate damage dealt modifier
    pub fn total_damage_dealt_modifier(&self) -> f32 {
        self.effects
            .iter()
            .map(|e| e.damage_dealt_modifier())
            .product()
    }

    /// Aggregate damage taken modifier
    pub fn total_damage_taken_modifier(&self) -> f32 {
        self.effects
            .iter()
            .map(|e| e.damage_taken_modifier())
            .product()
    }

    /// Total heal over time per second
    pub fn total_hot(&self) -> f32 {
        self.effects
            .iter()
            .filter(|e| e.effect_type == StatusType::Regenerating)
            .map(|e| 10.0 * e.strength)
            .sum()
    }

    /// Total damage over time per second
    pub fn total_dot(&self) -> f32 {
        self.effects.iter().map(|e| e.dps()).sum()
    }
}

/// System: tick status effect timers and apply DoT/HoT
pub fn tick_status_effects(
    time: Res<Time>,
    mut query: Query<(&mut StatusEffects, &mut super::hitbox::Health)>,
) {
    let dt = time.delta_secs();

    for (mut statuses, mut health) in &mut query {
        // Apply damage over time
        let dot = statuses.total_dot();
        if dot > 0.0 {
            let dot_damage = dot * dt;
            health.current = (health.current - dot_damage).max(0.0);
        }

        // Apply healing over time
        let hot = statuses.total_hot();
        if hot > 0.0 {
            let heal = hot * dt;
            health.current = (health.current + heal).min(health.max);
        }

        // Tick timers and remove expired
        for effect in &mut statuses.effects {
            effect.remaining -= dt;
        }
        statuses.effects.retain(|e| !e.is_expired());
    }
}

/// Determine status effect from semantic tags of the attacker
pub fn status_from_element(element: &str, strength: f32) -> Option<StatusEffect> {
    match element {
        "fire" => Some(StatusEffect::new(StatusType::Burning, 5.0, strength)),
        "water" => Some(StatusEffect::new(StatusType::Frozen, 2.0, strength * 0.5)),
        "corruption" => Some(StatusEffect::new(StatusType::Corrupted, 8.0, strength)),
        "void" => Some(StatusEffect::new(StatusType::Poisoned, 6.0, strength)),
        "wind" => Some(StatusEffect::new(StatusType::Slowed, 3.0, strength * 0.3)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_effect_stacking() {
        let mut statuses = StatusEffects::default();

        statuses.apply(StatusEffect::new(StatusType::Burning, 5.0, 0.5));
        statuses.apply(StatusEffect::new(StatusType::Burning, 3.0, 0.7));

        assert_eq!(statuses.effects.len(), 1, "Same type should stack");
        let burn = &statuses.effects[0];
        assert_eq!(burn.stacks, 2);
        assert!(
            (burn.remaining - 5.0).abs() < 0.01,
            "Should keep longer duration"
        );
        assert!(
            (burn.strength - 0.7).abs() < 0.01,
            "Should keep higher strength"
        );
    }

    #[test]
    fn test_can_act() {
        let mut statuses = StatusEffects::default();
        assert!(statuses.can_act());

        statuses.apply(StatusEffect::new(StatusType::Stunned, 2.0, 1.0));
        assert!(!statuses.can_act());
    }

    #[test]
    fn test_cleanse_debuffs() {
        let mut statuses = StatusEffects::default();
        statuses.apply(StatusEffect::new(StatusType::Burning, 5.0, 0.5));
        statuses.apply(StatusEffect::new(StatusType::Empowered, 10.0, 1.0));

        statuses.cleanse_debuffs();

        assert!(
            !statuses.has(StatusType::Burning),
            "Debuff should be cleansed"
        );
        assert!(statuses.has(StatusType::Empowered), "Buff should remain");
    }

    #[test]
    fn test_speed_modifiers() {
        let mut statuses = StatusEffects::default();
        statuses.apply(StatusEffect::new(StatusType::Slowed, 5.0, 1.0));

        let speed = statuses.total_speed_modifier();
        assert!(speed < 1.0, "Slowed should reduce speed");
        assert!(speed > 0.5, "Slowed shouldn't immobilize");
    }

    #[test]
    fn test_damage_modifiers() {
        let mut statuses = StatusEffects::default();
        statuses.apply(StatusEffect::new(StatusType::Empowered, 5.0, 1.0));
        statuses.apply(StatusEffect::new(StatusType::Exposed, 5.0, 1.0));

        assert!(statuses.total_damage_dealt_modifier() > 1.0);
        assert!(statuses.total_damage_taken_modifier() > 1.0);
    }

    #[test]
    fn test_dot_damage() {
        let mut statuses = StatusEffects::default();
        statuses.apply(StatusEffect::new(StatusType::Burning, 5.0, 1.0));

        let dot = statuses.total_dot();
        assert!(dot > 0.0, "Burning should do damage over time");
    }

    #[test]
    fn test_status_from_fire() {
        let effect = status_from_element("fire", 0.8);
        assert!(effect.is_some());
        let e = effect.unwrap();
        assert_eq!(e.effect_type, StatusType::Burning);
    }

    #[test]
    fn test_status_from_unknown() {
        let effect = status_from_element("earth", 0.5);
        assert!(effect.is_none());
    }
}
