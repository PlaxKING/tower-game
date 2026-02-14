//! Socket & Gem System
//!
//! From ddopensource.txt Category 4:
//! Equipment has sockets for gems/runes that add effects.
//! Sockets are created by Armorsmith specialization or found on rare drops.
//! Gems are crafted or found. Runes provide unique named effects.
//!
//! Socket types: Offensive (red), Defensive (blue), Utility (yellow), Prismatic (any)
//! Gems provide stat bonuses. Runes provide equipment-like effects.

use serde::{Deserialize, Serialize};

/// Socket color — determines what can be inserted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SocketColor {
    Red,       // Offensive — damage, crit
    Blue,      // Defensive — HP, defense, resistance
    Yellow,    // Utility — speed, resource, cooldown
    Prismatic, // Accepts any gem/rune
}

impl SocketColor {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Red => "Offensive",
            Self::Blue => "Defensive",
            Self::Yellow => "Utility",
            Self::Prismatic => "Prismatic",
        }
    }

    pub fn accepts(&self, gem_color: SocketColor) -> bool {
        match self {
            Self::Prismatic => true,
            other => *other == gem_color,
        }
    }
}

/// A socket on equipment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Socket {
    pub color: SocketColor,
    pub content: Option<SocketContent>,
}

impl Socket {
    pub fn empty(color: SocketColor) -> Self {
        Self {
            color,
            content: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.content.is_none()
    }

    /// Insert gem/rune into socket. Returns previous content if any.
    pub fn insert(&mut self, content: SocketContent) -> Result<Option<SocketContent>, SocketError> {
        let content_color = content.color();
        if !self.color.accepts(content_color) {
            return Err(SocketError::ColorMismatch {
                socket: self.color,
                content: content_color,
            });
        }
        let prev = self.content.take();
        self.content = Some(content);
        Ok(prev)
    }

    /// Remove content from socket (destroying or returning it)
    pub fn remove(&mut self) -> Option<SocketContent> {
        self.content.take()
    }
}

/// What goes into a socket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SocketContent {
    Gem(Gem),
    Rune(Rune),
}

impl SocketContent {
    pub fn color(&self) -> SocketColor {
        match self {
            Self::Gem(g) => g.color,
            Self::Rune(r) => r.color,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Gem(g) => &g.name,
            Self::Rune(r) => &r.name,
        }
    }
}

/// Gem — provides stat bonuses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gem {
    pub id: String,
    pub name: String,
    pub color: SocketColor,
    pub tier: GemTier,
    pub bonus: GemBonus,
}

/// Gem quality tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GemTier {
    Chipped,  // +1
    Flawed,   // +2
    Regular,  // +3
    Flawless, // +5
    Perfect,  // +8
    Radiant,  // +12
}

impl GemTier {
    pub fn multiplier(&self) -> f32 {
        match self {
            Self::Chipped => 1.0,
            Self::Flawed => 2.0,
            Self::Regular => 3.0,
            Self::Flawless => 5.0,
            Self::Perfect => 8.0,
            Self::Radiant => 12.0,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::Chipped => "Chipped",
            Self::Flawed => "Flawed",
            Self::Regular => "Regular",
            Self::Flawless => "Flawless",
            Self::Perfect => "Perfect",
            Self::Radiant => "Radiant",
        }
    }

    /// Combine 3 gems of same tier to upgrade
    pub fn next_tier(&self) -> Option<Self> {
        match self {
            Self::Chipped => Some(Self::Flawed),
            Self::Flawed => Some(Self::Regular),
            Self::Regular => Some(Self::Flawless),
            Self::Flawless => Some(Self::Perfect),
            Self::Perfect => Some(Self::Radiant),
            Self::Radiant => None,
        }
    }
}

/// Stat bonus from a gem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GemBonus {
    // Red gems
    AttackPower(f32),
    CriticalChance(f32),
    ElementalDamage { element: String, amount: f32 },
    // Blue gems
    MaxHp(f32),
    Defense(f32),
    ElementalResist { element: String, amount: f32 },
    // Yellow gems
    CooldownReduction(f32),
    ResourceRegen(f32),
    MovementSpeed(f32),
}

