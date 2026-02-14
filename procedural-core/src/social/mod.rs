//! Social Systems: Guilds, Parties, Friends, Trading
//!
//! From dopopensource.txt Categories 2, 5, 11, 14:
//! - Guild/clan management (Nakama Groups)
//! - Party system (Nakama Parties)
//! - Friends list (Nakama Friends)
//! - Player-to-player trading
//! - Auction house
//!
//! All social state is stored server-side via Nakama.
//! This module defines the data structures and validation logic.

use serde::{Deserialize, Serialize};

// =====================
// Guild System
// =====================

/// Guild rank hierarchy
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum GuildRank {
    Recruit,
    Member,
    Officer,
    ViceLeader,
    Leader,
}

impl GuildRank {
    pub fn can_invite(&self) -> bool {
        *self >= GuildRank::Officer
    }
    pub fn can_kick(&self) -> bool {
        *self >= GuildRank::Officer
    }
    pub fn can_promote(&self) -> bool {
        *self >= GuildRank::ViceLeader
    }
    pub fn can_edit_settings(&self) -> bool {
        *self >= GuildRank::ViceLeader
    }
    pub fn can_disband(&self) -> bool {
        *self == GuildRank::Leader
    }
}

/// Guild data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guild {
    pub id: String,
    pub name: String,
    pub tag: String, // 2-4 char tag e.g. [AO]
    pub description: String,
    pub faction_affinity: Option<String>,
    pub members: Vec<GuildMember>,
    pub max_members: u32,
    pub created_at: u64, // unix timestamp
    pub guild_level: u32,
    pub guild_xp: u64,
    pub settings: GuildSettings,
    pub bank_shards: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildMember {
    pub user_id: String,
    pub name: String,
    pub rank: GuildRank,
    pub joined_at: u64,
    pub contribution: u64, // total guild XP contributed
    pub last_online: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildSettings {
    pub open_recruitment: bool,
    pub min_floor_requirement: u32,
    pub motd: String, // message of the day
}

impl Default for GuildSettings {
    fn default() -> Self {
        Self {
            open_recruitment: true,
            min_floor_requirement: 1,
            motd: String::new(),
        }
    }
}

impl Guild {
    pub fn new(
        id: String,
        name: String,
        tag: String,
        leader_id: String,
        leader_name: String,
    ) -> Self {
        Self {
            id,
            name,
            tag,
            description: String::new(),
            faction_affinity: None,
            members: vec![GuildMember {
                user_id: leader_id,
                name: leader_name,
                rank: GuildRank::Leader,
                joined_at: 0,
                contribution: 0,
                last_online: 0,
            }],
            max_members: 50,
            created_at: 0,
            guild_level: 1,
            guild_xp: 0,
            settings: GuildSettings::default(),
            bank_shards: 0,
        }
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn is_full(&self) -> bool {
        self.members.len() >= self.max_members as usize
    }

    pub fn find_member(&self, user_id: &str) -> Option<&GuildMember> {
        self.members.iter().find(|m| m.user_id == user_id)
    }

    pub fn can_join(&self, floor_reached: u32) -> bool {
        !self.is_full()
            && self.settings.open_recruitment
            && floor_reached >= self.settings.min_floor_requirement
    }

    pub fn add_member(&mut self, user_id: String, name: String) -> bool {
        if self.is_full() {
            return false;
        }
        if self.find_member(&user_id).is_some() {
            return false;
        }
        self.members.push(GuildMember {
            user_id,
            name,
            rank: GuildRank::Recruit,
            joined_at: 0,
            contribution: 0,
            last_online: 0,
        });
        true
    }

    pub fn remove_member(&mut self, user_id: &str) -> bool {
        let before = self.members.len();
        self.members.retain(|m| m.user_id != user_id);
        self.members.len() < before
    }

    pub fn add_guild_xp(&mut self, amount: u64) {
        self.guild_xp += amount;
        // Guild levels up every 1000 XP
        let new_level = (self.guild_xp / 1000) as u32 + 1;
        if new_level > self.guild_level {
            self.guild_level = new_level;
            // Expand capacity every 5 guild levels
            self.max_members = 50 + (self.guild_level / 5) * 10;
        }
    }
}

// =====================
// Party System
// =====================

/// Party role (for group content)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PartyRole {
    Vanguard,  // front-line, high aggro
    Striker,   // DPS focus
    Support,   // healing, buffs
    Tactician, // CC, positioning
}

/// Party member data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMember {
    pub user_id: String,
    pub name: String,
    pub role: PartyRole,
    pub is_leader: bool,
    pub current_floor: u32,
    pub hp_percent: f32,
}

