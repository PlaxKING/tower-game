use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::engine::config::EngineConfig;
use crate::engine::messages::LootItemMsg;

/// EconomyService — trading, crafting, economy operations
pub struct EconomyService {
    wallets: HashMap<u64, WalletState>,
}

#[derive(Debug, Clone, Default)]
pub struct WalletState {
    pub gold: u64,
    pub premium: u64,
    pub seasonal: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMsg {
    pub gold: u64,
    pub premium_currency: u64,
    pub seasonal_currency: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeResultMsg {
    pub success: bool,
    pub failure_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftResultMsg {
    pub success: bool,
    pub failure_reason: String,
    pub crafted_item: Option<LootItemMsg>,
    pub mastery_xp_gained: f32,
}

impl EconomyService {
    pub fn new(_config: &EngineConfig) -> Self {
        Self {
            wallets: HashMap::new(),
        }
    }

    pub fn get_wallet(&self, player_id: u64) -> WalletMsg {
        let default = WalletState::default();
        let wallet = self.wallets.get(&player_id).unwrap_or(&default);
        WalletMsg {
            gold: wallet.gold,
            premium_currency: wallet.premium,
            seasonal_currency: wallet.seasonal,
        }
    }

    pub fn add_gold(&mut self, player_id: u64, amount: u64) {
        let wallet = self.wallets.entry(player_id).or_default();
        wallet.gold += amount;
    }

    pub fn try_spend_gold(&mut self, player_id: u64, amount: u64) -> bool {
        let wallet = self.wallets.entry(player_id).or_default();
        if wallet.gold >= amount {
            wallet.gold -= amount;
            true
        } else {
            false
        }
    }

    pub fn trade(
        &mut self,
        player_a: u64,
        player_b: u64,
        gold_from_a: u64,
        gold_from_b: u64,
    ) -> TradeResultMsg {
        let wallet_a = self.wallets.entry(player_a).or_default().clone();
        let wallet_b = self.wallets.entry(player_b).or_default().clone();

        if wallet_a.gold < gold_from_a {
            return TradeResultMsg {
                success: false,
                failure_reason: "Player A has insufficient gold".into(),
            };
        }
        if wallet_b.gold < gold_from_b {
            return TradeResultMsg {
                success: false,
                failure_reason: "Player B has insufficient gold".into(),
            };
        }

        let wa = self.wallets.get_mut(&player_a).unwrap();
        wa.gold -= gold_from_a;
        wa.gold += gold_from_b;

        let wb = self.wallets.get_mut(&player_b).unwrap();
        wb.gold -= gold_from_b;
        wb.gold += gold_from_a;

        TradeResultMsg {
            success: true,
            failure_reason: String::new(),
        }
    }
}