impl GemBonus {
    pub fn scaled(&self, tier: GemTier) -> GemBonus {
        let mult = tier.multiplier();
        match self {
            Self::AttackPower(v) => Self::AttackPower(v * mult),
            Self::CriticalChance(v) => Self::CriticalChance(v * mult),
            Self::ElementalDamage { element, amount } => Self::ElementalDamage {
                element: element.clone(),
                amount: amount * mult,
            },
            Self::MaxHp(v) => Self::MaxHp(v * mult),
            Self::Defense(v) => Self::Defense(v * mult),
            Self::ElementalResist { element, amount } => Self::ElementalResist {
                element: element.clone(),
                amount: amount * mult,
            },
            Self::CooldownReduction(v) => Self::CooldownReduction(v * mult),
            Self::ResourceRegen(v) => Self::ResourceRegen(v * mult),
            Self::MovementSpeed(v) => Self::MovementSpeed(v * mult),
        }
    }
}

/// Rune — provides named equipment-like effects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rune {
    pub id: String,
    pub name: String,
    pub color: SocketColor,
    pub description: String,
    pub effect: RuneEffect,
}

/// Effect granted by a rune
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuneEffect {
    /// On-hit proc effect
    OnHitProc {
        proc_name: String,
        chance: f32,
        damage: f32,
    },
    /// Damage absorption shield on hit received
    OnHitShield { amount: f32, cooldown: f32 },
    /// Resource recovery on kill
    OnKillRestore { resource: String, amount: f32 },
    /// Movement trail that damages enemies
    MovementTrail { damage: f32, element: String },
    /// Lifesteal on critical hits
    CritLifesteal { percent: f32 },
    /// Periodic AoE pulse
    AuraPulse {
        radius: f32,
        damage: f32,
        interval: f32,
    },
    /// Chance to not consume ability resources
    ResourcePreservation { chance: f32 },
    /// Bonus damage to low-HP targets
    ExecuteDamage { threshold: f32, bonus_percent: f32 },
}

/// Equipment socket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocketedEquipment {
    pub equipment_id: String,
    pub sockets: Vec<Socket>,
}

impl SocketedEquipment {
    pub fn new(equipment_id: String, socket_colors: Vec<SocketColor>) -> Self {
        Self {
            equipment_id,
            sockets: socket_colors.into_iter().map(Socket::empty).collect(),
        }
    }

    /// Insert content into a specific socket
    pub fn insert_at(
        &mut self,
        index: usize,
        content: SocketContent,
    ) -> Result<Option<SocketContent>, SocketError> {
        if index >= self.sockets.len() {
            return Err(SocketError::InvalidSlot(index));
        }
        self.sockets[index].insert(content)
    }

    /// Remove content from a socket
    pub fn remove_at(&mut self, index: usize) -> Option<SocketContent> {
        self.sockets.get_mut(index).and_then(|s| s.remove())
    }

    /// Count filled sockets
    pub fn filled_count(&self) -> usize {
        self.sockets.iter().filter(|s| !s.is_empty()).count()
    }

    /// Count total sockets
    pub fn socket_count(&self) -> usize {
        self.sockets.len()
    }

    /// Get all active gem bonuses
    pub fn gem_bonuses(&self) -> Vec<GemBonus> {
        self.sockets
            .iter()
            .filter_map(|s| s.content.as_ref())
            .filter_map(|c| match c {
                SocketContent::Gem(g) => Some(g.bonus.scaled(g.tier)),
                _ => None,
            })
            .collect()
    }

    /// Get all active rune effects
    pub fn rune_effects(&self) -> Vec<&RuneEffect> {
        self.sockets
            .iter()
            .filter_map(|s| s.content.as_ref())
            .filter_map(|c| match c {
                SocketContent::Rune(r) => Some(&r.effect),
                _ => None,
            })
            .collect()
    }

