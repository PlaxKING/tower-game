//! EconomyService — Trading, crafting, auction, wallet endpoints
//!
//! Endpoints:
//! - POST /tower.EconomyService/GetWallet
//! - POST /tower.EconomyService/Craft
//! - POST /tower.EconomyService/ListAuction
//! - POST /tower.EconomyService/BuyAuction
//! - POST /tower.EconomyService/Trade

use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use super::ApiState;

pub fn routes() -> Router<ApiState> {
    Router::new()
        .route("/tower.EconomyService/GetWallet", post(get_wallet))
        .route("/tower.EconomyService/Craft", post(craft))
        .route("/tower.EconomyService/ListAuction", post(list_auctions))
        .route("/tower.EconomyService/BuyAuction", post(buy_auction))
        .route("/tower.EconomyService/Trade", post(trade))
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Deserialize)]
pub struct WalletRequest {
    pub player_id: u64,
}

#[derive(Serialize)]
pub struct WalletResponse {
    pub gold: u64,
    pub premium_currency: u32,
    pub honor_points: u32,
    /// Alias for honor_points — UE5 GRPCClientManager reads this field
    pub seasonal_currency: u32,
}

#[derive(Deserialize)]
pub struct CraftRequest {
    pub player_id: u64,
    pub recipe_id: String,
    pub material_item_ids: Vec<String>,
}

#[derive(Serialize)]
pub struct CraftResponse {
    pub success: bool,
    pub failure_reason: String,
    pub crafted_item_id: String,
    pub mastery_xp_gained: f32,
}

#[derive(Deserialize)]
pub struct AuctionListRequest {
    pub category: String,
    pub page: u32,
    pub per_page: u32,
}

#[derive(Serialize)]
pub struct AuctionListResponse {
    pub entries: Vec<AuctionEntry>,
    pub total_count: u32,
}

#[derive(Serialize)]
pub struct AuctionEntry {
    pub id: u64,
    pub item_template_id: String,
    pub quantity: u32,
    pub buyout_price: u64,
    pub seller_id: u64,
    pub expires_at: i64,
}

#[derive(Deserialize)]
pub struct AuctionBuyRequest {
    pub player_id: u64,
    pub auction_id: u64,
}

#[derive(Serialize)]
pub struct AuctionBuyResponse {
    pub success: bool,
    pub failure_reason: String,
}

#[derive(Deserialize)]
pub struct TradeRequest {
    pub player_a: u64,
    pub player_b: u64,
    pub gold_from_a: u64,
    pub gold_from_b: u64,
}

#[derive(Serialize)]
pub struct TradeResponse {
    pub success: bool,
    pub failure_reason: String,
}

// ============================================================================
// Handlers
// ============================================================================

async fn get_wallet(
    State(state): State<ApiState>,
    Json(req): Json<WalletRequest>,
) -> Json<WalletResponse> {
    match state.pg.get_wallet(req.player_id as i64).await {
        Ok(wallet) => {
            let honor = wallet.honor_points as u32;
            Json(WalletResponse {
                gold: wallet.gold as u64,
                premium_currency: wallet.premium_currency as u32,
                honor_points: honor,
                seasonal_currency: honor, // UE5 reads this field
            })
        }
        Err(_) => Json(WalletResponse {
            gold: 0,
            premium_currency: 0,
            honor_points: 0,
            seasonal_currency: 0,
        }),
    }
}

