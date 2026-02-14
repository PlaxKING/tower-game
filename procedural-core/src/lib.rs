//! Tower Game - Procedural Core Library
//!
//! This crate provides the deterministic game logic for the Tower MMORPG:
//! - Semantic tag system (entity descriptions via float vectors)
//! - Procedural generation (seed + delta model, WFC floor layouts)
//! - Combat system (timing-based, angular hitboxes)
//! - Movement and aerial combat
//! - Death/echo system
//! - World cycle (Breath of the Tower)
//! - Faction and economy
//! - Monster generation (grammar-based)
//! - Player controller
//! - Loot system (semantic drops)
//! - FFI bridge for UE5 integration

pub mod abilities;
pub mod achievements;
pub mod aerial;
pub mod analytics;
pub mod anticheat;
pub mod balance;
pub mod bridge;
pub mod combat;
pub mod constants;
pub mod cosmetics;
pub mod death;
pub mod economy;
pub mod engine;
pub mod equipment;
pub mod events;
pub mod faction;
pub mod gameflow;
pub mod generation;
pub mod hotreload;
pub mod logging;
pub mod loot;
pub mod mastery;
pub mod monster;
pub mod movement;
pub mod mutators;
pub mod player;
pub mod replay;
pub mod replication;
pub mod savemigration;
pub mod seasons;
pub mod semantic;
pub mod social;
pub mod sockets;
pub mod specialization;
pub mod towermap;
pub mod tutorial;
pub mod visualization;
pub mod world;