    /// Add a socket (from armorsmith specialization)
    pub fn add_socket(&mut self, color: SocketColor) -> bool {
        if self.sockets.len() >= 4 {
            return false; // max 4 sockets per item
        }
        self.sockets.push(Socket::empty(color));
        true
    }
}

/// Combine 3 gems of same type+tier into next tier
pub fn combine_gems(gems: &[Gem; 3]) -> Option<Gem> {
    // All must be same color, tier, and bonus type
    let first = &gems[0];
    if !gems
        .iter()
        .all(|g| g.color == first.color && g.tier == first.tier)
    {
        return None;
    }

    let next_tier = first.tier.next_tier()?;

    Some(Gem {
        id: format!(
            "{}_{}",
            first.id.split('_').next().unwrap_or("gem"),
            next_tier.display_name().to_lowercase()
        ),
        name: format!(
            "{} {}",
            next_tier.display_name(),
            first.name.split_whitespace().last().unwrap_or("Gem")
        ),
        color: first.color,
        tier: next_tier,
        bonus: first.bonus.clone(),
    })
}

/// Predefined gems
pub fn starter_gems() -> Vec<Gem> {
    vec![
        Gem {
            id: "ruby_chipped".into(),
            name: "Chipped Ruby".into(),
            color: SocketColor::Red,
            tier: GemTier::Chipped,
            bonus: GemBonus::AttackPower(2.0),
        },
        Gem {
            id: "sapphire_chipped".into(),
            name: "Chipped Sapphire".into(),
            color: SocketColor::Blue,
            tier: GemTier::Chipped,
            bonus: GemBonus::MaxHp(15.0),
        },
        Gem {
            id: "topaz_chipped".into(),
            name: "Chipped Topaz".into(),
            color: SocketColor::Yellow,
            tier: GemTier::Chipped,
            bonus: GemBonus::CooldownReduction(0.01),
        },
        Gem {
            id: "emerald_chipped".into(),
            name: "Chipped Emerald".into(),
            color: SocketColor::Red,
            tier: GemTier::Chipped,
            bonus: GemBonus::CriticalChance(0.005),
        },
        Gem {
            id: "diamond_regular".into(),
            name: "Regular Diamond".into(),
            color: SocketColor::Blue,
            tier: GemTier::Regular,
            bonus: GemBonus::Defense(3.0),
        },
    ]
}

/// Predefined runes
pub fn starter_runes() -> Vec<Rune> {
    vec![
        Rune {
            id: "rune_ember".into(),
            name: "Rune of Embers".into(),
            color: SocketColor::Red,
            description: "20% chance on hit to ignite target for fire damage.".into(),
            effect: RuneEffect::OnHitProc {
                proc_name: "Ignite".into(),
                chance: 0.20,
                damage: 25.0,
            },
        },
        Rune {
            id: "rune_aegis".into(),
            name: "Rune of Aegis".into(),
            color: SocketColor::Blue,
            description: "When hit, gain a shield absorbing 50 damage. 10s cooldown.".into(),
            effect: RuneEffect::OnHitShield {
                amount: 50.0,
                cooldown: 10.0,
            },
        },
        Rune {
            id: "rune_harvest".into(),
            name: "Rune of Harvest".into(),
            color: SocketColor::Yellow,
            description: "On kill, restore 10% kinetic energy.".into(),
            effect: RuneEffect::OnKillRestore {
                resource: "kinetic".into(),
                amount: 0.10,
            },
        },
        Rune {
            id: "rune_executioner".into(),
            name: "Rune of Execution".into(),
            color: SocketColor::Red,
            description: "Deal 25% bonus damage to enemies below 30% HP.".into(),
            effect: RuneEffect::ExecuteDamage {
                threshold: 0.30,
                bonus_percent: 0.25,
            },
        },
    ]
}

#[derive(Debug, Clone)]
pub enum SocketError {
    ColorMismatch {
        socket: SocketColor,
        content: SocketColor,
    },
    InvalidSlot(usize),
}

