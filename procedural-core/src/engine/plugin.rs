use bevy::prelude::*;
use std::sync::{Arc, RwLock};

use crate::engine::config::EngineConfig;
use crate::engine::hybrid::HybridEngine;

pub struct EnginePlugin;

impl Plugin for EnginePlugin {
    fn build(&self, app: &mut App) {
        let config = EngineConfig::default();
        let engine = HybridEngine::new(config);

        app.insert_resource(EngineResource(Arc::new(RwLock::new(engine))))
            .add_systems(Update, engine_tick_system);
    }
}

#[derive(Resource)]
pub struct EngineResource(pub Arc<RwLock<HybridEngine>>);

fn engine_tick_system(time: Res<Time>, engine_res: Res<EngineResource>) {
    if let Ok(mut engine) = engine_res.0.write() {
        engine.tick(time.delta_secs());
    }
}
