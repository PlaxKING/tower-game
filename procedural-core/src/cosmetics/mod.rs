//! Cosmetics & Transmog System
//!
//! From ddopensource.txt Category 9:
//! Character customization, outfit presets, transmog (appearance override),
//! dye system, and cosmetic unlocks from season pass / achievements.
//!
//! Transmog separates appearance from stats — equip strong gear but look however you want.
//! Dyes let you recolor equipment. Cosmetics are purely visual, no gameplay impact.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cosmetic slot type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CosmeticSlot {
    HeadOverride,
    ChestOverride,
    LegsOverride,
    BootsOverride,
    GlovesOverride,
    WeaponSkin,
    BackAccessory,  // cape, wings, backpack
    Aura,           // visual particle effect
    Emote,          // gesture/animation
    Title,          // displayed name prefix/suffix
    ProfileBorder,  // UI frame
    NameplateStyle, // nameplate customization
}

impl CosmeticSlot {
    pub fn display_name(&self) -> &str {
        match self {
            Self::HeadOverride => "Head Appearance",
            Self::ChestOverride => "Chest Appearance",
            Self::LegsOverride => "Legs Appearance",
            Self::BootsOverride => "Boots Appearance",
            Self::GlovesOverride => "Gloves Appearance",
            Self::WeaponSkin => "Weapon Skin",
            Self::BackAccessory => "Back Accessory",
            Self::Aura => "Aura Effect",
            Self::Emote => "Emote",
            Self::Title => "Title",
            Self::ProfileBorder => "Profile Border",
            Self::NameplateStyle => "Nameplate",
        }
    }
}

/// Source of a cosmetic unlock
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CosmeticSource {
    Achievement(String),
    SeasonPass {
        season_id: String,
        level: u32,
    },
    Shop {
        price_shards: i64,
    },
    CraftingMastery {
        domain: String,
        tier: String,
    },
    EventReward(String),
    QuestReward(String),
    Drop {
        floor_range: (u32, u32),
        rarity: String,
    },
}

/// A single cosmetic item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CosmeticItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub slot: CosmeticSlot,
    pub source: CosmeticSource,
    /// Mesh/material asset reference for UE5
    pub asset_ref: String,
    /// Whether this cosmetic supports dyeing
    pub dyeable: bool,
    /// Rarity (for visual flair in UI)
    pub rarity: String,
}

/// Color dye for equipment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dye {
    pub id: String,
    pub name: String,
    pub color: DyeColor,
    pub source: CosmeticSource,
}

/// RGB + metadata for dye
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DyeColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub metallic: f32,   // 0.0 - 1.0
    pub glossiness: f32, // 0.0 - 1.0
}

impl DyeColor {
    pub fn new(r: f32, g: f32, b: f32) -> Self {
        Self {
            r,
            g,
            b,
            metallic: 0.0,
            glossiness: 0.5,
        }
    }

    pub fn metallic(mut self, m: f32) -> Self {
        self.metallic = m;
        self
    }
    pub fn glossy(mut self, g: f32) -> Self {
        self.glossiness = g;
        self
    }
}

/// Dye slot on an equipment piece
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DyeChannel {
    Primary,
    Secondary,
    Accent,
}

/// Transmog override — keeps stats of equipped gear but shows different appearance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransmogOverride {
    pub slot: CosmeticSlot,
    pub cosmetic_id: String,
    pub dyes: HashMap<DyeChannel, String>, // channel → dye_id
}

/// Character appearance settings (set at creation, some changeable later)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterAppearance {
    pub hair_style: u32,
    pub hair_color: DyeColor,
    pub face_preset: u32,
    pub skin_tone: u32,
    pub eye_color: DyeColor,
    pub body_type: u32,    // 0-3 body builds
    pub height_scale: f32, // 0.9-1.1
    pub voice_set: u32,
}

impl Default for CharacterAppearance {
    fn default() -> Self {
        Self {
            hair_style: 0,
            hair_color: DyeColor::new(0.2, 0.15, 0.1),
            face_preset: 0,
            skin_tone: 2,
            eye_color: DyeColor::new(0.3, 0.5, 0.8),
            body_type: 0,
            height_scale: 1.0,
            voice_set: 0,
        }
    }
}

