use crate::generation::wfc;
use crate::mastery::MasteryTier;

pub(crate) fn tile_type_to_u8(tile: &wfc::TileType) -> u8 {
    match tile {
        wfc::TileType::Empty => 0,
        wfc::TileType::Floor => 1,
        wfc::TileType::Wall => 2,
        wfc::TileType::Door => 3,
        wfc::TileType::StairsUp => 4,
        wfc::TileType::StairsDown => 5,
        wfc::TileType::Chest => 6,
        wfc::TileType::Trap => 7,
        wfc::TileType::Spawner => 8,
        wfc::TileType::Shrine => 9,
        wfc::TileType::WindColumn => 10,
        wfc::TileType::VoidPit => 11,
    }
}

pub(crate) fn tier_to_u32(tier: MasteryTier) -> u32 {
    match tier {
        MasteryTier::Novice => 0,
        MasteryTier::Apprentice => 1,
        MasteryTier::Journeyman => 2,
        MasteryTier::Expert => 3,
        MasteryTier::Master => 4,
        MasteryTier::Grandmaster => 5,
    }
}
