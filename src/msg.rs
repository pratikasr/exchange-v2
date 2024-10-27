use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};
use crate::state:: OrderSide;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
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
pub struct MigrateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig { field: String, value: String },
    CreateMarket {
        category: String,
        question: String,
        description: String,
        options: Vec<String>,
        start_time: String, 
        end_time: String,   
        resolution_bond: Uint128,
        resolution_reward: Uint128,
    },
    CancelMarket { market_id: u64 },
    CloseMarket { market_id: u64 },
    ProposeResult { market_id: u64, winning_outcome: u8 },
    PlaceOrder {
        market_id: u64,
        option_id: u8,
        order_type: OrderType,
        side: OrderSide,
        amount: Uint128,
        odds: u32,
    },
    CancelOrder { order_id: u64 },
    RedeemWinnings { matched_bet_id: u64 },
    AddToWhitelist { address: Addr },
    RemoveFromWhitelist { address: Addr },
    RaiseDispute {
        market_id: u64,
        proposed_outcome: u8,
        evidence: String,
    },
    CastVote { market_id: u64, outcome: u8 },
    ResolveDispute { market_id: u64 },
    RedeemBondAmount { market_id: u64 }, // Fix Bug ID #2
}

#[cw_serde]
pub enum OrderType {
    Limit,
    Market,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Market { market_id: u64 },
    Markets { status: Option<String>, start_after: Option<u64>, limit: Option<u32> },
    Order { order_id: u64 },
    UserOrders { user: Addr, market_id: Option<u64>, start_after: Option<u64>, limit: Option<u32> },
    MarketOrders { market_id: u64, side: Option<String>, start_after: Option<u64>, limit: Option<u32> },
    MatchedBets { market_id: Option<u64>, user: Option<Addr>, start_after: Option<u64>, limit: Option<u32> },
    ResolutionProposal { market_id: u64 },
    Dispute { market_id: u64 },
    Votes { market_id: u64 },
    IsWhitelisted { user: Addr },
    MarketStatistics { market_id: u64 },
    WhitelistedAddresses { start_after: Option<String>, limit: Option<u32> },
}