/// Bevy plugin stub
pub struct SocketsPlugin;
impl bevy::prelude::Plugin for SocketsPlugin {
    fn build(&self, _app: &mut bevy::prelude::App) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_color_accepts() {
        assert!(SocketColor::Red.accepts(SocketColor::Red));
        assert!(!SocketColor::Red.accepts(SocketColor::Blue));
        assert!(SocketColor::Prismatic.accepts(SocketColor::Red));
        assert!(SocketColor::Prismatic.accepts(SocketColor::Blue));
        assert!(SocketColor::Prismatic.accepts(SocketColor::Yellow));
    }

    #[test]
    fn test_insert_gem_correct_color() {
        let mut socket = Socket::empty(SocketColor::Red);
        let gem = Gem {
            id: "ruby".into(),
            name: "Ruby".into(),
            color: SocketColor::Red,
            tier: GemTier::Regular,
            bonus: GemBonus::AttackPower(5.0),
        };
        let result = socket.insert(SocketContent::Gem(gem));
        assert!(result.is_ok());
        assert!(!socket.is_empty());
    }

    #[test]
    fn test_insert_gem_wrong_color() {
        let mut socket = Socket::empty(SocketColor::Red);
        let gem = Gem {
            id: "sapphire".into(),
            name: "Sapphire".into(),
            color: SocketColor::Blue,
            tier: GemTier::Regular,
            bonus: GemBonus::MaxHp(20.0),
        };
        let result = socket.insert(SocketContent::Gem(gem));
        assert!(result.is_err());
    }

    #[test]
    fn test_prismatic_accepts_all() {
        let mut socket = Socket::empty(SocketColor::Prismatic);
        let gem = Gem {
            id: "ruby".into(),
            name: "Ruby".into(),
            color: SocketColor::Red,
            tier: GemTier::Chipped,
            bonus: GemBonus::AttackPower(2.0),
        };
        assert!(socket.insert(SocketContent::Gem(gem)).is_ok());
    }

    #[test]
    fn test_replace_returns_old() {
        let mut socket = Socket::empty(SocketColor::Red);
        let gem1 = Gem {
            id: "ruby1".into(),
            name: "Ruby 1".into(),
            color: SocketColor::Red,
            tier: GemTier::Chipped,
            bonus: GemBonus::AttackPower(2.0),
        };
        let gem2 = Gem {
            id: "ruby2".into(),
            name: "Ruby 2".into(),
            color: SocketColor::Red,
            tier: GemTier::Regular,
            bonus: GemBonus::AttackPower(5.0),
        };

        socket.insert(SocketContent::Gem(gem1)).unwrap();
        let prev = socket.insert(SocketContent::Gem(gem2)).unwrap();
        assert!(prev.is_some());
        assert_eq!(prev.unwrap().display_name(), "Ruby 1");
    }

    #[test]
    fn test_socketed_equipment() {
        let mut equip = SocketedEquipment::new(
            "test_chest".into(),
            vec![SocketColor::Red, SocketColor::Blue, SocketColor::Prismatic],
        );

        assert_eq!(equip.socket_count(), 3);
        assert_eq!(equip.filled_count(), 0);

        let gem = Gem {
            id: "ruby".into(),
            name: "Ruby".into(),
            color: SocketColor::Red,
            tier: GemTier::Regular,
            bonus: GemBonus::AttackPower(5.0),
        };
        equip.insert_at(0, SocketContent::Gem(gem)).unwrap();
        assert_eq!(equip.filled_count(), 1);
    }

    #[test]
    fn test_gem_bonuses_collection() {
        let mut equip =
            SocketedEquipment::new("test".into(), vec![SocketColor::Red, SocketColor::Blue]);

        let ruby = Gem {
            id: "ruby".into(),
            name: "Ruby".into(),
            color: SocketColor::Red,
            tier: GemTier::Regular,
            bonus: GemBonus::AttackPower(2.0),
        };
        let sapphire = Gem {
            id: "sapphire".into(),
            name: "Sapphire".into(),
            color: SocketColor::Blue,
            tier: GemTier::Flawless,
            bonus: GemBonus::MaxHp(10.0),
        };

        equip.insert_at(0, SocketContent::Gem(ruby)).unwrap();
        equip.insert_at(1, SocketContent::Gem(sapphire)).unwrap();

        let bonuses = equip.gem_bonuses();
        assert_eq!(bonuses.len(), 2);
    }

