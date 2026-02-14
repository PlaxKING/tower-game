use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct AerialPlugin;

impl Plugin for AerialPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                update_flight_state,
                process_aerial_combat,
                apply_wind_currents,
            )
                .chain(),
        );
    }
}

/// Flight state for entities that can fly/glide
#[derive(Component, Debug)]
pub struct FlightState {
    pub mode: FlightMode,
    pub altitude: f32,
    pub stamina: f32,
    pub max_stamina: f32,
    pub stamina_drain_rate: f32, // per second while hovering
    pub stamina_regen_rate: f32, // per second while grounded
    pub dive_speed_bonus: f32,   // speed multiplier during dive
}

impl Default for FlightState {
    fn default() -> Self {
        Self {
            mode: FlightMode::Grounded,
            altitude: 0.0,
            stamina: 100.0,
            max_stamina: 100.0,
            stamina_drain_rate: 10.0,
            stamina_regen_rate: 20.0,
            dive_speed_bonus: 2.5,
        }
    }
}

/// Current flight mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlightMode {
    Grounded,
    Ascending, // burning stamina fast
    Hovering,  // slow stamina drain, thermal energy generation
    Gliding,   // slow descent, minimal stamina use
    Diving,    // fast descent, kinetic energy generation, attack bonus
}

/// Aerial combat modifier based on height advantage
#[derive(Component, Debug)]
pub struct AerialAdvantage {
    pub height_diff: f32,
    pub damage_modifier: f32,
}

impl AerialAdvantage {
    /// Calculate damage modifier from height difference (attacker - target)
    pub fn from_height_diff(diff: f32) -> Self {
        let modifier = if diff > 5.0 {
            1.3 // significant height advantage
        } else if diff > 2.0 {
            1.1 // minor advantage
        } else if diff < -5.0 {
            0.7 // significant disadvantage
        } else if diff < -2.0 {
            0.9 // minor disadvantage
        } else {
            1.0 // neutral
        };

        Self {
            height_diff: diff,
            damage_modifier: modifier,
        }
    }
}

/// Wind current volumes that affect flight
#[derive(Component, Debug)]
pub struct WindCurrent {
    pub direction: Vec3,
    pub strength: f32,
    pub radius: f32,
}

/// Dive attack component - activated during a dive towards an enemy
#[derive(Component, Debug)]
pub struct DiveAttack {
    pub active: bool,
    pub speed: f32,
    pub impact_damage_multiplier: f32, // scales with speed
    pub kinetic_energy_generated: f32,
}

impl Default for DiveAttack {
    fn default() -> Self {
        Self {
            active: false,
            speed: 0.0,
            impact_damage_multiplier: 1.0,
            kinetic_energy_generated: 0.0,
        }
    }
}

fn update_flight_state(time: Res<Time>, mut query: Query<(&mut FlightState, &Transform)>) {
    let dt = time.delta_secs();

    for (mut flight, transform) in &mut query {
        flight.altitude = transform.translation.y;

        match flight.mode {
            FlightMode::Grounded => {
                flight.stamina =
                    (flight.stamina + flight.stamina_regen_rate * dt).min(flight.max_stamina);
            }
            FlightMode::Ascending => {
                flight.stamina -= flight.stamina_drain_rate * 2.0 * dt;
            }
            FlightMode::Hovering => {
                flight.stamina -= flight.stamina_drain_rate * dt;
            }
            FlightMode::Gliding => {
                flight.stamina -= flight.stamina_drain_rate * 0.3 * dt;
            }
            FlightMode::Diving => {
                // Diving doesn't cost stamina, generates kinetic energy
            }
        }

        // Force land if out of stamina
        if flight.stamina <= 0.0 {
            flight.stamina = 0.0;
            if flight.mode != FlightMode::Grounded && flight.mode != FlightMode::Diving {
                flight.mode = FlightMode::Gliding; // gentle forced descent
            }
        }
    }
}

fn process_aerial_combat(mut query: Query<(&mut DiveAttack, &FlightState, &Transform)>) {
    for (mut dive, flight, transform) in &mut query {
        if flight.mode == FlightMode::Diving && dive.active {
            // Speed increases damage
            dive.speed = transform.translation.y.abs(); // simplified
            dive.impact_damage_multiplier = 1.0 + (dive.speed / 20.0).min(2.0);
            dive.kinetic_energy_generated = dive.speed * 0.5;
        }
    }
}

fn apply_wind_currents(
    winds: Query<(&WindCurrent, &GlobalTransform)>,
    mut flyers: Query<(&FlightState, &mut Transform), Without<WindCurrent>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for (flight, mut flyer_transform) in &mut flyers {
        if flight.mode == FlightMode::Grounded {
            continue;
        }

        for (wind, wind_transform) in &winds {
            let distance = flyer_transform
                .translation
                .distance(wind_transform.translation());
            if distance < wind.radius {
                let falloff = 1.0 - (distance / wind.radius);
                flyer_transform.translation += wind.direction * wind.strength * falloff * dt;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aerial_advantage_high() {
        let adv = AerialAdvantage::from_height_diff(10.0);
        assert!((adv.damage_modifier - 1.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_aerial_advantage_low() {
        let adv = AerialAdvantage::from_height_diff(-10.0);
        assert!((adv.damage_modifier - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_aerial_advantage_neutral() {
        let adv = AerialAdvantage::from_height_diff(0.5);
        assert!((adv.damage_modifier - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_flight_state_default() {
        let flight = FlightState::default();
        assert_eq!(flight.mode, FlightMode::Grounded);
        assert!((flight.stamina - 100.0).abs() < f32::EPSILON);
    }
}
