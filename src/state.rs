use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
    pub token_denom: String,
    pub platform_fee: Uint128,
    pub treasury: Addr,
    pub challenging_period: u64,
    pub voting_period: u64,
    pub min_bet: Uint128,
    pub whitelist_enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Market {
    pub id: u64,
    pub creator: Addr,
    pub question: String, 
    pub description: String,
    pub options: Vec<String>,
    pub category: String,
    pub start_time: u64,
    pub end_time: u64,
    pub status: MarketStatus,
    pub resolution_bond: Uint128,
    pub resolution_reward: Uint128,
    pub result: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum MarketStatus {
    Active,
    Closed,
    Canceled,
    InDispute,
    Resolved,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Order {
    pub id: u64,
    pub market_id: u64,
    pub creator: Addr,
    pub option_id: u8,
    pub side: OrderSide,
    pub amount: Uint128,
    pub odds: u32,
    pub filled_amount: Uint128,
    pub status: OrderStatus,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OrderSide {
    Back,
    Lay,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Canceled,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MatchedBet {
    pub id: u64,
    pub market_id: u64,
    pub option_id: u8,
    pub amount: Uint128,
    pub odds: u32,
    pub timestamp: u64,
    pub back_user: Addr,
    pub lay_user: Addr,
    pub redeemed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ResolutionProposal {
    pub market_id: u64,
    pub proposer: Addr,
    pub proposed_result: u8,
    pub bond_amount: Uint128,
    pub proposal_time: u64,
    pub challenge_deadline: u64,
    pub status: ProposalStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ProposalStatus {
    Active,
    Challenged,
    Resolved,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Vote {
    pub voter: Addr,
    pub option_id: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Dispute {
    pub market_id: u64,
    pub challenger: Addr,
    pub proposed_outcome: u8,
    pub evidence: String,
    pub status: DisputeStatus,
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum DisputeStatus {
    Active,
    Resolved,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MarketStatistics {
    pub market_id: u64,
    pub total_volume: Uint128,
    pub order_count: u64,
    pub status: MarketStatus,
}

impl fmt::Display for MarketStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MarketStatus::Active => write!(f, "Active"),
            MarketStatus::Closed => write!(f, "Closed"),
            MarketStatus::Canceled => write!(f, "Canceled"),
            MarketStatus::InDispute => write!(f, "InDispute"),
            MarketStatus::Resolved => write!(f, "Resolved"),
        }
    }
}

impl fmt::Display for OrderSide {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OrderSide::Back => write!(f, "Back"),
            OrderSide::Lay => write!(f, "Lay"),
        }
    }
}


pub const CONFIG: Item<Config> = Item::new("config");
pub const MARKETS: Map<u64, Market> = Map::new("markets");
pub const MARKET_COUNT: Item<u64> = Item::new("market_count");
pub const ORDERS: Map<u64, Order> = Map::new("orders");
pub const ORDER_COUNT: Item<u64> = Item::new("order_count");
pub const USER_ORDERS: Map<(Addr, u64), Vec<u64>> = Map::new("user_orders");
pub const MATCHED_BETS: Map<u64, MatchedBet> = Map::new("matched_bets");
pub const MATCHED_BET_COUNT: Item<u64> = Item::new("matched_bet_count");
pub const USER_MATCHED_BETS: Map<(Addr, u64), Vec<u64>> = Map::new("user_matched_bets");
pub const PROPOSALS: Map<u64, ResolutionProposal> = Map::new("proposals");
pub const VOTES: Map<(u64, Addr), Vote> = Map::new("votes");
pub const VOTE_COUNTS: Map<(u64, u8), u64> = Map::new("vote_counts");
pub const WHITELISTED_ADDRESSES: Map<Addr, bool> = Map::new("whitelisted_addresses");
pub const DISPUTES: Map<u64, Dispute> = Map::new("disputes");