async fn craft(
    State(state): State<ApiState>,
    Json(req): Json<CraftRequest>,
) -> Json<CraftResponse> {
    // Look up recipe from LMDB
    let recipe = match state.lmdb.get_recipe(&req.recipe_id) {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Json(CraftResponse {
                success: false,
                failure_reason: format!("Recipe '{}' not found", req.recipe_id),
                crafted_item_id: String::new(),
                mastery_xp_gained: 0.0,
            })
        }
        Err(e) => {
            return Json(CraftResponse {
                success: false,
                failure_reason: e.to_string(),
                crafted_item_id: String::new(),
                mastery_xp_gained: 0.0,
            })
        }
    };

    // Verify player has materials (simplified: check inventory via PostgreSQL)
    let bag = state
        .pg
        .get_bag(req.player_id as i64)
        .await
        .unwrap_or_default();
    let bag_items: std::collections::HashSet<String> =
        bag.iter().map(|s| s.item_template_id.clone()).collect();

    for ingredient in &recipe.ingredients {
        if !bag_items.contains(&ingredient.item_template_id) {
            return Json(CraftResponse {
                success: false,
                failure_reason: format!("Missing material: {}", ingredient.item_template_id),
                crafted_item_id: String::new(),
                mastery_xp_gained: 0.0,
            });
        }
    }

    // Add crafted item to inventory
    let crafted_id = &recipe.result_item_id;
    match state
        .pg
        .add_item(
            req.player_id as i64,
            crafted_id,
            recipe.result_quantity as i32,
            0,
        )
        .await
    {
        Ok(_) => {
            // Award mastery XP for crafting (base 50 XP per craft)
            let xp = 50i64;
            let _ = state
                .pg
                .add_mastery_experience(req.player_id as i64, &recipe.profession, xp)
                .await;

            Json(CraftResponse {
                success: true,
                failure_reason: String::new(),
                crafted_item_id: crafted_id.clone(),
                mastery_xp_gained: xp as f32,
            })
        }
        Err(e) => Json(CraftResponse {
            success: false,
            failure_reason: e.to_string(),
            crafted_item_id: String::new(),
            mastery_xp_gained: 0.0,
        }),
    }
}

async fn list_auctions(
    State(state): State<ApiState>,
    Json(req): Json<AuctionListRequest>,
) -> Json<AuctionListResponse> {
    let per_page = req.per_page.min(50).max(1) as i32;
    let offset = (req.page * req.per_page) as i32;

    let rows = state
        .pg
        .get_active_auctions(per_page, offset)
        .await
        .unwrap_or_default();

    let entries: Vec<AuctionEntry> = rows
        .iter()
        .map(|r| AuctionEntry {
            id: r.id as u64,
            item_template_id: r.item_template_id.clone(),
            quantity: r.quantity as u32,
            buyout_price: r.buyout_price as u64,
            seller_id: r.seller_id as u64,
            expires_at: r.expires_at.timestamp(),
        })
        .collect();

    let total = entries.len() as u32;

    Json(AuctionListResponse {
        entries,
        total_count: total,
    })
}

async fn buy_auction(
    State(state): State<ApiState>,
    Json(req): Json<AuctionBuyRequest>,
) -> Json<AuctionBuyResponse> {
    match state
        .pg
        .buyout_auction(req.auction_id as i64, req.player_id as i64)
        .await
    {
        Ok(()) => Json(AuctionBuyResponse {
            success: true,
            failure_reason: String::new(),
        }),
        Err(e) => Json(AuctionBuyResponse {
            success: false,
            failure_reason: e.to_string(),
        }),
    }
}

async fn trade(
    State(state): State<ApiState>,
    Json(req): Json<TradeRequest>,
) -> Json<TradeResponse> {
    // Gold trade (atomic)
    if req.gold_from_a > 0 {
        if let Err(e) = state
            .pg
            .transfer_gold(
                req.player_a as i64,
                req.player_b as i64,
                req.gold_from_a as i64,
            )
            .await
        {
            return Json(TradeResponse {
                success: false,
                failure_reason: e.to_string(),
            });
        }
    }
    if req.gold_from_b > 0 {
        if let Err(e) = state
            .pg
            .transfer_gold(
                req.player_b as i64,
                req.player_a as i64,
                req.gold_from_b as i64,
            )
            .await
        {
            return Json(TradeResponse {
                success: false,
                failure_reason: e.to_string(),
            });
        }
    }

    Json(TradeResponse {
        success: true,
        failure_reason: String::new(),
    })
}
