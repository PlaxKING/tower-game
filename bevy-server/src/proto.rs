//! Auto-generated Protobuf types for all game systems
//!
//! This module includes code generated from all proto files in `shared/proto/`.
//! Types are automatically generated at compile time by prost-build.
//!
//! ## Packages
//! - `tower::game` - Core types, networking, world state
//! - `tower::entities` - Player, Monster, Item, Ability templates
//! - `tower::economy` - Crafting, trading, auctions
//! - `tower::social` - Guilds, friends, parties, chat, mail
//! - `tower::quests` - Quests, factions, seasons, achievements, PvP
//!
//! ## Usage
//! ```rust,ignore
//! use tower_bevy_server::proto::tower::game::{Vec3, ChunkData};
//! use tower_bevy_server::proto::tower::entities::{ItemTemplate, MonsterTemplate, Rarity};
//! use tower_bevy_server::proto::tower::economy::{CraftingRecipe, AuctionListing};
//! use tower_bevy_server::proto::tower::social::{Guild, Party};
//! use tower_bevy_server::proto::tower::quests::{QuestTemplate, FactionTemplate};
//! ```

// Include generated Protobuf code from build.rs
pub mod tower {
    /// Core game types: Vec3, Rotation, Velocity, ChunkData, PlayerData, networking
    pub mod game {
        #![allow(dead_code)]
        include!(concat!(env!("OUT_DIR"), "/tower.game.rs"));
    }

    /// Entity definitions: Players, Monsters, Items, Abilities, Effects, Loot
    pub mod entities {
        #![allow(dead_code)]
        include!(concat!(env!("OUT_DIR"), "/tower.entities.rs"));
    }

    /// Economy systems: Crafting, Trading, Auctions, Transactions
    pub mod economy {
        #![allow(dead_code)]
        include!(concat!(env!("OUT_DIR"), "/tower.economy.rs"));
    }

    /// Social systems: Guilds, Friends, Parties, Chat, Mail, Leaderboards
    pub mod social {
        #![allow(dead_code)]
        include!(concat!(env!("OUT_DIR"), "/tower.social.rs"));
    }

    /// Content systems: Quests, Factions, Seasons, Achievements, PvP, NPCs
    pub mod quests {
        #![allow(dead_code)]
        include!(concat!(env!("OUT_DIR"), "/tower.quests.rs"));
    }
}
