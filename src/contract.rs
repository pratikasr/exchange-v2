use cosmwasm_std::{
    entry_point, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, Addr, BankMsg, Coin, to_json_binary, Deps, Binary, CosmosMsg
};
use crate::error::ContractError;
use crate::msg::{InstantiateMsg, ExecuteMsg, MigrateMsg};
use crate::state::{Config, CONFIG, MARKET_COUNT, ORDER_COUNT, MATCHED_BET_COUNT, Market, MARKETS, PROPOSALS, ResolutionProposal, ProposalStatus, MarketStatus, Dispute, DisputeStatus, WHITELISTED_ADDRESSES, OrderSide, ORDERS, Order, OrderStatus, MATCHED_BETS, MatchedBet, VOTES, VOTE_COUNTS, Vote, DISPUTES, MarketStatistics};
use crate::msg::OrderType;
use std::str::FromStr;
use crate::msg::QueryMsg;
use cw_storage_plus::Bound;
use regex::Regex;


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // Fix Bug ID #13: Validate non-zero addresses
    validate_non_zero_addr(&msg.admin)?;
    validate_non_zero_addr(&msg.treasury)?;

    // Fix Bug #14 & #15: Validate both periods
    validate_period(msg.challenging_period, "challenging_period")?;
    validate_period(msg.voting_period, "voting_period")?;

    // Fix Bug #16: Validate min_bet
    validate_min_bet(msg.min_bet)?;

    let config = Config {
        admin: msg.admin,
        token_denom: msg.token_denom,
        platform_fee: msg.platform_fee,
        treasury: msg.treasury,
        challenging_period: msg.challenging_period,
        voting_period: msg.voting_period,
        min_bet: msg.min_bet,
        whitelist_enabled: msg.whitelist_enabled,
    };

    CONFIG.save(deps.storage, &config)?;
    MARKET_COUNT.save(deps.storage, &0u64)?;
    ORDER_COUNT.save(deps.storage, &0u64)?;
    MATCHED_BET_COUNT.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", config.admin.to_string())
        .add_attribute("token_denom", config.token_denom)
        .add_attribute("platform_fee", config.platform_fee.to_string())
        .add_attribute("treasury", config.treasury.to_string())
        .add_attribute("challenging_period", config.challenging_period.to_string())
        .add_attribute("voting_period", config.voting_period.to_string())
        .add_attribute("min_bet", config.min_bet.to_string())
        .add_attribute("whitelist_enabled", config.whitelist_enabled.to_string()))
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    field: String,
    value: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // Check if the sender is the admin
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    match field.as_str() {
        "admin" => {
            let new_admin = deps.api.addr_validate(&value)?;
            // Fix Bug ID #13: Validate non-zero address
            validate_non_zero_addr(&new_admin)?;
            config.admin = new_admin;
        },
        "token_denom" => config.token_denom = value.clone(),
        "platform_fee" => config.platform_fee = value.parse::<Uint128>()?,
        "treasury" => {
            let new_treasury = deps.api.addr_validate(&value)?;
            // Fix Bug ID #13: Validate non-zero address
            validate_non_zero_addr(&new_treasury)?;
            config.treasury = new_treasury;
        },        
        "challenging_period" => {
            let period = u64::from_str(&value)
                .map_err(|_| ContractError::InvalidField { field: field.clone() })?;
            // Fix Bug #14: Validate challenging period
            validate_period(period, &field)?;
            config.challenging_period = period;
        },
        "voting_period" => {
            let period = u64::from_str(&value)
                .map_err(|_| ContractError::InvalidField { field: field.clone() })?;
            // Fix Bug #15: Validate voting period
            validate_period(period, &field)?;
            config.voting_period = period;
        },
        "min_bet" => {
            let min_bet = Uint128::from_str(&value)
                .map_err(|_| ContractError::InvalidField { field: field.clone() })?;
            // Fix Bug #16: Validate min_bet
            validate_min_bet(min_bet)?;
            config.min_bet = min_bet;
        },        
        "whitelist_enabled" => config.whitelist_enabled = bool::from_str(&value).map_err(|_| ContractError::InvalidField { field: field.clone() })?,
        _ => return Err(ContractError::InvalidField { field: field.to_string() }),
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "update_config")
        .add_attribute("field", field)
        .add_attribute("value", value))
}

fn validate_non_zero_addr(addr: &Addr) -> Result<(), ContractError> {
    if addr == &Addr::unchecked("") {
        return Err(ContractError::ZeroAddress {});
    }
    Ok(())
}

fn validate_period(period: u64, field: &str) -> Result<(), ContractError> {
    if period == 0 {
        return Err(ContractError::InvalidPeriod { field: field.to_string() });
    }
    Ok(())
}

fn validate_min_bet(amount: Uint128) -> Result<(), ContractError> {
    if amount.is_zero() {
        return Err(ContractError::InvalidMinBet {});
    }
    Ok(())
}

pub fn create_market(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    category: String,
    question: String, 
    description: String,
    options: Vec<String>,
    start_time: String,
    end_time: String,
    resolution_bond: Uint128,
    resolution_reward: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Fix Bug ID #11: Validate resolution_bond
    if resolution_bond.is_zero() || resolution_bond < config.min_bet {
        return Err(ContractError::InvalidResolutionBond {});
    }

    // Parse times
    let start_time = start_time.parse::<u64>().map_err(|_| ContractError::InvalidTimeFormat {})?;
    let end_time = end_time.parse::<u64>().map_err(|_| ContractError::InvalidTimeFormat {})?;

    // Fix Bug ID #12: Validate start_time is in the future
    if start_time <= env.block.time.seconds() {
        return Err(ContractError::InvalidTimeRange {});
    }

    // Check if whitelist is enabled and if so, if the creator is whitelisted
    if config.whitelist_enabled {
        let is_whitelisted = crate::state::WHITELISTED_ADDRESSES.may_load(deps.storage, info.sender.clone())?.unwrap_or(false);
        if !is_whitelisted {
            return Err(ContractError::NotWhitelisted {});
        }
    }

    // Validate input parameters
    if options.is_empty() || options.len() > 10 {
        return Err(ContractError::InvalidOptions {});
    }
    if start_time >= end_time {
        return Err(ContractError::InvalidTimeRange {});
    }

    // Validate question format
    validate_question(&question)?;

    // Fix Bug ID #17: Validate description
    validate_description(&description)?;

    // Check if the correct amount of funds is sent for resolution_reward
    let required_funds = resolution_reward;
    let sent_funds = info.funds.iter().find(|coin| coin.denom == config.token_denom);
    match sent_funds {
        Some(coin) if coin.amount == required_funds => {}
        _ => return Err(ContractError::InsufficientFunds {}),
    }

    let mut market_id = MARKET_COUNT.load(deps.storage)?;
    market_id += 1;

    let market = Market {
        id: market_id,
        creator: info.sender.clone(),
        question,
        description,
        options,
        category,
        start_time,
        end_time,
        status: MarketStatus::Active,
        resolution_bond,
        resolution_reward,
        result: None,
    };

    MARKETS.save(deps.storage, market_id, &market)?;
    MARKET_COUNT.save(deps.storage, &market_id)?;

    Ok(Response::new()
        .add_attribute("method", "create_market")
        .add_attribute("market_id", market_id.to_string())
        .add_attribute("creator", info.sender))
}


fn validate_question(question: &str) -> Result<(), ContractError> {
    let re = Regex::new(r"^[A-Za-z0-9\s\?\.,!-]{10,200}$").unwrap();
    if !re.is_match(question) {
        return Err(ContractError::InvalidQuestionFormat {});
    }
    Ok(())
}

fn validate_description(description: &str) -> Result<(), ContractError> {
    // Description should be between 20 and 1000 characters
    if description.len() < 20 || description.len() > 1000 {
        return Err(ContractError::InvalidDescriptionLength {});
    }

    // Check if description contains only valid characters
    let re = Regex::new(r"^[A-Za-z0-9\s\.,!?-]{20,1000}$").unwrap();
    if !re.is_match(description) {
        return Err(ContractError::InvalidDescriptionFormat {});
    }

    Ok(())
}

pub fn cancel_market(
    mut deps: DepsMut,
    info: MessageInfo,
    market_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut market = MARKETS.load(deps.storage, market_id)?;

    // Only admin or market creator can cancel the market
    if info.sender != config.admin && info.sender != market.creator {
        return Err(ContractError::Unauthorized {});
    }

    // Can only cancel active markets
    if market.status != MarketStatus::Active {
        return Err(ContractError::InvalidMarketState {});
    }

    market.status = MarketStatus::Canceled;
    MARKETS.save(deps.storage, market_id, &market)?;

    //Implement logic to refund all bets
    let refund_messages = refund_all_bets(&mut deps, market_id)?;

    Ok(Response::new()
        .add_messages(refund_messages)
        .add_attribute("method", "cancel_market")
        .add_attribute("market_id", market_id.to_string()))
}

