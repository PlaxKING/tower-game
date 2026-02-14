use bevy::prelude::*;

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                apply_gravity,
                process_movement_input,
                update_velocity,
                apply_dash,
            )
                .chain(),
        );
    }
}

/// Movement capabilities for any entity
#[derive(Component, Debug)]
pub struct MovementState {
    pub velocity: Vec3,
    pub grounded: bool,
    pub move_speed: f32,
    pub jump_force: f32,
    pub gravity_scale: f32,
    pub dash_cooldown: f32,
    pub dash_timer: f32,
    pub facing: Vec3,
}

impl Default for MovementState {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            grounded: false,
            move_speed: 8.0,
            jump_force: 12.0,
            gravity_scale: 1.0,
            dash_cooldown: 0.0,
            dash_timer: 0.0,
            facing: Vec3::NEG_Z,
        }
    }
}

/// Input vector from player or AI controller
#[derive(Component, Debug, Default)]
pub struct MovementInput {
    pub direction: Vec2, // normalized XZ movement
    pub jump: bool,
    pub dash: bool,
}

/// Dash ability parameters
#[derive(Component, Debug)]
pub struct DashAbility {
    pub speed: f32,
    pub duration: f32,
    pub cooldown: f32,
    pub invulnerable: bool, // i-frames during dash
}

impl Default for DashAbility {
    fn default() -> Self {
        Self {
            speed: 25.0,
            duration: 0.2,
            cooldown: 1.5,
            invulnerable: true,
        }
    }
}

const GRAVITY: f32 = -20.0;
const TERMINAL_VELOCITY: f32 = -50.0;

fn apply_gravity(time: Res<Time>, mut query: Query<&mut MovementState>) {
    let dt = time.delta_secs();
    for mut state in &mut query {
        if !state.grounded {
            state.velocity.y += GRAVITY * state.gravity_scale * dt;
            state.velocity.y = state.velocity.y.max(TERMINAL_VELOCITY);
        }
    }
}

fn process_movement_input(mut query: Query<(&mut MovementState, &MovementInput)>) {
    for (mut state, input) in &mut query {
        // Horizontal movement
        let move_dir = Vec3::new(input.direction.x, 0.0, input.direction.y);
        state.velocity.x = move_dir.x * state.move_speed;
        state.velocity.z = move_dir.z * state.move_speed;

        // Update facing
        if move_dir.length_squared() > 0.01 {
            state.facing = move_dir.normalize();
        }

        // Jump
        if input.jump && state.grounded {
            state.velocity.y = state.jump_force;
            state.grounded = false;
        }
    }
}

fn update_velocity(time: Res<Time>, mut query: Query<(&MovementState, &mut Transform)>) {
    let dt = time.delta_secs();
    for (state, mut transform) in &mut query {
        transform.translation += state.velocity * dt;

        // Simple ground plane check (y = 0)
        if transform.translation.y <= 0.0 {
            transform.translation.y = 0.0;
        }
    }
}

fn apply_dash(
    time: Res<Time>,
    mut query: Query<(&mut MovementState, &MovementInput, &DashAbility)>,
) {
    let dt = time.delta_secs();
    for (mut state, input, dash) in &mut query {
        // Cooldown tick
        state.dash_cooldown = (state.dash_cooldown - dt).max(0.0);
        state.dash_timer = (state.dash_timer - dt).max(0.0);

        // Start dash
        if input.dash && state.dash_cooldown <= 0.0 && state.dash_timer <= 0.0 {
            state.dash_timer = dash.duration;
            state.dash_cooldown = dash.cooldown;
        }

        // Apply dash velocity
        if state.dash_timer > 0.0 {
            let dash_dir = if state.facing.length_squared() > 0.01 {
                state.facing.normalize()
            } else {
                Vec3::NEG_Z
            };
            state.velocity = dash_dir * dash.speed;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_movement_state() {
        let state = MovementState::default();
        assert_eq!(state.velocity, Vec3::ZERO);
        assert!(!state.grounded);
        assert!((state.move_speed - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_dash_defaults() {
        let dash = DashAbility::default();
        assert!((dash.speed - 25.0).abs() < f32::EPSILON);
        assert!(dash.invulnerable);
    }
}
