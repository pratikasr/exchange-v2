# Updated Prediction Market Contract Design

## 1. Data Structures

### Config
```rust
pub struct Config {
    pub admin: Addr,                 // Contract administrator
    pub token_denom: String,         // Token denomination for betting
    pub platform_fee: Uint128,       // Platform fee in basis points
    pub treasury: Addr,              // Treasury address for fee collection
    pub challenging_period: u64,     // Period for challenging results (in seconds)
    pub voting_period: u64,          // Period for voting on disputes (in seconds)
    pub min_bet: Uint128,            // Minimum bet amount
    pub whitelist_enabled: bool,     // Flag to enable/disable whitelist
}

pub const CONFIG: Item<Config> = Item::new("config");
```

### Market
```rust
pub struct Market {
    pub id: u64,                     // Unique market identifier
    pub creator: Addr,               // Market creator's address
    pub description: String,         // Market description
    pub options: Vec<String>,        // Available betting options
    pub category: String,            // Market category
    pub start_time: u64,             // Market start time
    pub end_time: u64,               // Market end time
    pub status: MarketStatus,        // Current market status
    pub resolution_bond: Uint128,    // Bond required for resolution
    pub resolution_reward: Uint128,  // Reward for resolving the market
    pub result: Option<u8>,          // Winning option (if resolved)
}

pub enum MarketStatus {
    Active,
    Closed,
    InDispute,
    Resolved,
}

pub const MARKETS: Map<u64, Market> = Map::new("markets");
pub const MARKET_COUNT: Item<u64> = Item::new("market_count");
```

### Order
```rust
pub struct Order {
    pub id: u64,                     // Unique order identifier
    pub market_id: u64,              // Associated market ID
    pub creator: Addr,               // Order creator's address
    pub option_id: u8,               // Selected option ID
    pub side: OrderSide,             // Back or Lay
    pub amount: Uint128,             // Total order amount
    pub odds: u32,                   // Odds (percentage * 100)
    pub filled_amount: Uint128,      // Amount already filled
    pub status: OrderStatus,         // Current order status
    pub timestamp: u64,              // Order creation timestamp
}

pub enum OrderSide {
    Back,
    Lay,
}

pub enum OrderStatus {
    Open,
    PartiallyFilled,
    Filled,
    Canceled,
}

pub const ORDERS: Map<u64, Order> = Map::new("orders");
pub const ORDER_COUNT: Item<u64> = Item::new("order_count");
pub const USER_ORDERS: Map<(Addr, u64), Vec<u64>> = Map::new("user_orders");
```

### MatchedBet
```rust
pub struct MatchedBet {
    pub id: u64,                     // Unique matched bet identifier
    pub market_id: u64,              // Associated market ID
    pub option_id: u8,               // Selected option ID
    pub amount: Uint128,             // Matched amount
    pub odds: u32,                   // Matched odds
    pub timestamp: u64,              // Match timestamp
    pub redeemed: bool,              // Whether the bet has been redeemed
}

pub const MATCHED_BETS: Map<u64, MatchedBet> = Map::new("matched_bets");
pub const MATCHED_BET_COUNT: Item<u64> = Item::new("matched_bet_count");
pub const USER_MATCHED_BETS: Map<(Addr, u64), Vec<u64>> = Map::new("user_matched_bets");
```

### ResolutionProposal
```rust
pub struct ResolutionProposal {
    pub market_id: u64,              // Associated market ID
    pub proposer: Addr,              // Proposer's address
    pub proposed_result: u8,         // Proposed winning option
    pub bond_amount: Uint128,        // Bond amount for proposal
    pub proposal_time: u64,          // Proposal timestamp
    pub challenge_deadline: u64,     // Deadline for challenges
    pub status: ProposalStatus,      // Current proposal status
}

pub enum ProposalStatus {
    Active,
    Challenged,
    Resolved,
}

pub const PROPOSALS: Map<u64, ResolutionProposal> = Map::new("proposals");
```

### Vote
```rust
pub struct Vote {
    pub voter: Addr,                 // Voter's address
    pub option_id: u8,               // Voted option ID
}

pub const VOTES: Map<(u64, Addr), Vote> = Map::new("votes");
pub const VOTE_COUNTS: Map<(u64, u8), u64> = Map::new("vote_counts");
```

### Whitelist
```rust
pub const WHITELISTED_ADDRESSES: Map<Addr, bool> = Map::new("whitelisted_addresses");
```

## 2. Key Functions

### Configuration Management
1. `initialize_config(admin, token_denom, platform_fee, min_bet, dispute_period, settlement_period, whitelist_enabled)`
   - Initializes the contract configuration

2. `update_config(field, value)`
   - Updates a specific field in the configuration

### Market Management
1. `create_market(category, description, options, start_time, end_time, resolution_bond, resolution_reward)`
   - Creates a new market with the specified parameters

2. `cancel_market(market_id)`
   - Cancels a market, refunding all bets