fn refund_all_bets(deps: &mut DepsMut, market_id: u64) -> Result<Vec<CosmosMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut refund_messages = Vec::new();

    // Refund open and partially filled orders
    let orders: Vec<Order> = ORDERS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|r| {
            let order = r.unwrap().1;
            if order.market_id == market_id && 
               (order.status == OrderStatus::Open || order.status == OrderStatus::PartiallyFilled) {
                Some(order)
            } else {
                None
            }
        })
        .collect();

    for mut order in orders {
        let refund_amount = match order.side {
            OrderSide::Back => order.amount - order.filled_amount,
            OrderSide::Lay => (order.amount - order.filled_amount).multiply_ratio(order.odds - 100, 100u128),
        };

        if refund_amount > Uint128::zero() {
            let refund_msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: order.creator.to_string(),
                amount: vec![Coin {
                    denom: config.token_denom.clone(),
                    amount: refund_amount,
                }],
            });
            refund_messages.push(refund_msg);

            // Update order status
            order.status = OrderStatus::Canceled;
            ORDERS.save(deps.storage, order.id, &order)?;
        }
    }

    // Refund matched bets
    let matched_bets: Vec<MatchedBet> = MATCHED_BETS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|r| {
            let matched_bet = r.unwrap().1;
            if matched_bet.market_id == market_id && !matched_bet.redeemed {
                Some(matched_bet)
            } else {
                None
            }
        })
        .collect();

    for mut matched_bet in matched_bets {
        // Refund back user
        let back_refund_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: matched_bet.back_user.to_string(),
            amount: vec![Coin {
                denom: config.token_denom.clone(),
                amount: matched_bet.amount,
            }],
        });
        refund_messages.push(back_refund_msg);

        // Refund lay user
        let lay_amount = matched_bet.amount.multiply_ratio(matched_bet.odds - 100, 100u128);
        let lay_refund_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: matched_bet.lay_user.to_string(),
            amount: vec![Coin {
                denom: config.token_denom.clone(),
                amount: lay_amount,
            }],
        });
        refund_messages.push(lay_refund_msg);

        // Mark matched bet as redeemed
        matched_bet.redeemed = true;
        MATCHED_BETS.save(deps.storage, matched_bet.id, &matched_bet)?;
    }

    Ok(refund_messages)
}

pub fn close_market(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    market_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut market = MARKETS.load(deps.storage, market_id)?;

    // Only admin can close the market
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Can only close active markets
    if market.status != MarketStatus::Active {
        return Err(ContractError::InvalidMarketState {});
    }

    // Check if the market end time has passed
    if env.block.time.seconds() < market.end_time {
        return Err(ContractError::MarketNotEnded {});
    }

    market.status = MarketStatus::Closed;
    MARKETS.save(deps.storage, market_id, &market)?;

     // Refund unmatched orders
     let refund_messages = refund_unmatched_orders(&mut deps, market_id)?;

    Ok(Response::new()
        .add_messages(refund_messages)
        .add_attribute("method", "close_market")
        .add_attribute("market_id", market_id.to_string()))
}

pub fn refund_unmatched_orders(
    deps: &mut DepsMut,
    market_id: u64,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut refund_messages = Vec::new();

    let orders: Vec<Order> = ORDERS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|r| {
            let order = r.unwrap().1;
            if order.market_id == market_id && 
               (order.status == OrderStatus::Open || order.status == OrderStatus::PartiallyFilled) {
                Some(order)
            } else {
                None
            }
        })
        .collect();

    for mut order in orders {
        let refund_amount = match order.side {
            OrderSide::Back => order.amount - order.filled_amount,
            OrderSide::Lay => (order.amount - order.filled_amount).multiply_ratio(order.odds - 100, 100u128),
        };

        if refund_amount > Uint128::zero() {
            let refund_msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: order.creator.to_string(),
                amount: vec![Coin {
                    denom: config.token_denom.clone(),
                    amount: refund_amount,
                }],
            });
            refund_messages.push(refund_msg);

            // Update order status
            order.status = OrderStatus::Canceled;
            order.amount = order.filled_amount;
            ORDERS.save(deps.storage, order.id, &order)?;
        }
    }

    Ok(refund_messages)
}

pub fn place_order(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    market_id: u64,
    option_id: u8,
    _order_type: OrderType,
    side: OrderSide,
    amount: Uint128,
    odds: u32,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut market = MARKETS.load(deps.storage, market_id)?;

    // Check if market is active
    if market.status != MarketStatus::Active {
        return Err(ContractError::MarketNotActive {});
    }

    // Check if market has ended
    if env.block.time.seconds() > market.end_time {
        market.status = MarketStatus::Closed;
        MARKETS.save(deps.storage, market_id, &market)?;
        // Fix Bug ID #6: Return Ok instead of Err to avoid transaction revert
        return Ok(Response::new()
            .add_attribute("action", "market_closed")
            .add_attribute("market_id", market_id.to_string())
            .add_attribute("message", "Market is closed, no more orders can be placed"));
    }

    // Check if the bet amount is above the minimum
    if amount < config.min_bet {
        println!("amount: {:?}", amount);
        println!("config.min_bet: {:?}", config.min_bet);
        return Err(ContractError::BetTooSmall {});
    }

    // Fix Bug ID #20: Use Rust's range feature for more idiomatic validation
    if !(100..=9900).contains(&odds) {
        return Err(ContractError::InvalidOdds {});
    }

    // Calculate required amount
    let required_amount = match side {
        OrderSide::Back => amount,
        OrderSide::Lay => {
            // Fix Bug ID #1: Ensure required_amount is never zero for Lay orders
            let lay_amount = amount.multiply_ratio(odds - 100, 100u128);
            if lay_amount.is_zero() {
                return Err(ContractError::InvalidOdds {});
            }
            lay_amount
        }
    };

    // Check if sufficient funds are sent
    let sent_funds = info.funds.iter().find(|coin| coin.denom == config.token_denom)
        .ok_or(ContractError::NoFundsSent {})?;
    if sent_funds.amount < required_amount {
        return Err(ContractError::InsufficientFunds {});
    }

    // Create new order
    let order_id = ORDER_COUNT.load(deps.storage)? + 1;
    let order = Order {
        id: order_id,
        market_id,
        creator: info.sender.clone(),
        option_id,
        side,
        amount,
        odds,
        filled_amount: Uint128::zero(),
        status: OrderStatus::Open,
        timestamp: env.block.time.seconds(),
    };

    // Save the order
    ORDERS.save(deps.storage, order_id, &order)?;
    ORDER_COUNT.save(deps.storage, &order_id)?;

    // Match the order
    let (matched_amount, matched_bets) = match_orders(&mut deps, &env, &order)?;

    // Update order status based on matching result
    let mut updated_order = ORDERS.load(deps.storage, order_id)?;
    if matched_amount == amount {
        updated_order.status = OrderStatus::Filled;
        updated_order.amount = matched_amount;  // Set amount to matched amount
    } else if matched_amount > Uint128::zero() {
        updated_order.status = OrderStatus::PartiallyFilled;
    }
    updated_order.filled_amount = matched_amount;
    ORDERS.save(deps.storage, order_id, &updated_order)?;

    // If there's any excess funds, return them
    let excess_funds = sent_funds.amount - required_amount;
    let mut response = Response::new()
        .add_attribute("method", "place_order")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("matched_amount", matched_amount.to_string())
        .add_attribute("remaining_matched_bets", matched_bets.len().to_string());

    if excess_funds > Uint128::zero() {
        let refund_msg = BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![Coin {
                denom: config.token_denom,
                amount: excess_funds,
            }],
        };
        response = response.add_message(refund_msg);
    }

    Ok(response)
}

pub fn cancel_order(
    deps: DepsMut,
    info: MessageInfo,
    order_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut order = ORDERS.load(deps.storage, order_id)?;

    // Check if the order belongs to the sender
    if order.creator != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // Check if the order can be cancelled
    if order.status != OrderStatus::Open && order.status != OrderStatus::PartiallyFilled {
        return Err(ContractError::OrderNotCancellable {});
    }

    // Calculate refund amount
    let refund_amount = match order.side {
        OrderSide::Back => order.amount - order.filled_amount,
        OrderSide::Lay => (order.amount - order.filled_amount).multiply_ratio(order.odds - 100, 100u128),
    };

    // Update order status
    order.status = OrderStatus::Canceled;
    order.amount = order.filled_amount;  // Set the amount to the filled amount
    ORDERS.save(deps.storage, order_id, &order)?;

    // Prepare refund message
    let refund_msg = BankMsg::Send {
        to_address: order.creator.to_string(),
        amount: vec![Coin {
            denom: config.token_denom,
            amount: refund_amount,
        }],
    };

    Ok(Response::new()
        .add_message(refund_msg)
        .add_attribute("method", "cancel_order")
        .add_attribute("order_id", order_id.to_string())
        .add_attribute("refund_amount", refund_amount.to_string()))
}