/// Party data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: String,
    pub members: Vec<PartyMember>,
    pub max_size: u32,
    pub loot_rule: LootRule,
    pub target_floor: Option<u32>,
}

/// How loot is distributed in a party
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LootRule {
    FreeForAll,      // anyone can pick up
    RoundRobin,      // rotating assignment
    NeedBeforeGreed, // roll for needed items
    MasterLooter,    // leader decides
}

impl Party {
    pub fn new(leader_id: String, leader_name: String) -> Self {
        Self {
            id: String::new(),
            members: vec![PartyMember {
                user_id: leader_id,
                name: leader_name,
                role: PartyRole::Striker,
                is_leader: true,
                current_floor: 1,
                hp_percent: 1.0,
            }],
            max_size: 4,
            loot_rule: LootRule::NeedBeforeGreed,
            target_floor: None,
        }
    }

    pub fn is_full(&self) -> bool {
        self.members.len() >= self.max_size as usize
    }

    pub fn add_member(&mut self, user_id: String, name: String, role: PartyRole) -> bool {
        if self.is_full() {
            return false;
        }
        if self.members.iter().any(|m| m.user_id == user_id) {
            return false;
        }
        self.members.push(PartyMember {
            user_id,
            name,
            role,
            is_leader: false,
            current_floor: 1,
            hp_percent: 1.0,
        });
        true
    }

    pub fn remove_member(&mut self, user_id: &str) -> bool {
        let before = self.members.len();
        self.members.retain(|m| m.user_id != user_id);
        // If leader left, promote first remaining member
        if !self.members.is_empty() && !self.members.iter().any(|m| m.is_leader) {
            self.members[0].is_leader = true;
        }
        self.members.len() < before
    }

    pub fn leader(&self) -> Option<&PartyMember> {
        self.members.iter().find(|m| m.is_leader)
    }
}

// =====================
// Friends System
// =====================

/// Friend relationship status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FriendStatus {
    Pending,  // request sent
    Accepted, // mutual friends
    Blocked,  // blocked by this player
}

/// Friend entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendEntry {
    pub user_id: String,
    pub name: String,
    pub status: FriendStatus,
    pub since: u64,
    pub is_online: bool,
    pub current_floor: Option<u32>,
}

/// Player's friend list
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FriendList {
    pub friends: Vec<FriendEntry>,
}

impl FriendList {
    pub fn add_friend(&mut self, user_id: String, name: String) -> bool {
        if self.friends.iter().any(|f| f.user_id == user_id) {
            return false;
        }
        self.friends.push(FriendEntry {
            user_id,
            name,
            status: FriendStatus::Pending,
            since: 0,
            is_online: false,
            current_floor: None,
        });
        true
    }

    pub fn accept(&mut self, user_id: &str) -> bool {
        if let Some(f) = self.friends.iter_mut().find(|f| f.user_id == user_id) {
            if f.status == FriendStatus::Pending {
                f.status = FriendStatus::Accepted;
                return true;
            }
        }
        false
    }

    pub fn block(&mut self, user_id: &str) -> bool {
        if let Some(f) = self.friends.iter_mut().find(|f| f.user_id == user_id) {
            f.status = FriendStatus::Blocked;
            return true;
        }
        // Add as blocked even if not friends
        self.friends.push(FriendEntry {
            user_id: user_id.to_string(),
            name: String::new(),
            status: FriendStatus::Blocked,
            since: 0,
            is_online: false,
            current_floor: None,
        });
        true
    }

    pub fn remove(&mut self, user_id: &str) -> bool {
        let before = self.friends.len();
        self.friends.retain(|f| f.user_id != user_id);
        self.friends.len() < before
    }

    pub fn online_friends(&self) -> Vec<&FriendEntry> {
        self.friends
            .iter()
            .filter(|f| f.is_online && f.status == FriendStatus::Accepted)
            .collect()
    }

    pub fn is_blocked(&self, user_id: &str) -> bool {
        self.friends
            .iter()
            .any(|f| f.user_id == user_id && f.status == FriendStatus::Blocked)
    }
}

// =====================
// Trading System
// =====================

/// Trade state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeState {
    Proposing, // items being placed
    Locked,    // both sides locked in (review)
    Confirmed, // both confirmed
    Completed, // trade executed
    Cancelled, // trade cancelled
}

/// Item offered in a trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeItem {
    pub item_name: String,
    pub quantity: u32,
    pub rarity: String,
}

