use crate::constants::*;
use crate::engine::config::EngineConfig;
use crate::engine::helpers::tile_type_to_u8;
use crate::engine::messages::{
    FloorLayoutMsg, FloorResponseMsg, LootItemMsg, MonsterGrammarMsg, MonsterSpawnMsg, RoomDataMsg,
    SemanticTagMsg, Vec3Msg,
};
use crate::generation::wfc;
use crate::generation::{FloorSpec, TowerSeed};
use crate::loot;
use crate::monster::MonsterTemplate;
use crate::semantic::SemanticTags;

/// GenerationService — procedural content generation
pub struct GenerationService {
    tower_seed: TowerSeed,
}

impl GenerationService {
    pub fn new(config: &EngineConfig) -> Self {
        Self {
            tower_seed: TowerSeed {
                seed: config.tower_seed,
            },
        }
    }

    pub fn generate_floor(&self, floor_id: u32) -> FloorResponseMsg {
        let spec = FloorSpec::generate(&self.tower_seed, floor_id);
        let layout = wfc::generate_layout(&spec);

        let tile_nums: Vec<Vec<u8>> = layout
            .tiles
            .iter()
            .map(|row| row.iter().map(tile_type_to_u8).collect())
            .collect();

        let rooms: Vec<RoomDataMsg> = layout
            .rooms
            .iter()
            .enumerate()
            .map(|(i, r)| RoomDataMsg {
                room_id: i as u32,
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
                room_type: format!("{:?}", r.room_type),
            })
            .collect();

        let base_hash = self.tower_seed.floor_hash(floor_id);
        let monster_count = BASE_MONSTER_COUNT + (floor_id % MONSTER_COUNT_MOD) as u64;
        let monsters: Vec<MonsterSpawnMsg> = (0..monster_count)
            .map(|i| {
                let hash = base_hash.wrapping_add(i * MONSTER_HASH_PRIME);
                let template = MonsterTemplate::from_hash(hash, floor_id);
                let stats = template.compute_stats();
                let tags = template.semantic_tags();

                MonsterSpawnMsg {
                    entity_id: hash,
                    monster_type: template.name.clone(),
                    position: Vec3Msg {
                        x: (hash % 100) as f32,
                        y: 0.0,
                        z: ((hash / 100) % 100) as f32,
                    },
                    tags: tags.tags.iter().map(SemanticTagMsg::from).collect(),
                    health: stats.max_hp,
                    max_health: stats.max_hp,
                    grammar: MonsterGrammarMsg {
                        body_type: format!("{:?}", template.size),
                        locomotion: format!("{:?}", template.behavior),
                        attack_style: format!("{:?}", template.element),
                        modifiers: vec![format!("{:?}", template.corruption)],
                    },
                }
            })
            .collect();

        FloorResponseMsg {
            floor_id: spec.id,
            floor_hash: spec.hash,
            biome_tags: spec
                .biome_tags
                .tags
                .iter()
                .map(SemanticTagMsg::from)
                .collect(),
            tier: format!("{:?}", spec.tier),
            layout: FloorLayoutMsg {
                width: layout.width,
                height: layout.height,
                tiles: tile_nums,
                rooms,
            },
            monsters,
        }
    }

    pub fn generate_loot(
        &self,
        source_tags: &[(String, f32)],
        floor_level: u32,
        drop_hash: u64,
    ) -> Vec<LootItemMsg> {
        let tags = SemanticTags {
            tags: source_tags.to_vec(),
        };
        let items = loot::generate_loot(&tags, floor_level, drop_hash);

        items
            .iter()
            .map(|item| LootItemMsg {
                item_name: item.name.clone(),
                rarity: format!("{:?}", item.rarity),
                tags: item
                    .semantic_tags
                    .iter()
                    .map(SemanticTagMsg::from)
                    .collect(),
                socket_count: 0,
            })
            .collect()
    }

    pub fn query_semantic_similarity(
        &self,
        tags_a: &[(String, f32)],
        tags_b: &[(String, f32)],
    ) -> f32 {
        let a = SemanticTags {
            tags: tags_a.to_vec(),
        };
        let b = SemanticTags {
            tags: tags_b.to_vec(),
        };
        a.similarity(&b)
    }
}