pub fn match_orders(deps: &mut DepsMut, env: &Env, new_order: &Order) -> Result<(Uint128, Vec<MatchedBet>), ContractError> {
    let mut matched_amount = Uint128::zero();
    let mut matched_bets = Vec::new();
    let opposite_side = if new_order.side == OrderSide::Back { OrderSide::Lay } else { OrderSide::Back };
    let mut orders: Vec<Order> = ORDERS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|r| {
            let order = r.unwrap().1;
            if order.market_id == new_order.market_id && 
               order.option_id == new_order.option_id && 
               order.side == opposite_side &&
               (order.status == OrderStatus::Open || order.status == OrderStatus::PartiallyFilled) &&
               order.amount > order.filled_amount {
                Some(order)
            } else {
                None
            }
        })
        .collect();

    // Sort orders based on the new order's side
    orders.sort_by(|a, b| {
        match new_order.side {
            OrderSide::Back => a.odds.cmp(&b.odds).then_with(|| a.timestamp.cmp(&b.timestamp)),
            OrderSide::Lay => b.odds.cmp(&a.odds).then_with(|| a.timestamp.cmp(&b.timestamp)),
        }
    });


    for order in &mut orders {
        if matched_amount == new_order.amount {
            break;
        }

        // Check if the odds are favorable for matching
        let odds_match = match new_order.side {
            OrderSide::Back => new_order.odds <= order.odds,
            OrderSide::Lay => new_order.odds >= order.odds,
        };

        if !odds_match {
            continue;
        }

        let available_amount = order.amount - order.filled_amount;
        let match_amount = std::cmp::min(new_order.amount - matched_amount, available_amount);

        let matched_bet_id = MATCHED_BET_COUNT.load(deps.storage)? + 1;
        let matched_bet = MatchedBet {
            id: matched_bet_id,
            market_id: new_order.market_id,
            option_id: new_order.option_id,
            amount: match_amount,
            odds: order.odds,
            timestamp: env.block.time.seconds(),
            back_user: if new_order.side == OrderSide::Back { new_order.creator.clone() } else { order.creator.clone() },
            lay_user: if new_order.side == OrderSide::Lay { new_order.creator.clone() } else { order.creator.clone() },
            redeemed: false,
        };

        MATCHED_BETS.save(deps.storage, matched_bet_id, &matched_bet)?;
        MATCHED_BET_COUNT.save(deps.storage, &matched_bet_id)?;
        matched_bets.push(matched_bet);

        matched_amount += match_amount;
        order.filled_amount += match_amount;

        if order.filled_amount == order.amount {
            order.status = OrderStatus::Filled;
        } else {
            order.status = OrderStatus::PartiallyFilled;
        }

        // Fix Bug ID #21: Remove unnecessary reference creation
        ORDERS.save(deps.storage, order.id, order)?;
    }

    let mut updated_new_order = new_order.clone();
    updated_new_order.filled_amount = matched_amount;
    if matched_amount == new_order.amount {
        updated_new_order.status = OrderStatus::Filled;
    } else if matched_amount > Uint128::zero() {
        updated_new_order.status = OrderStatus::PartiallyFilled;
    }
    ORDERS.save(deps.storage, new_order.id, &updated_new_order)?;

    Ok((matched_amount, matched_bets))
}

pub fn redeem_winnings(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    matched_bet_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut matched_bet = MATCHED_BETS.load(deps.storage, matched_bet_id)?;
    let market = MARKETS.load(deps.storage, matched_bet.market_id)?;

    // Check if market is resolved
    if market.status != MarketStatus::Resolved {
        return Err(ContractError::MarketNotResolved {});
    }

    // Check if bet is already redeemed
    if matched_bet.redeemed {
        return Err(ContractError::AlreadyRedeemed {});
    }

    // Check if the caller is the winner
    let is_winner = matched_bet.option_id == market.result.unwrap();
    if (is_winner && matched_bet.back_user != info.sender) || (!is_winner && matched_bet.lay_user != info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    // Calculate winnings
    let winnings = if is_winner {
        matched_bet.amount.multiply_ratio(matched_bet.odds, 100u128)
    } else {
        matched_bet.amount
    };

    // Mark bet as redeemed
    matched_bet.redeemed = true;
    MATCHED_BETS.save(deps.storage, matched_bet_id, &matched_bet)?;

    // Send winnings
    let send_msg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            denom: config.token_denom,
            amount: winnings,
        }],
    };

    Ok(Response::new()
        .add_message(send_msg)
        .add_attribute("method", "redeem_winnings")
        .add_attribute("matched_bet_id", matched_bet_id.to_string())
        .add_attribute("winnings", winnings.to_string()))
}


pub fn add_to_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    address: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only admin can add to whitelist
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    // Fix Bug ID #9: Validate and normalize the address
    let validated_address = deps.api.addr_validate(&address.to_string().to_lowercase())?;

    // Check if the address is already whitelisted
    if WHITELISTED_ADDRESSES.may_load(deps.storage, validated_address.clone())?.is_some() {
        return Err(ContractError::AlreadyWhitelisted {});
    }

    WHITELISTED_ADDRESSES.save(deps.storage, validated_address.clone(), &true)?;

    Ok(Response::new()
        .add_attribute("method", "add_to_whitelist")
        .add_attribute("address", validated_address.to_string()))
}

pub fn remove_from_whitelist(
    deps: DepsMut,
    info: MessageInfo,
    address: Addr,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Only admin can remove from whitelist
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }
     // Fix Bug ID #9: Validate and normalize the address
    let validated_address = deps.api.addr_validate(&address.to_string().to_lowercase())?;

    WHITELISTED_ADDRESSES.remove(deps.storage, validated_address.clone());

    Ok(Response::new()
        .add_attribute("method", "remove_from_whitelist")
        .add_attribute("address", validated_address.to_string()))
}

pub fn propose_market_result(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    market_id: u64,
    proposed_result: u8,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut market = MARKETS.load(deps.storage, market_id)?;

    // Fix Bug ID #5: Ensure market is already properly closed
    if market.status != MarketStatus::Closed {
        return Err(ContractError::InvalidMarketState {});
    }

    // Fix Bug ID #3: Check if market is canceled
    if market.status == MarketStatus::Canceled {
        return Err(ContractError::InvalidMarketState {});
    }

    // Check if the market end time has passed
    if env.block.time.seconds() <= market.end_time {
        return Err(ContractError::MarketNotEnded {});
    }

    // Fix Bug ID #4: Check if any proposal exists regardless of status
    if PROPOSALS.may_load(deps.storage, market_id)?.is_some() {
        return Err(ContractError::ProposalAlreadyExists {});
    }

    // Check if the correct bond amount is sent
    let sent_funds = info.funds.iter().find(|coin| coin.denom == config.token_denom);
    if sent_funds.is_none() || sent_funds.unwrap().amount != market.resolution_bond {
        return Err(ContractError::IncorrectBondAmount {});
    }

    // Create and save proposal
    let proposal = ResolutionProposal {
        market_id,
        proposer: info.sender.clone(),
        proposed_result,
        bond_amount: market.resolution_bond,
        proposal_time: env.block.time.seconds(),
        challenge_deadline: env.block.time.seconds() + config.challenging_period,
        status: ProposalStatus::Active,
    };
    PROPOSALS.save(deps.storage, market_id, &proposal)?;

    // Update market status
    market.status = MarketStatus::Closed;
    MARKETS.save(deps.storage, market_id, &market)?;

    Ok(Response::new()
        .add_attribute("method", "propose_market_result")
        .add_attribute("market_id", market_id.to_string())
        .add_attribute("proposed_result", proposed_result.to_string())
        .add_attribute("proposer", info.sender.to_string())
        .add_attribute("bond_amount", market.resolution_bond.to_string()))
}

pub fn raise_dispute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    market_id: u64,
    proposed_outcome: u8,
    evidence: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut market = MARKETS.load(deps.storage, market_id)?;
    let mut proposal = PROPOSALS.load(deps.storage, market_id)?;

    // Check if proposal is active
    if proposal.status != ProposalStatus::Active {
        return Err(ContractError::InvalidProposalState {});
    }

    // Check if within challenging period
    if env.block.time.seconds() > proposal.challenge_deadline {
        return Err(ContractError::ChallengePeriodEnded {});
    }

    // Check if the correct bond amount is sent
    let sent_funds = info.funds.iter().find(|coin| coin.denom == config.token_denom);
    if sent_funds.is_none() || sent_funds.unwrap().amount != market.resolution_bond {
        return Err(ContractError::IncorrectBondAmount {});
    }

    // Create and save dispute
    let dispute = Dispute {
        market_id,
        challenger: info.sender.clone(),
        proposed_outcome,
        evidence,
        status: DisputeStatus::Active,
        created_at: env.block.time.seconds(),
    };
    DISPUTES.save(deps.storage, market_id, &dispute)?;

    // Update proposal status
    proposal.status = ProposalStatus::Challenged;
    PROPOSALS.save(deps.storage, market_id, &proposal)?;

    market.status = MarketStatus::InDispute;
    MARKETS.save(deps.storage, market_id, &market)?;

    Ok(Response::new()
        .add_attribute("method", "raise_dispute")
        .add_attribute("market_id", market_id.to_string())
        .add_attribute("proposed_outcome", proposed_outcome.to_string())
        .add_attribute("challenger", info.sender.to_string())
        .add_attribute("bond_amount", market.resolution_bond.to_string()))
}