    #[test]
    fn test_rune_effects_collection() {
        let mut equip = SocketedEquipment::new("test".into(), vec![SocketColor::Red]);

        let rune = Rune {
            id: "ember".into(),
            name: "Ember Rune".into(),
            color: SocketColor::Red,
            description: "Fire proc".into(),
            effect: RuneEffect::OnHitProc {
                proc_name: "Ignite".into(),
                chance: 0.20,
                damage: 25.0,
            },
        };
        equip.insert_at(0, SocketContent::Rune(rune)).unwrap();

        let effects = equip.rune_effects();
        assert_eq!(effects.len(), 1);
    }

    #[test]
    fn test_gem_tier_combine() {
        let gems = [
            Gem {
                id: "ruby_c".into(),
                name: "Chipped Ruby".into(),
                color: SocketColor::Red,
                tier: GemTier::Chipped,
                bonus: GemBonus::AttackPower(2.0),
            },
            Gem {
                id: "ruby_c".into(),
                name: "Chipped Ruby".into(),
                color: SocketColor::Red,
                tier: GemTier::Chipped,
                bonus: GemBonus::AttackPower(2.0),
            },
            Gem {
                id: "ruby_c".into(),
                name: "Chipped Ruby".into(),
                color: SocketColor::Red,
                tier: GemTier::Chipped,
                bonus: GemBonus::AttackPower(2.0),
            },
        ];

        let result = combine_gems(&gems);
        assert!(result.is_some());
        let upgraded = result.unwrap();
        assert_eq!(upgraded.tier, GemTier::Flawed);
    }

    #[test]
    fn test_cannot_combine_mixed_tiers() {
        let gems = [
            Gem {
                id: "a".into(),
                name: "A".into(),
                color: SocketColor::Red,
                tier: GemTier::Chipped,
                bonus: GemBonus::AttackPower(2.0),
            },
            Gem {
                id: "b".into(),
                name: "B".into(),
                color: SocketColor::Red,
                tier: GemTier::Regular,
                bonus: GemBonus::AttackPower(5.0),
            }, // different tier
            Gem {
                id: "c".into(),
                name: "C".into(),
                color: SocketColor::Red,
                tier: GemTier::Chipped,
                bonus: GemBonus::AttackPower(2.0),
            },
        ];

        assert!(combine_gems(&gems).is_none());
    }

    #[test]
    fn test_max_4_sockets() {
        let mut equip = SocketedEquipment::new(
            "test".into(),
            vec![
                SocketColor::Red,
                SocketColor::Blue,
                SocketColor::Yellow,
                SocketColor::Prismatic,
            ],
        );
        assert!(!equip.add_socket(SocketColor::Red)); // already 4
    }

    #[test]
    fn test_add_socket() {
        let mut equip = SocketedEquipment::new("test".into(), vec![SocketColor::Red]);
        assert_eq!(equip.socket_count(), 1);
        assert!(equip.add_socket(SocketColor::Blue));
        assert_eq!(equip.socket_count(), 2);
    }

    #[test]
    fn test_gem_tier_scaling() {
        let base = GemBonus::AttackPower(2.0);
        match base.scaled(GemTier::Perfect) {
            GemBonus::AttackPower(v) => assert!((v - 16.0).abs() < 0.01), // 2.0 * 8.0
            _ => panic!("Expected AttackPower"),
        }
    }

    #[test]
    fn test_starter_gems_and_runes() {
        let gems = starter_gems();
        assert!(gems.len() >= 5);

        let runes = starter_runes();
        assert!(runes.len() >= 4);
    }
}