/// Player's complete cosmetic profile
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CosmeticProfile {
    /// All unlocked cosmetic item IDs
    pub unlocked_cosmetics: Vec<String>,
    /// All unlocked dye IDs
    pub unlocked_dyes: Vec<String>,
    /// Currently active transmog overrides
    pub active_transmogs: HashMap<CosmeticSlot, TransmogOverride>,
    /// Currently active title
    pub active_title: Option<String>,
    /// Currently active aura
    pub active_aura: Option<String>,
    /// Currently active back accessory
    pub active_back: Option<String>,
    /// Character base appearance
    pub appearance: CharacterAppearance,
    /// Outfit presets (name → set of transmog overrides)
    pub outfit_presets: HashMap<String, Vec<TransmogOverride>>,
}

impl CosmeticProfile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Unlock a cosmetic item
    pub fn unlock_cosmetic(&mut self, cosmetic_id: &str) -> bool {
        if self.unlocked_cosmetics.contains(&cosmetic_id.to_string()) {
            return false;
        }
        self.unlocked_cosmetics.push(cosmetic_id.to_string());
        true
    }

    /// Unlock a dye
    pub fn unlock_dye(&mut self, dye_id: &str) -> bool {
        if self.unlocked_dyes.contains(&dye_id.to_string()) {
            return false;
        }
        self.unlocked_dyes.push(dye_id.to_string());
        true
    }

    /// Apply a transmog override
    pub fn apply_transmog(&mut self, slot: CosmeticSlot, cosmetic_id: &str) -> bool {
        if !self.unlocked_cosmetics.contains(&cosmetic_id.to_string()) {
            return false;
        }
        self.active_transmogs.insert(
            slot,
            TransmogOverride {
                slot,
                cosmetic_id: cosmetic_id.to_string(),
                dyes: HashMap::new(),
            },
        );
        true
    }

    /// Apply a dye to a transmog slot
    pub fn apply_dye(&mut self, slot: CosmeticSlot, channel: DyeChannel, dye_id: &str) -> bool {
        if !self.unlocked_dyes.contains(&dye_id.to_string()) {
            return false;
        }
        if let Some(transmog) = self.active_transmogs.get_mut(&slot) {
            transmog.dyes.insert(channel, dye_id.to_string());
            true
        } else {
            false
        }
    }

    /// Remove transmog override (show actual gear)
    pub fn remove_transmog(&mut self, slot: CosmeticSlot) -> bool {
        self.active_transmogs.remove(&slot).is_some()
    }

    /// Set active title
    pub fn set_title(&mut self, title_id: &str) -> bool {
        if !self.unlocked_cosmetics.contains(&title_id.to_string()) {
            return false;
        }
        self.active_title = Some(title_id.to_string());
        true
    }

    /// Save current transmog setup as an outfit preset
    pub fn save_preset(&mut self, name: &str) {
        let overrides: Vec<TransmogOverride> = self.active_transmogs.values().cloned().collect();
        self.outfit_presets.insert(name.to_string(), overrides);
    }

    /// Load an outfit preset
    pub fn load_preset(&mut self, name: &str) -> bool {
        if let Some(preset) = self.outfit_presets.get(name).cloned() {
            self.active_transmogs.clear();
            for ov in preset {
                self.active_transmogs.insert(ov.slot, ov);
            }
            true
        } else {
            false
        }
    }

    /// Count total unlocked cosmetics
    pub fn total_unlocked(&self) -> usize {
        self.unlocked_cosmetics.len() + self.unlocked_dyes.len()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Predefined cosmetics
pub fn tower_cosmetics() -> Vec<CosmeticItem> {
    vec![
        CosmeticItem {
            id: "title_first_ascent".into(),
            name: "First Ascender".into(),
            description: "Reach floor 10 for the first time.".into(),
            slot: CosmeticSlot::Title,
            source: CosmeticSource::Achievement("first_floor_10".into()),
            asset_ref: "".into(),
            dyeable: false,
            rarity: "Uncommon".into(),
        },
        CosmeticItem {
            id: "title_grandmaster".into(),
            name: "Grandmaster".into(),
            description: "Reach Grandmaster tier in any mastery.".into(),
            slot: CosmeticSlot::Title,
            source: CosmeticSource::CraftingMastery {
                domain: "any".into(),
                tier: "Grandmaster".into(),
            },
            asset_ref: "".into(),
            dyeable: false,
            rarity: "Legendary".into(),
        },
        CosmeticItem {
            id: "aura_flame".into(),
            name: "Ember Aura".into(),
            description: "Subtle fire particles orbit your character.".into(),
            slot: CosmeticSlot::Aura,
            source: CosmeticSource::SeasonPass {
                season_id: "s1".into(),
                level: 25,
            },
            asset_ref: "NS_Aura_Flame".into(),
            dyeable: false,
            rarity: "Epic".into(),
        },
        CosmeticItem {
            id: "back_wings_echo".into(),
            name: "Echo Wings".into(),
            description: "Translucent ethereal wings that pulse with tower energy.".into(),
            slot: CosmeticSlot::BackAccessory,
            source: CosmeticSource::SeasonPass {
                season_id: "s1".into(),
                level: 50,
            },
            asset_ref: "SM_Back_EchoWings".into(),
            dyeable: true,
            rarity: "Mythic".into(),
        },
        CosmeticItem {
            id: "weapon_crystal".into(),
            name: "Crystalline Edge".into(),
            description: "Weapon glows with an inner crystal light.".into(),
            slot: CosmeticSlot::WeaponSkin,
            source: CosmeticSource::Drop {
                floor_range: (30, 50),
                rarity: "Epic".into(),
            },
            asset_ref: "MI_Weapon_Crystal".into(),
            dyeable: true,
            rarity: "Epic".into(),
        },
        CosmeticItem {
            id: "border_seeker".into(),
            name: "Seeker's Frame".into(),
            description: "Profile border in the colors of the Ascending Order.".into(),
            slot: CosmeticSlot::ProfileBorder,
            source: CosmeticSource::QuestReward("seeker_allegiance".into()),
            asset_ref: "UI_Border_Seeker".into(),
            dyeable: false,
            rarity: "Rare".into(),
        },
    ]
}

/// Predefined dyes
pub fn tower_dyes() -> Vec<Dye> {
    vec![
        Dye {
            id: "dye_crimson".into(),
            name: "Crimson".into(),
            color: DyeColor::new(0.8, 0.1, 0.1),
            source: CosmeticSource::Shop { price_shards: 500 },
        },
        Dye {
            id: "dye_midnight".into(),
            name: "Midnight Blue".into(),
            color: DyeColor::new(0.05, 0.05, 0.3).glossy(0.8),
            source: CosmeticSource::Shop { price_shards: 500 },
        },
        Dye {
            id: "dye_gold".into(),
            name: "Royal Gold".into(),
            color: DyeColor::new(0.9, 0.7, 0.1).metallic(0.9).glossy(0.9),
            source: CosmeticSource::Achievement("guild_leader".into()),
        },
        Dye {
            id: "dye_void".into(),
            name: "Void Black".into(),
            color: DyeColor::new(0.02, 0.01, 0.05).metallic(0.3),
            source: CosmeticSource::Drop {
                floor_range: (40, 50),
                rarity: "Rare".into(),
            },
        },
        Dye {
            id: "dye_spring".into(),
            name: "Spring Green".into(),
            color: DyeColor::new(0.2, 0.8, 0.3),
            source: CosmeticSource::EventReward("spring_festival".into()),
        },
    ]
}

/// Bevy plugin stub
pub struct CosmeticsPlugin;
impl bevy::prelude::Plugin for CosmeticsPlugin {
    fn build(&self, _app: &mut bevy::prelude::App) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosmetic_profile_new() {
        let profile = CosmeticProfile::new();
        assert_eq!(profile.total_unlocked(), 0);
        assert!(profile.active_transmogs.is_empty());
    }

    #[test]
    fn test_unlock_cosmetic() {
        let mut profile = CosmeticProfile::new();
        assert!(profile.unlock_cosmetic("title_first_ascent"));
        assert!(!profile.unlock_cosmetic("title_first_ascent")); // duplicate
        assert_eq!(profile.unlocked_cosmetics.len(), 1);
    }

    #[test]
    fn test_apply_transmog() {
        let mut profile = CosmeticProfile::new();
        profile.unlock_cosmetic("weapon_crystal");

        assert!(profile.apply_transmog(CosmeticSlot::WeaponSkin, "weapon_crystal"));
        assert!(profile
            .active_transmogs
            .contains_key(&CosmeticSlot::WeaponSkin));
    }

    #[test]
    fn test_cannot_transmog_unlocked() {
        let mut profile = CosmeticProfile::new();
        assert!(!profile.apply_transmog(CosmeticSlot::WeaponSkin, "weapon_crystal"));
    }

    #[test]
    fn test_dye_application() {
        let mut profile = CosmeticProfile::new();
        profile.unlock_cosmetic("weapon_crystal");
        profile.unlock_dye("dye_crimson");

        profile.apply_transmog(CosmeticSlot::WeaponSkin, "weapon_crystal");
        assert!(profile.apply_dye(CosmeticSlot::WeaponSkin, DyeChannel::Primary, "dye_crimson"));

        let transmog = profile
            .active_transmogs
            .get(&CosmeticSlot::WeaponSkin)
            .unwrap();
        assert!(transmog.dyes.contains_key(&DyeChannel::Primary));
    }

    #[test]
    fn test_remove_transmog() {
        let mut profile = CosmeticProfile::new();
        profile.unlock_cosmetic("weapon_crystal");
        profile.apply_transmog(CosmeticSlot::WeaponSkin, "weapon_crystal");

        assert!(profile.remove_transmog(CosmeticSlot::WeaponSkin));
        assert!(!profile
            .active_transmogs
            .contains_key(&CosmeticSlot::WeaponSkin));
    }

    #[test]
    fn test_outfit_preset() {
        let mut profile = CosmeticProfile::new();
        profile.unlock_cosmetic("weapon_crystal");
        profile.unlock_cosmetic("aura_flame");

        profile.apply_transmog(CosmeticSlot::WeaponSkin, "weapon_crystal");
        profile.apply_transmog(CosmeticSlot::Aura, "aura_flame");

        profile.save_preset("Battle Look");
        assert!(profile.outfit_presets.contains_key("Battle Look"));

        // Clear and reload
        profile.active_transmogs.clear();
        assert!(profile.load_preset("Battle Look"));
        assert_eq!(profile.active_transmogs.len(), 2);
    }

    #[test]
    fn test_set_title() {
        let mut profile = CosmeticProfile::new();
        profile.unlock_cosmetic("title_first_ascent");

        assert!(profile.set_title("title_first_ascent"));
        assert_eq!(profile.active_title, Some("title_first_ascent".to_string()));
    }

    #[test]
    fn test_tower_cosmetics() {
        let cosmetics = tower_cosmetics();
        assert!(cosmetics.len() >= 6);
        assert!(cosmetics.iter().any(|c| c.slot == CosmeticSlot::Title));
        assert!(cosmetics.iter().any(|c| c.slot == CosmeticSlot::Aura));
        assert!(cosmetics
            .iter()
            .any(|c| c.slot == CosmeticSlot::BackAccessory));
    }

    #[test]
    fn test_tower_dyes() {
        let dyes = tower_dyes();
        assert!(dyes.len() >= 5);
    }

    #[test]
    fn test_dye_color_builder() {
        let color = DyeColor::new(1.0, 0.0, 0.0).metallic(0.8).glossy(0.9);
        assert_eq!(color.r, 1.0);
        assert_eq!(color.metallic, 0.8);
        assert_eq!(color.glossiness, 0.9);
    }

    #[test]
    fn test_cosmetic_json() {
        let mut profile = CosmeticProfile::new();
        profile.unlock_cosmetic("test_cosmetic");
        let json = profile.to_json();
        assert!(!json.is_empty());
        assert!(json.contains("test_cosmetic"));
    }

    #[test]
    fn test_character_appearance_default() {
        let appearance = CharacterAppearance::default();
        assert_eq!(appearance.height_scale, 1.0);
        assert_eq!(appearance.body_type, 0);
    }
}