pub fn cast_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    market_id: u64,
    vote: u8,
) -> Result<Response, ContractError> {
    // Fix Bug ID #7: Validate vote is either 0 or 1
    if vote > 1 {
        return Err(ContractError::InvalidVote {});
    }

    let config = CONFIG.load(deps.storage)?;
    let market = MARKETS.load(deps.storage, market_id)?;
    let dispute = DISPUTES.load(deps.storage, market_id)?;

    // Check if voter is whitelisted
    if !WHITELISTED_ADDRESSES.has(deps.storage, info.sender.clone()) {
        return Err(ContractError::NotWhitelisted {});
    }

    // Check if market is in disputed state
    if market.status != MarketStatus::InDispute {
        return Err(ContractError::InvalidMarketState {});
    }

    // Check if within voting period
    if env.block.time.seconds() > dispute.created_at + config.voting_period {
        return Err(ContractError::VotingPeriodEnded {});
    }

    // Check if voter has already voted
    if VOTES.has(deps.storage, (market_id, info.sender.clone())) {
        return Err(ContractError::AlreadyVoted {});
    }

    // Save vote
    let vote_record = Vote {
        voter: info.sender.clone(),
        option_id: vote,
    };
    VOTES.save(deps.storage, (market_id, info.sender.clone()), &vote_record)?;

    // Update vote count
    VOTE_COUNTS.update(deps.storage, (market_id, vote), |count| -> StdResult<u64> {
        Ok(count.unwrap_or(0) + 1)
    })?;

    Ok(Response::new()
        .add_attribute("method", "cast_vote")
        .add_attribute("market_id", market_id.to_string())
        .add_attribute("voter", info.sender)
        .add_attribute("vote", vote.to_string()))
}

pub fn resolve_dispute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    market_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let mut market = MARKETS.load(deps.storage, market_id)?;
    let mut proposal = PROPOSALS.load(deps.storage, market_id)?;

    // Only admin can resolve disputes
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let mut messages = vec![];
    let mut attributes = vec![
        ("method".to_string(), "resolve_dispute".to_string()),
        ("market_id".to_string(), market_id.to_string()),
    ];

    if proposal.status == ProposalStatus::Active {
        // No dispute case
        if env.block.time.seconds() <= proposal.challenge_deadline {
            return Err(ContractError::ChallengePeriodNotEnded {});
        }

        // Resolve in favor of the proposer
        market.status = MarketStatus::Resolved;
        market.result = Some(proposal.proposed_result);
        proposal.status = ProposalStatus::Resolved;

        // Send reward to proposer
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: proposal.proposer.to_string(),
            amount: vec![Coin {
                denom: config.token_denom.clone(),
                amount: market.resolution_reward,
            }],
        }));

        attributes.push(("result".to_string(), "proposal_accepted".to_string()));
        attributes.push(("winner".to_string(), proposal.proposer.to_string()));
    } else if proposal.status == ProposalStatus::Challenged {
        // Disputed case
        let dispute = DISPUTES.load(deps.storage, market_id)?;
        if env.block.time.seconds() <= proposal.challenge_deadline + config.voting_period {
            return Err(ContractError::VotingPeriodNotEnded {});
        }

        // Count votes
        let (_votes, vote_counts) = query_votes(deps.as_ref(), market_id)?;
        let winning_outcome = vote_counts.iter().max_by_key(|&(_, count)| count).map(|&(outcome, _)| outcome)
            .ok_or(ContractError::NoVotes {})?;

        market.status = MarketStatus::Resolved;
        market.result = Some(winning_outcome);
        proposal.status = ProposalStatus::Resolved;

        // Determine the winner and send reward
        let winner = if winning_outcome == proposal.proposed_result {
            proposal.proposer.clone()
        } else {
            dispute.challenger.clone()
        };

        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: winner.to_string(),
            amount: vec![Coin {
                denom: config.token_denom.clone(),
                amount: market.resolution_reward,
            }],
        }));

        attributes.push(("result".to_string(), "dispute_resolved".to_string()));
        attributes.push(("winner".to_string(), winner.to_string()));
        attributes.push(("winning_outcome".to_string(), winning_outcome.to_string()));
    } else {
        return Err(ContractError::InvalidProposalState {});
    }

    MARKETS.save(deps.storage, market_id, &market)?;
    PROPOSALS.save(deps.storage, market_id, &proposal)?;

    Ok(Response::new()
        .add_messages(messages)
        .add_attributes(attributes))
}

pub fn redeem_bond_amount(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    market_id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let market = MARKETS.load(deps.storage, market_id)?;
    let proposal = PROPOSALS.load(deps.storage, market_id)?;

    // Check if the market is resolved
    if market.status != MarketStatus::Resolved {
        return Err(ContractError::MarketNotResolved {});
    }

    // Check if the caller is either the proposer or the challenger
    let is_proposer = info.sender == proposal.proposer;
    let is_challenger = DISPUTES.may_load(deps.storage, market_id)?.map_or(false, |d| info.sender == d.challenger);

    if !is_proposer && !is_challenger {
        return Err(ContractError::Unauthorized {});
    }

    // Check if the caller is the correct proposer
    let is_winner = if is_proposer {
        market.result == Some(proposal.proposed_result)
    } else {
        market.result != Some(proposal.proposed_result)
    };

    if !is_winner {
        return Err(ContractError::NotWinner {});
    }

    // Send the bond amount to the winner
    let msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            denom: config.token_denom,
            amount: market.resolution_bond,
        }],
    });

    Ok(Response::new()
        .add_message(msg)
        .add_attributes(vec![
            ("method".to_string(), "redeem_bond_amount".to_string()),
            ("market_id".to_string(), market_id.to_string()),
            ("recipient".to_string(), info.sender.to_string()),
            ("amount".to_string(), market.resolution_bond.to_string()),
        ]))
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    Ok(Response::default())
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { field, value } => update_config(deps, info, field, value),
        ExecuteMsg::CreateMarket { category, question, description, options, start_time, end_time, resolution_bond, resolution_reward } => 
            create_market(deps, env, info, category, question, description, options, start_time, end_time, resolution_bond, resolution_reward),
        ExecuteMsg::CancelMarket { market_id } => cancel_market(deps, info, market_id),
        ExecuteMsg::CloseMarket { market_id } => close_market(deps, env, info, market_id),
        ExecuteMsg::ProposeResult { market_id, winning_outcome } => propose_market_result(deps, env, info, market_id, winning_outcome),
        ExecuteMsg::PlaceOrder { market_id, option_id, order_type, side, amount, odds } => 
            place_order(deps, env, info, market_id, option_id, order_type, side, amount, odds),
        ExecuteMsg::CancelOrder { order_id } => cancel_order(deps, info, order_id),
        ExecuteMsg::RedeemWinnings { matched_bet_id } => redeem_winnings(deps, env, info, matched_bet_id),
        ExecuteMsg::AddToWhitelist { address } => add_to_whitelist(deps, info, address),
        ExecuteMsg::RemoveFromWhitelist { address } => remove_from_whitelist(deps, info, address),
        ExecuteMsg::RaiseDispute { market_id, proposed_outcome, evidence } => 
            raise_dispute(deps, env, info, market_id, proposed_outcome, evidence),
        ExecuteMsg::CastVote { market_id, outcome } => cast_vote(deps, env, info, market_id, outcome),
        ExecuteMsg::ResolveDispute { market_id } => resolve_dispute(deps, env, info, market_id),
        ExecuteMsg::RedeemBondAmount { market_id } => redeem_bond_amount(deps, env, info, market_id), // Fix Bug ID #2
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Market { market_id } => to_json_binary(&query_market(deps, market_id)?),
        QueryMsg::Markets { status, start_after, limit } => to_json_binary(&query_markets(deps, status, start_after, limit)?),
        QueryMsg::Order { order_id } => to_json_binary(&query_order(deps, order_id)?),
        QueryMsg::UserOrders { user, market_id, start_after, limit } => to_json_binary(&query_user_orders(deps, user, market_id, start_after, limit)?),
        QueryMsg::MarketOrders { market_id, side, start_after, limit } => to_json_binary(&query_market_orders(deps, market_id, side, start_after, limit)?),
        QueryMsg::MatchedBets { market_id, user, start_after, limit } => to_json_binary(&query_matched_bets(deps, market_id, user, start_after, limit)?),
        QueryMsg::ResolutionProposal { market_id } => to_json_binary(&query_resolution_proposal(deps, market_id)?),
        QueryMsg::Dispute { market_id } => to_json_binary(&query_dispute(deps, market_id)?),
        QueryMsg::Votes { market_id } => to_json_binary(&query_votes(deps, market_id)?),
        QueryMsg::IsWhitelisted { user } => to_json_binary(&query_is_whitelisted(deps, user)?),
        QueryMsg::MarketStatistics { market_id } => to_json_binary(&query_market_statistics(deps, market_id)?),
        QueryMsg::WhitelistedAddresses { start_after, limit } => to_json_binary(&query_whitelisted_addresses(deps, start_after, limit)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_market(deps: Deps, market_id: u64) -> StdResult<Market> {
    MARKETS.load(deps.storage, market_id)
}

fn query_markets(deps: Deps, status: Option<String>, start_after: Option<u64>, limit: Option<u32>) -> StdResult<Vec<Market>> {
    let limit = limit.unwrap_or(30) as usize;
    let start = start_after.map(Bound::exclusive);
    
    MARKETS
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .filter(|r| {
            if let Ok((_, market)) = r {
                if let Some(status_str) = &status {
                    market.status.to_string() == *status_str
                } else {
                    true
                }
            } else {
                false
            }
        })
        .take(limit)
        .map(|item| item.map(|(_, market)| market))
        .collect()
}

fn query_order(deps: Deps, order_id: u64) -> StdResult<Order> {
    ORDERS.load(deps.storage, order_id)
}

fn query_user_orders(deps: Deps, user: Addr, market_id: Option<u64>, start_after: Option<u64>, limit: Option<u32>) -> StdResult<Vec<Order>> {
    let limit = limit.unwrap_or(30) as usize;
    let start = start_after.map(Bound::exclusive);
    
    ORDERS
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .filter(|r| {
            if let Ok((_, order)) = r {
                order.creator == user && market_id.map_or(true, |id| order.market_id == id)
            } else {
                false
            }
        })
        .take(limit)
        .map(|item| item.map(|(_, order)| order))
        .collect()
}


fn query_is_whitelisted(deps: Deps, user: Addr) -> StdResult<bool> {
    Ok(WHITELISTED_ADDRESSES.may_load(deps.storage, user)?.unwrap_or(false))
}

fn query_market_statistics(deps: Deps, market_id: u64) -> StdResult<MarketStatistics> {
    let market = MARKETS.load(deps.storage, market_id)?;
    
    let total_volume: Uint128 = MATCHED_BETS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter_map(|r| {
            r.ok().and_then(|(_, matched_bet)| {
                if matched_bet.market_id == market_id {
                    Some(matched_bet.amount)
                } else {
                    None
                }
            })
        })
        .sum();

    let order_count = ORDERS
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .filter(|r| r.as_ref().map(|(_, order)| order.market_id == market_id).unwrap_or(false))
        .count();

    Ok(MarketStatistics {
        market_id,
        total_volume,
        order_count: order_count as u64,
        status: market.status,
    })
}

pub fn query_market_orders(
    deps: Deps,
    market_id: u64,
    side: Option<String>,
    start_after: Option<u64>,
    limit: Option<u32>
) -> StdResult<Vec<Order>> {
    let limit = limit.unwrap_or(30) as usize;
    let start = start_after.map(Bound::exclusive);

    ORDERS
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .filter(|r| {
            if let Ok((_, order)) = r {
                order.market_id == market_id && 
                side.as_ref().map_or(true, |s| order.side.to_string() == *s) &&
                (order.status == OrderStatus::Open || order.status == OrderStatus::PartiallyFilled) &&
                order.amount > order.filled_amount
            } else {
                false
            }
        })
        .take(limit)
        .map(|item| item.map(|(_, order)| order))
        .collect()
}

pub fn query_matched_bets(
    deps: Deps,
    market_id: Option<u64>,
    user: Option<Addr>,
    start_after: Option<u64>,
    limit: Option<u32>
) -> StdResult<Vec<MatchedBet>> {
    let limit = limit.unwrap_or(30) as usize;
    let start = start_after.map(Bound::exclusive);

    MATCHED_BETS
        .range(deps.storage, start, None, cosmwasm_std::Order::Ascending)
        .filter(|r| {
            if let Ok((_, matched_bet)) = r {
                market_id.map_or(true, |id| matched_bet.market_id == id) &&
                user.as_ref().map_or(true, |addr| matched_bet.back_user == *addr || matched_bet.lay_user == *addr)
            } else {
                false
            }
        })
        .take(limit)
        .map(|item| item.map(|(_, matched_bet)| matched_bet))
        .collect()
}

pub fn query_resolution_proposal(deps: Deps, market_id: u64) -> StdResult<Option<ResolutionProposal>> {
    PROPOSALS.may_load(deps.storage, market_id)
}

pub fn query_dispute(deps: Deps, market_id: u64) -> StdResult<Option<Dispute>> {
    DISPUTES.may_load(deps.storage, market_id)
}

pub fn query_votes(deps: Deps, market_id: u64) -> StdResult<(Vec<Vote>, Vec<(u8, u64)>)> {
    let votes: Vec<Vote> = VOTES
        .prefix(market_id)
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|item| item.map(|(_, vote)| vote))
        .collect::<StdResult<Vec<Vote>>>()?;

    // Fix Bug ID #22: Remove unnecessary identity mapping
    let vote_counts: Vec<(u8, u64)> = VOTE_COUNTS
        .prefix(market_id)
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .collect::<StdResult<Vec<(u8, u64)>>>()?;

    Ok((votes, vote_counts))
}