/// A trade between two players
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: String,
    pub player_a: String,
    pub player_b: String,
    pub items_a: Vec<TradeItem>,
    pub items_b: Vec<TradeItem>,
    pub shards_a: u64,
    pub shards_b: u64,
    pub state: TradeState,
    pub a_locked: bool,
    pub b_locked: bool,
    pub a_confirmed: bool,
    pub b_confirmed: bool,
}

impl Trade {
    pub fn new(player_a: String, player_b: String) -> Self {
        Self {
            id: String::new(),
            player_a,
            player_b,
            items_a: Vec::new(),
            items_b: Vec::new(),
            shards_a: 0,
            shards_b: 0,
            state: TradeState::Proposing,
            a_locked: false,
            b_locked: false,
            a_confirmed: false,
            b_confirmed: false,
        }
    }

    pub fn add_item(&mut self, player_id: &str, item: TradeItem) -> bool {
        if self.state != TradeState::Proposing {
            return false;
        }
        if player_id == self.player_a {
            self.items_a.push(item);
            true
        } else if player_id == self.player_b {
            self.items_b.push(item);
            true
        } else {
            false
        }
    }

    pub fn set_shards(&mut self, player_id: &str, amount: u64) -> bool {
        if self.state != TradeState::Proposing {
            return false;
        }
        if player_id == self.player_a {
            self.shards_a = amount;
            true
        } else if player_id == self.player_b {
            self.shards_b = amount;
            true
        } else {
            false
        }
    }

    pub fn lock(&mut self, player_id: &str) -> bool {
        if self.state != TradeState::Proposing {
            return false;
        }
        if player_id == self.player_a {
            self.a_locked = true;
        } else if player_id == self.player_b {
            self.b_locked = true;
        } else {
            return false;
        }

        if self.a_locked && self.b_locked {
            self.state = TradeState::Locked;
        }
        true
    }

    pub fn confirm(&mut self, player_id: &str) -> bool {
        if self.state != TradeState::Locked {
            return false;
        }
        if player_id == self.player_a {
            self.a_confirmed = true;
        } else if player_id == self.player_b {
            self.b_confirmed = true;
        } else {
            return false;
        }

        if self.a_confirmed && self.b_confirmed {
            self.state = TradeState::Confirmed;
        }
        true
    }

    pub fn cancel(&mut self) {
        self.state = TradeState::Cancelled;
    }

    pub fn execute(&mut self) -> bool {
        if self.state != TradeState::Confirmed {
            return false;
        }
        self.state = TradeState::Completed;
        true
    }
}

// =====================
// Auction House
// =====================

/// Auction listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuctionListing {
    pub id: String,
    pub seller_id: String,
    pub seller_name: String,
    pub item: TradeItem,
    pub buyout_price: u64,
    pub current_bid: u64,
    pub highest_bidder: Option<String>,
    pub expires_at: u64,
    pub category: String,
}

impl AuctionListing {
    pub fn place_bid(&mut self, bidder_id: String, amount: u64) -> bool {
        if amount <= self.current_bid {
            return false;
        }
        if bidder_id == self.seller_id {
            return false;
        }
        self.current_bid = amount;
        self.highest_bidder = Some(bidder_id);
        true
    }

