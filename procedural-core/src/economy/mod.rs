use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod crafting;

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(MarketState::default())
            .add_systems(Update, update_market_prices);
    }
}

/// Player inventory and currency
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct Wallet {
    pub tower_shards: u64,   // primary currency
    pub echo_fragments: u64, // rare currency from echoes
    pub faction_tokens: FactionTokens,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactionTokens {
    pub ascending_order: u32,
    pub deep_dwellers: u32,
    pub echo_keepers: u32,
    pub free_climbers: u32,
}

/// Item rarity tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ItemRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Mythic, // only from Echelon 4+
}

impl ItemRarity {
    /// Base price multiplier for this rarity
    pub fn price_multiplier(&self) -> f32 {
        match self {
            Self::Common => 1.0,
            Self::Uncommon => 3.0,
            Self::Rare => 10.0,
            Self::Epic => 50.0,
            Self::Legendary => 250.0,
            Self::Mythic => 1000.0,
        }
    }

    /// Drop chance weight (lower = rarer)
    pub fn drop_weight(&self) -> f32 {
        match self {
            Self::Common => 100.0,
            Self::Uncommon => 40.0,
            Self::Rare => 10.0,
            Self::Epic => 2.0,
            Self::Legendary => 0.3,
            Self::Mythic => 0.02,
        }
    }
}

/// Tradeable item definition
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct TradeItem {
    pub name: String,
    pub rarity: ItemRarity,
    pub base_price: u64,
    pub stack_size: u32,
    pub max_stack: u32,
    pub soulbound: bool, // cannot be traded
}

/// Market state - dynamic pricing based on supply/demand
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct MarketState {
    pub inflation_rate: f32,                // global price modifier
    pub demand_factors: Vec<(String, f32)>, // item_name -> demand multiplier
    pub last_update_cycle: u32,             // breath cycle when last updated
}

impl Default for MarketState {
    fn default() -> Self {
        Self {
            inflation_rate: 1.0,
            demand_factors: Vec::new(),
            last_update_cycle: 0,
        }
    }
}

impl MarketState {
    /// Calculate current price for an item
    pub fn current_price(&self, item: &TradeItem) -> u64 {
        let demand = self
            .demand_factors
            .iter()
            .find(|(name, _)| name == &item.name)
            .map(|(_, factor)| *factor)
            .unwrap_or(1.0);

        let price =
            item.base_price as f32 * item.rarity.price_multiplier() * self.inflation_rate * demand;

        price.max(1.0) as u64
    }
}

/// Trade offer between players
#[derive(Component, Debug)]
pub struct TradeOffer {
    pub seller: Entity,
    pub buyer: Option<Entity>,
    pub items_offered: Vec<Entity>, // item entities
    pub price: u64,
    pub expires_in: f32, // seconds
}

fn update_market_prices(time: Res<Time>, mut market: ResMut<MarketState>) {
    // Slowly normalize demand factors toward 1.0
    let dt = time.delta_secs();
    for (_, factor) in &mut market.demand_factors {
        let diff = 1.0 - *factor;
        *factor += diff * 0.01 * dt; // very slow normalization
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rarity_ordering() {
        assert!(ItemRarity::Common < ItemRarity::Mythic);
        assert!(ItemRarity::Rare < ItemRarity::Legendary);
    }

    #[test]
    fn test_price_multiplier_scaling() {
        assert!(ItemRarity::Mythic.price_multiplier() > ItemRarity::Legendary.price_multiplier());
        assert!((ItemRarity::Common.price_multiplier() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_market_price_calculation() {
        let market = MarketState::default();
        let item = TradeItem {
            name: "Iron Sword".into(),
            rarity: ItemRarity::Common,
            base_price: 100,
            stack_size: 1,
            max_stack: 1,
            soulbound: false,
        };
        // Common item, no demand modifier, inflation 1.0
        // 100 * 1.0 * 1.0 * 1.0 = 100
        assert_eq!(market.current_price(&item), 100);
    }

    #[test]
    fn test_market_price_with_rarity() {
        let market = MarketState::default();
        let item = TradeItem {
            name: "Fire Blade".into(),
            rarity: ItemRarity::Epic,
            base_price: 100,
            stack_size: 1,
            max_stack: 1,
            soulbound: false,
        };
        // 100 * 50.0 * 1.0 * 1.0 = 5000
        assert_eq!(market.current_price(&item), 5000);
    }

    #[test]
    fn test_wallet_default() {
        let wallet = Wallet::default();
        assert_eq!(wallet.tower_shards, 0);
        assert_eq!(wallet.echo_fragments, 0);
    }
}