fn query_whitelisted_addresses(deps: Deps, start_after: Option<String>, limit: Option<u32>) -> StdResult<Vec<String>> {
    let start = start_after.map(|s| Addr::unchecked(s));
    let limit = limit.unwrap_or(30) as usize;

    let addresses: Vec<String> = WHITELISTED_ADDRESSES
        .range(deps.storage, start.map(Bound::exclusive), None, cosmwasm_std::Order::Ascending)
        .take(limit)
        .filter_map(|item| item.ok().map(|(addr, _)| addr.to_string()))
        .collect();

    Ok(addresses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{Addr, Uint128, Coin, DepsMut, from_json};
    use cosmwasm_std::Timestamp;

    const ADMIN: &str = "admin";
    const USER1: &str = "user1";
    const USER2: &str = "user2";
    const USER3: &str = "user3";
    const TOKEN_DENOM: &str = "utoken";

    fn setup_contract(deps: DepsMut) {
        let msg = InstantiateMsg {
            admin: Addr::unchecked(ADMIN),
            token_denom: TOKEN_DENOM.to_string(),
            platform_fee: Uint128::new(100),  // 1%
            treasury: Addr::unchecked("treasury"),
            challenging_period: 86400,  // 1 day
            voting_period: 86400,  // 1 day
            min_bet: Uint128::new(1000),
            whitelist_enabled: false,
        };
        let info = mock_info(ADMIN, &[]);
        let _ = instantiate(deps, mock_env(), info, msg).unwrap();
    }

    #[test]
    fn test_instantiation() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());
        
        // Query the config to check if it's set correctly
        let config: Config = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config.admin, Addr::unchecked(ADMIN));
        assert_eq!(config.token_denom, TOKEN_DENOM);
        assert_eq!(config.platform_fee, Uint128::new(100));
    }

    #[test]
    fn test_update_config() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Test updating config with valid parameters
        let msg = ExecuteMsg::UpdateConfig {
            field: "platform_fee".to_string(),
            value: "200".to_string(),
        };
        let info = mock_info(ADMIN, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(3, res.attributes.len());

        // Query the config to check if it's updated
        let config: Config = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config.platform_fee, Uint128::new(200));

        // Test updating config with unauthorized user
        let msg = ExecuteMsg::UpdateConfig {
            field: "platform_fee".to_string(),
            value: "300".to_string(),
        };
        let info = mock_info(USER1, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(res.is_err());
    }

    #[test]
    fn test_whitelist_management() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Test adding to whitelist
        let msg = ExecuteMsg::AddToWhitelist { 
            address: Addr::unchecked(USER1),
        };
        let info = mock_info(ADMIN, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(2, res.attributes.len());

        // Check if user is whitelisted
        let res: bool = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::IsWhitelisted { user: Addr::unchecked(USER1) }).unwrap()).unwrap();
        assert!(res);

        // Test removing from whitelist
        let msg = ExecuteMsg::RemoveFromWhitelist { 
            address: Addr::unchecked(USER1),
        };
        let info = mock_info(ADMIN, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(2, res.attributes.len());

        // Check if user is no longer whitelisted
        let res: bool = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::IsWhitelisted { user: Addr::unchecked(USER1) }).unwrap()).unwrap();
        assert!(!res);

        // Test unauthorized whitelist management
        let msg = ExecuteMsg::AddToWhitelist { 
            address: Addr::unchecked(USER2),
        };
        let info = mock_info(USER1, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(res.is_err());
    }

    #[test]
    fn test_create_market() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "1000000".to_string(),
            end_time: "2000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(3, res.attributes.len());

        // Query the market to check if it's created correctly
        let res: Market = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::Market { market_id: 1 }).unwrap()).unwrap();
        assert_eq!(res.description, "World Cup Final");
        assert_eq!(res.options, vec!["Team A".to_string(), "Team B".to_string()]);
    }
    #[test]
    fn test_remove_from_whitelist() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // First, add a user to the whitelist
        let msg = ExecuteMsg::AddToWhitelist { address: Addr::unchecked(USER1) };
        let info = mock_info(ADMIN, &[]);
        let _ = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Now, remove the user from the whitelist
        let msg = ExecuteMsg::RemoveFromWhitelist { address: Addr::unchecked(USER1) };
        let info = mock_info(ADMIN, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(2, res.attributes.len());

        // Verify that the user is no longer whitelisted
        let res: bool = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::IsWhitelisted { user: Addr::unchecked(USER1) }).unwrap()).unwrap();
        assert!(!res);
    }

    #[test]
    fn test_unauthorized_whitelist_modification() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Attempt to add to whitelist by non-admin
        let msg = ExecuteMsg::AddToWhitelist { address: Addr::unchecked(USER2) };
        let info = mock_info(USER1, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(res.is_err());

        // Attempt to remove from whitelist by non-admin
        let msg = ExecuteMsg::RemoveFromWhitelist { address: Addr::unchecked(USER2) };
        let info = mock_info(USER1, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(res.is_err());
    }

    #[test]
    fn test_create_market_valid_params() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "1000000".to_string(),
            end_time: "2000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(3, res.attributes.len());

        // Query the market to check if it's created correctly
        let res: Market = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::Market { market_id: 1 }).unwrap()).unwrap();
        assert_eq!(res.description, "World Cup Final");
        assert_eq!(res.options, vec!["Team A".to_string(), "Team B".to_string()]);
    }

    #[test]
    fn test_create_market_invalid_params() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        let msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "Invalid Time Range".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "2000000".to_string(),  // Start time after end time
            end_time: "1000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        assert!(res.is_err());
    }

    #[test]
    fn test_cancel_market() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // First, create a market
        let create_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final match details".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "1000000".to_string(),
            end_time: "2000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let _ = execute(deps.as_mut(), mock_env(), info, create_msg).unwrap();

        // Now, cancel the market
        let cancel_msg = ExecuteMsg::CancelMarket { market_id: 1 };
        let info = mock_info(ADMIN, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, cancel_msg).unwrap();
        assert_eq!(2, res.attributes.len());

        // Verify that the market is canceled
        let res: Market = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::Market { market_id: 1 }).unwrap()).unwrap();
        assert_eq!(res.status, MarketStatus::Canceled);
    }

    #[test]
    fn test_close_market() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // First, create a market
        let create_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final match details".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "1000000".to_string(),
            end_time: "2000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let _ = execute(deps.as_mut(), mock_env(), info, create_msg).unwrap();

        // Now, close the market
        let close_msg = ExecuteMsg::CloseMarket { market_id: 1 };
        let info = mock_info(ADMIN, &[]);
        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(2000001);  // Set time after end_time
        let res = execute(deps.as_mut(), env, info, close_msg).unwrap();
        assert_eq!(2, res.attributes.len());

        // Verify that the market is closed
        let res: Market = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::Market { market_id: 1 }).unwrap()).unwrap();
        assert_eq!(res.status, MarketStatus::Closed);
    }

    #[test]
    fn test_unauthorized_market_modification() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // First, create a market
        let create_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final match details".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "1000000".to_string(),
            end_time: "2000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let _ = execute(deps.as_mut(), mock_env(), info, create_msg).unwrap();

        // Attempt to cancel the market by non-admin
        let cancel_msg = ExecuteMsg::CancelMarket { market_id: 1 };
        let info = mock_info(USER1, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, cancel_msg);
        assert!(res.is_err());
    }

    #[test]
    fn test_place_valid_order() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // First, create a market
        let create_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "100000000".to_string(),
            end_time: "10000000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let _ = execute(deps.as_mut(), mock_env(), info, create_msg).unwrap();

        // Now, place an order
        let place_order_msg = ExecuteMsg::PlaceOrder { 
            market_id: 1,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Back,
            amount: Uint128::new(1000),
            odds: 150,
        };
        let info = mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000) }]);
        let res = execute(deps.as_mut(), mock_env(), info, place_order_msg).unwrap();
        assert!(res.attributes.len() > 0);

        // Verify that the order was placed
        let res: Vec<Order> = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::UserOrders { user: Addr::unchecked(USER1), market_id: Some(1), start_after: None, limit: None }).unwrap()).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].amount, Uint128::new(1000));
        assert_eq!(res[0].odds, 150);
    }

    #[test]
    fn test_place_order_insufficient_funds() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Create a market
        let create_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final match details".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "1000000".to_string(),
            end_time: "2000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let _ = execute(deps.as_mut(), mock_env(), info, create_msg).unwrap();

        // Attempt to place an order with insufficient funds
        let place_order_msg = ExecuteMsg::PlaceOrder { 
            market_id: 1,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Back,
            amount: Uint128::new(2000),  // Amount greater than min_bet
            odds: 150,
        };
        let info = mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000) }]);  // Insufficient funds
        let res = execute(deps.as_mut(), mock_env(), info, place_order_msg);
        assert!(res.is_err());
    }

    #[test]
    fn test_place_order_closed_market() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Create and close a market
        let create_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final match details".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "1000000".to_string(),
            end_time: "2000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let _ = execute(deps.as_mut(), mock_env(), info.clone(), create_msg).unwrap();

        let close_msg = ExecuteMsg::CloseMarket { market_id: 1 };
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(2000001);  // Set time after end_time
        let _ = execute(deps.as_mut(), env.clone(), info, close_msg).unwrap();

        // Attempt to place an order on the closed market
        let place_order_msg = ExecuteMsg::PlaceOrder { 
            market_id: 1,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Back,
            amount: Uint128::new(1000),
            odds: 150,
        };
        let info = mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000) }]);
        let res = execute(deps.as_mut(), env, info, place_order_msg);
        assert!(res.is_err());
    }

    #[test]
    fn test_order_matching() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Create a market
        let create_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "1000000".to_string(),
            end_time: "20000000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let _ = execute(deps.as_mut(), mock_env(), info, create_msg).unwrap();

        // Place a back order
        let back_order_msg = ExecuteMsg::PlaceOrder { 
            market_id: 1,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Back,
            amount: Uint128::new(1000),
            odds: 150,
        };
        let info = mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000) }]);
        let _ = execute(deps.as_mut(), mock_env(), info, back_order_msg).unwrap();

        // Place a matching lay order (full match)
        let lay_order_msg = ExecuteMsg::PlaceOrder { 
            market_id: 1,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Lay,
            amount: Uint128::new(1000),
            odds: 150,
        };
        let info = mock_info(USER2, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(500) }]);
        let res = execute(deps.as_mut(), mock_env(), info, lay_order_msg).unwrap();
        
        // Check that the orders were matched
        assert!(res.attributes.iter().any(|attr| attr.key == "matched_amount" && attr.value == "1000"));

        // Query matched bets
        let res: Vec<MatchedBet> = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::MatchedBets { market_id: Some(1), user: None, start_after: None, limit: None }).unwrap()).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].amount, Uint128::new(1000));
    }

    #[test]
    fn test_cancel_order() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Create a market and place an order
        let create_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "1000000".to_string(),
            end_time: "20000000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let _ = execute(deps.as_mut(), mock_env(), info, create_msg).unwrap();

        let place_order_msg = ExecuteMsg::PlaceOrder { 
            market_id: 1,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Back,
            amount: Uint128::new(1000),
            odds: 150,
        };
        let info = mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000) }]);
        let _ = execute(deps.as_mut(), mock_env(), info.clone(), place_order_msg).unwrap();

        // Cancel the order
        let cancel_order_msg = ExecuteMsg::CancelOrder { order_id: 1 };
        let res = execute(deps.as_mut(), mock_env(), info, cancel_order_msg).unwrap();
        
        // Check that the order was canceled
        assert!(res.attributes.iter().any(|attr| attr.key == "method" && attr.value == "cancel_order"));

        // Query the order to verify its status
        let res: Order = from_json(&query(deps.as_ref(), mock_env(), QueryMsg::Order { order_id: 1 }).unwrap()).unwrap();
        assert_eq!(res.status, OrderStatus::Canceled);
    }

    #[test]
    fn test_propose_result_closed_market() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        setup_contract(deps.as_mut());
    
        // Create a market
        let create_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final match details".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "1000000".to_string(),
            end_time: "2000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let _ = execute(deps.as_mut(), env.clone(), info.clone(), create_msg).unwrap();
    
        // Close the market
        env.block.time = env.block.time.plus_seconds(1001);  // Move time past end_time
        let close_msg = ExecuteMsg::CloseMarket { market_id: 1 };
        let _ = execute(deps.as_mut(), env.clone(), info, close_msg).unwrap();
    
        // Propose a result
        let propose_msg = ExecuteMsg::ProposeResult { 
            market_id: 1, 
            winning_outcome: 0 
        };
        let info = mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000000) }]);
        let res = execute(deps.as_mut(), env, info, propose_msg).unwrap();
        
        assert_eq!(res.attributes.len(), 5);
        assert_eq!(res.attributes[0].value, "propose_market_result");
        assert_eq!(res.attributes[1].value, "1");  // market_id
        assert_eq!(res.attributes[2].value, "0");  // proposed_result
        assert_eq!(res.attributes[3].value, USER1);  // proposer
        assert_eq!(res.attributes[4].value, "1000000");  // bond_amount
    }

    #[test]
    fn test_propose_result_active_market() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        setup_contract(deps.as_mut());

        // Create an active market
        let create_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: env.block.time.seconds().to_string(),
            end_time: (env.block.time.seconds() + 1000).to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let _ = execute(deps.as_mut(), env.clone(), info, create_msg).unwrap();

        // Attempt to propose a result for an active market
        let propose_msg = ExecuteMsg::ProposeResult { 
            market_id: 1, 
            winning_outcome: 0 
        };
        let info = mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000000) }]);
        let res = execute(deps.as_mut(), env, info, propose_msg);
        
        // Check that the proposal was rejected
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), ContractError::MarketNotEnded {});
    }

    #[test]
    fn test_raise_dispute() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        setup_contract(deps.as_mut());

        // Create, close a market, and propose a result
        let create_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final match details".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "1000000".to_string(),
            end_time: "2000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let _ = execute(deps.as_mut(), env.clone(), info.clone(), create_msg).unwrap();

        env.block.time = env.block.time.plus_seconds(1001);  // Move time past end_time
        let close_msg = ExecuteMsg::CloseMarket { market_id: 1 };
        let _ = execute(deps.as_mut(), env.clone(), info, close_msg).unwrap();

        let propose_msg = ExecuteMsg::ProposeResult { market_id: 1, winning_outcome: 0 };
        let info = mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000000) }]);
        let _ = execute(deps.as_mut(), env.clone(), info, propose_msg).unwrap();

        // Raise a dispute
        let dispute_msg = ExecuteMsg::RaiseDispute { 
            market_id: 1, 
            proposed_outcome: 1,
            evidence: "Evidence for Team B winning".to_string()
        };
        let info = mock_info(USER2, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000000) }]);
        let res = execute(deps.as_mut(), env, info, dispute_msg).unwrap();
        
        assert_eq!(res.attributes.len(), 5);
        assert_eq!(res.attributes[0].value, "raise_dispute");
        assert_eq!(res.attributes[1].value, "1");  // market_id
        assert_eq!(res.attributes[2].value, "1");  // proposed_outcome
    }

    #[test]
    fn test_vote_on_dispute() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        setup_contract(deps.as_mut());

        // Create, close a market, propose a result, and raise a dispute
        let create_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final match details".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: "1000000".to_string(),
            end_time: "2000000".to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let _ = execute(deps.as_mut(), env.clone(), info.clone(), create_msg).unwrap();

        env.block.time = env.block.time.plus_seconds(1001);  // Move time past end_time
        let close_msg = ExecuteMsg::CloseMarket { market_id: 1 };
        let _ = execute(deps.as_mut(), env.clone(), info.clone(), close_msg).unwrap();

        let propose_msg = ExecuteMsg::ProposeResult { market_id: 1, winning_outcome: 0 };
        let info = mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000000) }]);
        let _ = execute(deps.as_mut(), env.clone(), info, propose_msg).unwrap();

        let dispute_msg = ExecuteMsg::RaiseDispute { 
            market_id: 1, 
            proposed_outcome: 1,
            evidence: "Evidence for Team B winning".to_string()
        };
        let info = mock_info(USER2, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000000) }]);
        let _ = execute(deps.as_mut(), env.clone(), info, dispute_msg).unwrap();

        // Add USER3 to whitelist
        let whitelist_msg = ExecuteMsg::AddToWhitelist { address: Addr::unchecked(USER3) };
        let info = mock_info(ADMIN, &[]);
        let _ = execute(deps.as_mut(), env.clone(), info, whitelist_msg).unwrap();

        // Cast a vote
        let vote_msg = ExecuteMsg::CastVote { 
            market_id: 1, 
            outcome: 1 
        };
        let info = mock_info(USER3, &[]);
        let res = execute(deps.as_mut(), env.clone(), info, vote_msg).unwrap();
        
        assert_eq!(res.attributes.len(), 4);
        assert_eq!(res.attributes[0].value, "cast_vote");
        assert_eq!(res.attributes[1].value, "1");  // market_id
        assert_eq!(res.attributes[2].value, "user3");  // voter
        assert_eq!(res.attributes[3].value, "1");  // vote
    }

    fn create_active_market(deps: DepsMut, env: Env) -> u64 {
        let msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: env.block.time.seconds().to_string(),
            end_time: (env.block.time.seconds() + 10000).to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let info = mock_info(ADMIN, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let res = execute(deps, env, info, msg).unwrap();
        res.attributes.iter().find(|attr| attr.key == "market_id").unwrap().value.parse().unwrap()
    }

    #[test]
    fn test_resolve_dispute() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        setup_contract(deps.as_mut());

        let market_id = create_active_market(deps.as_mut(), env.clone());

        // Close the market
        env.block.time = env.block.time.plus_seconds(10001);
        let close_msg = ExecuteMsg::CloseMarket { market_id };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]), close_msg).unwrap();

        // Propose a result
        let propose_msg = ExecuteMsg::ProposeResult { market_id, winning_outcome: 0 };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000000) }]), propose_msg).unwrap();

        // Raise a dispute
        let dispute_msg = ExecuteMsg::RaiseDispute { 
            market_id, 
            proposed_outcome: 1,
            evidence: "Evidence for Team B winning".to_string()
        };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(USER2, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000000) }]), dispute_msg).unwrap();

        // Add users to whitelist and cast votes
        for user in &[USER1, USER2, USER3] {
            let whitelist_msg = ExecuteMsg::AddToWhitelist { address: Addr::unchecked(user.clone()) };
            let _ = execute(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]), whitelist_msg).unwrap();

            let vote_msg = ExecuteMsg::CastVote { market_id, outcome: 1 };
            let _ = execute(deps.as_mut(), env.clone(), mock_info(user, &[]), vote_msg).unwrap();
        }

        // Move time past voting period
        env.block.time = env.block.time.plus_seconds(186401);

        // Resolve dispute
        let resolve_msg = ExecuteMsg::ResolveDispute { market_id };
        let res = execute(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]), resolve_msg).unwrap();

        assert!(res.attributes.iter().any(|attr| attr.key == "method" && attr.value == "resolve_dispute"));
        assert!(res.attributes.iter().any(|attr| attr.key == "winner" && attr.value == USER2));

        // Check market status
        let market: Market = from_json(&query(deps.as_ref(), env, QueryMsg::Market { market_id }).unwrap()).unwrap();
        assert_eq!(market.status, MarketStatus::Resolved);
        assert_eq!(market.result, Some(1));
    }

    #[test]
    fn test_redeem_winnings() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        setup_contract(deps.as_mut());

        let market_id = create_active_market(deps.as_mut(), env.clone());

        // Place back bet
        let back_bet_msg = ExecuteMsg::PlaceOrder { 
            market_id,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Back,
            amount: Uint128::new(1000),
            odds: 150,
        };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000) }]), back_bet_msg).unwrap();

        // Place lay bet to ensure matching
        let lay_bet_msg = ExecuteMsg::PlaceOrder { 
            market_id,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Lay,
            amount: Uint128::new(1000),
            odds: 150,
        };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(USER2, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(500) }]), lay_bet_msg).unwrap();

        // Query matched bets to get the correct matched_bet_id
        let matched_bets: Vec<MatchedBet> = from_json(&query(deps.as_ref(), env.clone(), QueryMsg::MatchedBets { market_id: Some(market_id), user: None, start_after: None, limit: None }).unwrap()).unwrap();
        assert!(!matched_bets.is_empty(), "No matched bets found");
        let matched_bet_id = matched_bets[0].id;

        // Close market and resolve
        env.block.time = env.block.time.plus_seconds(10001);
        let close_msg = ExecuteMsg::CloseMarket { market_id };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]), close_msg).unwrap();

        let propose_msg = ExecuteMsg::ProposeResult { market_id, winning_outcome: 0 };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(USER3, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000000) }]), propose_msg).unwrap();

        env.block.time = env.block.time.plus_seconds(86401);
        let resolve_msg = ExecuteMsg::ResolveDispute { market_id };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]), resolve_msg).unwrap();

        // Redeem winnings for USER1
        let redeem_msg = ExecuteMsg::RedeemWinnings { matched_bet_id };
        let res = execute(deps.as_mut(), env.clone(), mock_info(USER1, &[]), redeem_msg).unwrap();
        assert!(res.attributes.iter().any(|attr| attr.key == "method" && attr.value == "redeem_winnings"));

        // Try to redeem again (should fail)
        let redeem_msg = ExecuteMsg::RedeemWinnings { matched_bet_id };
        let res = execute(deps.as_mut(), env.clone(), mock_info(USER1, &[]), redeem_msg);
        assert!(res.is_err());

        // USER2 tries to redeem (should fail as they lost)
        let redeem_msg = ExecuteMsg::RedeemWinnings { matched_bet_id };
        let res = execute(deps.as_mut(), env, mock_info(USER2, &[]), redeem_msg);
        assert!(res.is_err());
    }

    #[test]
    fn test_queries() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        setup_contract(deps.as_mut());

        let market_id = create_active_market(deps.as_mut(), env.clone());

        // Test valid queries
        let config: Config = from_json(&query(deps.as_ref(), env.clone(), QueryMsg::Config {}).unwrap()).unwrap();
        assert_eq!(config.admin, Addr::unchecked(ADMIN));

        let market: Market = from_json(&query(deps.as_ref(), env.clone(), QueryMsg::Market { market_id }).unwrap()).unwrap();
        assert_eq!(market.id, market_id);

        // Test invalid query
        let res = query(deps.as_ref(), env, QueryMsg::Market { market_id: 999 });
        assert!(res.is_err());
    }

    #[test]
    fn test_edge_cases() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        setup_contract(deps.as_mut());

        // Test market with no bets
        let market_id = create_active_market(deps.as_mut(), env.clone());
        env.block.time = env.block.time.plus_seconds(10001);
        let close_msg = ExecuteMsg::CloseMarket { market_id };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]), close_msg).unwrap();

        let propose_msg = ExecuteMsg::ProposeResult { market_id, winning_outcome: 0 };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000000) }]), propose_msg).unwrap();

        env.block.time = env.block.time.plus_seconds(86401);
        let resolve_msg = ExecuteMsg::ResolveDispute { market_id };
        let res = execute(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]), resolve_msg);
        assert!(res.is_ok());

        // Test placing order with extreme odds
        let market_id = create_active_market(deps.as_mut(), env.clone());
        let place_bet_msg = ExecuteMsg::PlaceOrder { 
            market_id,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Back,
            amount: Uint128::new(1000),
            odds: 9900, // Maximum allowed odds
        };
        let res = execute(deps.as_mut(), env, mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000) }]), place_bet_msg);
        assert!(res.is_ok());
    }

    #[test]
    fn test_whitelist() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        setup_contract(deps.as_mut());

        // Enable whitelist
        let update_config_msg = ExecuteMsg::UpdateConfig { 
            field: "whitelist_enabled".to_string(), 
            value: "true".to_string() 
        };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]), update_config_msg).unwrap();

        // Try to create market with non-whitelisted user (should fail)
        let create_market_msg = ExecuteMsg::CreateMarket { 
            category: "Sports".to_string(),
            question: "Who will win the World Cup Final?".to_string(), // New field
            description: "World Cup Final".to_string(),
            options: vec!["Team A".to_string(), "Team B".to_string()],
            start_time: env.block.time.seconds().to_string(),
            end_time: (env.block.time.seconds() + 10000).to_string(),
            resolution_bond: Uint128::new(1000000),
            resolution_reward: Uint128::new(500000),
        };
        let res = execute(deps.as_mut(), env.clone(), mock_info(USER1, &[]), create_market_msg.clone());
        assert!(res.is_err());

        // Whitelist USER1
        let whitelist_msg = ExecuteMsg::AddToWhitelist { address: Addr::unchecked(USER1) };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]), whitelist_msg).unwrap();

        // Create market with whitelisted user (should succeed)
        let info = mock_info(USER1, &[Coin {
            denom: TOKEN_DENOM.to_string(),
            amount: Uint128::new(500000),
        }]);
        let res = execute(deps.as_mut(), env, info, create_market_msg);
        assert!(res.is_ok());
    }

    #[test]
    fn test_time_based_functions() {
        let mut deps = mock_dependencies();
        let mut env = mock_env();
        setup_contract(deps.as_mut());

        let market_id = create_active_market(deps.as_mut(), env.clone());

        // Try to close market before end time (should fail)
        let close_msg = ExecuteMsg::CloseMarket { market_id };
        let res = execute(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]), close_msg.clone());
        assert!(res.is_err());

        // Move time past end time and close market (should succeed)
        env.block.time = env.block.time.plus_seconds(10001);
        let res = execute(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]), close_msg);
        assert!(res.is_ok());

        // Propose result
        let propose_msg = ExecuteMsg::ProposeResult { market_id, winning_outcome: 0 };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(1000000) }]), propose_msg).unwrap();

        // Try to resolve before challenge period ends (should fail)
        let resolve_msg = ExecuteMsg::ResolveDispute { market_id };
        let res = execute(deps.as_mut(), env.clone(), mock_info(ADMIN, &[]), resolve_msg.clone());
        assert!(res.is_err());

        // Move time past challenge period and resolve (should succeed)
        env.block.time = env.block.time.plus_seconds(86401);
        let res = execute(deps.as_mut(), env, mock_info(ADMIN, &[]), resolve_msg);
        assert!(res.is_ok());
    }

    #[test]
    fn test_order_matching_with_different_odds() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        setup_contract(deps.as_mut());

        // Create a market
        let market_id = create_active_market(deps.as_mut(), env.clone());

        // Place a back order at 2.2 odds
        let back_order_msg = ExecuteMsg::PlaceOrder { 
            market_id,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Back,
            amount: Uint128::new(70000000),
            odds: 220, // 2.2 in percentage format
        };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(70000000) }]), back_order_msg).unwrap();

        // Place a lay order at 1.5 odds
        let lay_order_msg = ExecuteMsg::PlaceOrder { 
            market_id,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Lay,
            amount: Uint128::new(50000000),
            odds: 150, // 1.5 in percentage format
        };
        let res = execute(deps.as_mut(), env.clone(), mock_info(USER2, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(25000000) }]), lay_order_msg).unwrap();

        // Check that the orders were not matched
        assert_eq!(
            res.attributes
                .iter()
                .find(|attr| attr.key == "matched_amount")
                .map(|attr| attr.value.as_str()),
            Some("0")
        );

        // Query the order book to verify both orders are still open
        let res: Vec<Order> = from_json(&query(deps.as_ref(), env.clone(), QueryMsg::MarketOrders { market_id, side: None, start_after: None, limit: None }).unwrap()).unwrap();
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].side, OrderSide::Back);
        assert_eq!(res[0].odds, 220);
        assert_eq!(res[0].amount, Uint128::new(70000000));
        assert_eq!(res[0].status, OrderStatus::Open);
        assert_eq!(res[1].side, OrderSide::Lay);
        assert_eq!(res[1].odds, 150);
        assert_eq!(res[1].amount, Uint128::new(50000000));
        assert_eq!(res[1].status, OrderStatus::Open);

        // Now place a matching lay order at 2.2 odds
        let matching_lay_order_msg = ExecuteMsg::PlaceOrder { 
            market_id,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Lay,
            amount: Uint128::new(70000000),
            odds: 220, // 2.2 in percentage format
        };
        let res = execute(deps.as_mut(), env.clone(), mock_info(USER3, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(84000000) }]), matching_lay_order_msg).unwrap();

        // Check that the orders were matched
        assert_eq!(
            res.attributes
                .iter()
                .find(|attr| attr.key == "matched_amount")
                .map(|attr| attr.value.as_str()),
            Some("70000000")
        );

        // Query the order book to verify the back order is filled and the lay order at 1.5 is still open
        let res: Vec<Order> = from_json(&query(deps.as_ref(), env, QueryMsg::MarketOrders { market_id, side: None, start_after: None, limit: None }).unwrap()).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].side, OrderSide::Lay);
        assert_eq!(res[0].odds, 150);
        assert_eq!(res[0].amount, Uint128::new(50000000));
        assert_eq!(res[0].status, OrderStatus::Open);
    }

    #[test]
    fn test_multiple_order_matching() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        setup_contract(deps.as_mut());

        let market_id = create_active_market(deps.as_mut(), env.clone());

        // Place first back order at 2.2 odds, amount 100
        let back_order_msg1 = ExecuteMsg::PlaceOrder { 
            market_id,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Back,
            amount: Uint128::new(100_000_000),
            odds: 220,
        };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(100_000_000) }]), back_order_msg1).unwrap();

        // Place lay order at 1.7 odds, amount 10
        let lay_order_msg = ExecuteMsg::PlaceOrder { 
            market_id,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Lay,
            amount: Uint128::new(10_000_000),
            odds: 170,
        };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(USER2, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(7_000_000) }]), lay_order_msg).unwrap();

        // Place second back order at 3.0 odds, amount 100
        let back_order_msg2 = ExecuteMsg::PlaceOrder { 
            market_id,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Back,
            amount: Uint128::new(100_000_000),
            odds: 300,
        };
        let res = execute(deps.as_mut(), env.clone(), mock_info(USER3, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(100_000_000) }]), back_order_msg2).unwrap();

        // Check that the second back order was not matched with the lay order
        assert_eq!(
            res.attributes
                .iter()
                .find(|attr| attr.key == "matched_amount")
                .map(|attr| attr.value.as_str()),
            Some("0")
        );

        // Query the order book to verify all orders are still open
        let res: Vec<Order> = from_json(&query(deps.as_ref(), env, QueryMsg::MarketOrders { market_id, side: None, start_after: None, limit: None }).unwrap()).unwrap();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0].side, OrderSide::Back);
        assert_eq!(res[0].odds, 220);
        assert_eq!(res[0].amount, Uint128::new(100_000_000));
        assert_eq!(res[0].status, OrderStatus::Open);
        assert_eq!(res[1].side, OrderSide::Lay);
        assert_eq!(res[1].odds, 170);
        assert_eq!(res[1].amount, Uint128::new(10_000_000));
        assert_eq!(res[1].status, OrderStatus::Open);
        assert_eq!(res[2].side, OrderSide::Back);
        assert_eq!(res[2].odds, 300);
        assert_eq!(res[2].amount, Uint128::new(100_000_000));
        assert_eq!(res[2].status, OrderStatus::Open);
    }

    #[test]
    fn test_successful_order_matching() {
        let mut deps = mock_dependencies();
        let env = mock_env();
        setup_contract(deps.as_mut());

        let market_id = create_active_market(deps.as_mut(), env.clone());

        // Place a back order at 2.0 odds
        let back_order_msg = ExecuteMsg::PlaceOrder { 
            market_id,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Back,
            amount: Uint128::new(100000000),
            odds: 200,
        };
        let _ = execute(deps.as_mut(), env.clone(), mock_info(USER1, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(100000000) }]), back_order_msg).unwrap();

        // Place a matching lay order at 2.1 odds
        let lay_order_msg = ExecuteMsg::PlaceOrder { 
            market_id,
            option_id: 0,
            order_type: OrderType::Limit,
            side: OrderSide::Lay,
            amount: Uint128::new(50000000),
            odds: 210,
        };
        let res = execute(deps.as_mut(), env.clone(), mock_info(USER2, &[Coin { denom: TOKEN_DENOM.to_string(), amount: Uint128::new(55000000) }]), lay_order_msg).unwrap();

        // Check that the lay order was fully matched
        assert_eq!(
            res.attributes
                .iter()
                .find(|attr| attr.key == "matched_amount")
                .map(|attr| attr.value.as_str()),
            Some("50000000")
        );

        // Query the order book to verify the remaining orders
        let res: Vec<Order> = from_json(&query(deps.as_ref(), env, QueryMsg::MarketOrders { market_id, side: None, start_after: None, limit: None }).unwrap()).unwrap();
        
        // We expect 1 order: the partially filled back order
        assert_eq!(res.len(), 1, "Expected 1 order, got {}", res.len());
        
        // Check the remaining back order
        assert_eq!(res[0].side, OrderSide::Back);
        assert_eq!(res[0].odds, 200);
        assert_eq!(res[0].amount, Uint128::new(100000000));
        assert_eq!(res[0].filled_amount, Uint128::new(50000000));
        assert_eq!(res[0].status, OrderStatus::PartiallyFilled);
    }
}