3. `close_market(market_id)`
   - Closes a market, preventing new bets

4. `settle_market(market_id, winning_outcome)`
   - Settles a market with the specified winning outcome

### Order Management
1. `place_order(market_id, option_id, order_type, side, amount, odds)`
   - Places a new order and attempts to match it

2. `cancel_order(order_id)`
   - Cancels an existing order, refunding unmatched amount

### Matching Engine
1. `match_orders(order)`
   - Internal function called by `place_order` to match the new order with existing ones

### Settlement System
1. `settle_matched_bets(market_id)`
   - Settles all matched bets for a resolved market

2. `redeem_winnings(matched_bet_id)`
   - Allows users to redeem winnings for a specific matched bet


### Admin Functions
1. `add_to_whitelist(address)`
   - Adds an address to the whitelist

2. `remove_from_whitelist(address)`
   - Removes an address from the whitelist

### Dispute Resolution
1. `raise_dispute(market_id, proposed_outcome, evidence)`
   - Raises a dispute for a settled market

2. `cast_vote(market_id, outcome)`
   - Casts a vote in an ongoing dispute

3. `resolve_dispute(market_id)`
   - Resolves a dispute based on voting results

### Query Functions
1. `query_config()`
2. `query_market(market_id)`
3. `query_order_book(market_id, option_id)`
4. `query_user_orders(user, market_id)`
5. `query_user_matched_bets(user, market_id)`
6. `query_user_balance(user)`
7. `query_whitelist_status(address)`
8. `query_dispute(market_id)`

## 3. Detailed Process Flows

### Market Lifecycle
1. Market Creation:
   * Only whitelisted addresses can create markets when whitelist is enabled.
   * Market starts in 'Active' status.
   * Order book is initialized for each option.

2. Betting Period:
   * Users can place and cancel orders.
   * Matching engine runs when orders are placed.
   * Partial matches are allowed and recorded.

3. Market Closure:
   * Market is not automatically closed at `end_time`.
   * The first action after `end_time` will trigger market closure.
   * No new orders can be placed after closure.
   * All unmatched orders are cancelled and funds returned.

4. Settlement:
   * Admin or whitelisted user settles the market by providing the winning outcome.
   * The settler receives the `resolution_reward`.
   * Matched bets are marked as settled, but not automatically paid out.

5. Redemption:
   * Users can call `redeem_winnings` to claim their winnings for each matched bet.

6. Dispute Period:
   * Users can raise disputes within the `challenging_period`.
   * If a dispute is raised, the market enters 'InDispute' status.
   * Whitelisted addresses can vote on the correct outcome.
   * After the voting period, the dispute is resolved based on votes.

7. Final Settlement:
   * If no dispute is raised or after dispute resolution, the market is finally settled.
   * Users can redeem winnings based on the final outcome.

### Order Matching Process
1. When a new order is placed:
   * The `match_orders` function is called internally.
   * It attempts to match the new order with existing orders in the order book.

2. Matching Priority:
   * Price-Time Priority: Better odds are matched first. For equal odds, earlier orders are prioritized.

3. Partial Matches:
   * Orders can be partially matched.
   * For each match (partial or full), a new `MatchedBet` is created.
   * Remaining unmatched amount stays in the order book.

4. After Market Closure:
   * All unmatched orders are cancelled and funds returned to users.
   * Partially matched orders are treated as follows:
     - The matched portion remains as a `MatchedBet`.
     - The unmatched portion is refunded to the user.

### Dispute Resolution Process
1. Dispute Raising:
   * Any user can raise a dispute within the `challenging_period` after market settlement.
   * The challenger must provide evidence and a proposed outcome.

2. Voting:
   * Only whitelisted addresses can vote.
   * Each address gets one vote per dispute.
   * Voting period lasts for the predefined `voting_period`.

3. Resolution:
   * After the voting period, the outcome with the most votes wins.
   * In case of a tie, the original outcome stands.
   * If the challenger's outcome wins, the market is re-settled with the new outcome.
   * If the vote is heavily skewed towards cancellation, the market can be cancelled, and all bets refunded.

## 4. Security Considerations

1. Access Control:
   * Implement strict access control for admin and whitelisted functions.
   * Use modifiers to enforce access restrictions.

2. Input Validation:
   * Validate all inputs in every function to prevent invalid data.
   * Check for overflows, underflows, and division by zero.

3. Reentrancy Protection:
   * Use reentrancy guards for functions involving token transfers.

4. Gas Limit Considerations:
   * Design functions to work within gas limits, especially for loops and complex operations.
   * Implement pagination for large data set queries.

5. Upgradability:
   * Consider implementing an upgrade pattern for future improvements and bug fixes.

6. Emergency Stops:
   * Implement a circuit breaker pattern to pause the contract in case of emergencies.

7. Event Emissions:
   * Emit events for all significant state changes to aid in off-chain tracking and auditing.


These changes should provide a more accurate and efficient design for your prediction market contract.