    pub fn buyout(&mut self, buyer_id: String) -> bool {
        if buyer_id == self.seller_id {
            return false;
        }
        self.current_bid = self.buyout_price;
        self.highest_bidder = Some(buyer_id);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Guild tests
    #[test]
    fn test_guild_create() {
        let guild = Guild::new(
            "g1".into(),
            "Test Guild".into(),
            "TG".into(),
            "u1".into(),
            "Leader".into(),
        );
        assert_eq!(guild.member_count(), 1);
        assert!(!guild.is_full());
        assert_eq!(guild.find_member("u1").unwrap().rank, GuildRank::Leader);
    }

    #[test]
    fn test_guild_add_remove() {
        let mut guild = Guild::new(
            "g1".into(),
            "Test".into(),
            "TG".into(),
            "u1".into(),
            "Leader".into(),
        );
        assert!(guild.add_member("u2".into(), "Player2".into()));
        assert_eq!(guild.member_count(), 2);
        assert!(!guild.add_member("u2".into(), "Player2".into())); // duplicate
        assert!(guild.remove_member("u2"));
        assert_eq!(guild.member_count(), 1);
    }

    #[test]
    fn test_guild_rank_permissions() {
        assert!(!GuildRank::Recruit.can_invite());
        assert!(!GuildRank::Member.can_kick());
        assert!(GuildRank::Officer.can_invite());
        assert!(GuildRank::ViceLeader.can_promote());
        assert!(GuildRank::Leader.can_disband());
    }

    #[test]
    fn test_guild_level_up() {
        let mut guild = Guild::new(
            "g1".into(),
            "Test".into(),
            "TG".into(),
            "u1".into(),
            "Leader".into(),
        );
        assert_eq!(guild.guild_level, 1);
        guild.add_guild_xp(1000);
        assert_eq!(guild.guild_level, 2);
        guild.add_guild_xp(4000);
        assert_eq!(guild.guild_level, 6);
        assert!(guild.max_members > 50); // expanded
    }

    // Party tests
    #[test]
    fn test_party_create() {
        let party = Party::new("u1".into(), "Leader".into());
        assert_eq!(party.members.len(), 1);
        assert!(party.leader().unwrap().is_leader);
    }

    #[test]
    fn test_party_full() {
        let mut party = Party::new("u1".into(), "Leader".into());
        party.add_member("u2".into(), "P2".into(), PartyRole::Striker);
        party.add_member("u3".into(), "P3".into(), PartyRole::Support);
        party.add_member("u4".into(), "P4".into(), PartyRole::Vanguard);
        assert!(party.is_full());
        assert!(!party.add_member("u5".into(), "P5".into(), PartyRole::Tactician));
    }

    #[test]
    fn test_party_leader_transfer() {
        let mut party = Party::new("u1".into(), "Leader".into());
        party.add_member("u2".into(), "P2".into(), PartyRole::Striker);
        party.remove_member("u1");
        assert!(party.leader().unwrap().user_id == "u2");
    }

    // Friends tests
    #[test]
    fn test_friend_list() {
        let mut list = FriendList::default();
        assert!(list.add_friend("u2".into(), "Player2".into()));
        assert!(!list.add_friend("u2".into(), "Player2".into())); // duplicate
        assert!(list.accept("u2"));
        assert_eq!(list.friends[0].status, FriendStatus::Accepted);
    }

    #[test]
    fn test_block_player() {
        let mut list = FriendList::default();
        list.add_friend("u2".into(), "Player2".into());
        list.accept("u2");
        list.block("u2");
        assert!(list.is_blocked("u2"));
    }

    // Trade tests
    #[test]
    fn test_trade_flow() {
        let mut trade = Trade::new("u1".into(), "u2".into());
        assert_eq!(trade.state, TradeState::Proposing);

        trade.add_item(
            "u1",
            TradeItem {
                item_name: "Sword".into(),
                quantity: 1,
                rarity: "Rare".into(),
            },
        );
        trade.set_shards("u2", 500);

        trade.lock("u1");
        assert_eq!(trade.state, TradeState::Proposing); // need both
        trade.lock("u2");
        assert_eq!(trade.state, TradeState::Locked);

        trade.confirm("u1");
        assert_eq!(trade.state, TradeState::Locked); // need both
        trade.confirm("u2");
        assert_eq!(trade.state, TradeState::Confirmed);

        assert!(trade.execute());
        assert_eq!(trade.state, TradeState::Completed);
    }

    #[test]
    fn test_trade_cancel() {
        let mut trade = Trade::new("u1".into(), "u2".into());
        trade.cancel();
        assert_eq!(trade.state, TradeState::Cancelled);
        assert!(!trade.add_item(
            "u1",
            TradeItem {
                item_name: "X".into(),
                quantity: 1,
                rarity: "C".into()
            }
        ));
    }

    // Auction tests
    #[test]
    fn test_auction_bid() {
        let mut listing = AuctionListing {
            id: "a1".into(),
            seller_id: "u1".into(),
            seller_name: "Seller".into(),
            item: TradeItem {
                item_name: "Staff".into(),
                quantity: 1,
                rarity: "Epic".into(),
            },
            buyout_price: 1000,
            current_bid: 100,
            highest_bidder: None,
            expires_at: 0,
            category: "Weapon".into(),
        };

        assert!(listing.place_bid("u2".into(), 200));
        assert!(!listing.place_bid("u3".into(), 150)); // too low
        assert!(!listing.place_bid("u1".into(), 500)); // seller can't bid
    }

    #[test]
    fn test_auction_buyout() {
        let mut listing = AuctionListing {
            id: "a1".into(),
            seller_id: "u1".into(),
            seller_name: "Seller".into(),
            item: TradeItem {
                item_name: "Helm".into(),
                quantity: 1,
                rarity: "Legendary".into(),
            },
            buyout_price: 5000,
            current_bid: 0,
            highest_bidder: None,
            expires_at: 0,
            category: "Armor".into(),
        };

        assert!(listing.buyout("u2".into()));
        assert_eq!(listing.current_bid, 5000);
    }
}
