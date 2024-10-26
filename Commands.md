
comdex init testdev --chain-id test-1
comdex keys add cooluser --recover --keyring-backend test
dinosaur siege capital east curtain cluster puzzle pear alcohol this flag series key bone sponsor trip boss keen swear solid course example foster dial
comdex add-genesis-account cooluser 1000000000000000000000stake,100000000000000000000000000ucmdx,100000000000000000000000ibc/ED07A3391A112B175915CD8FAF43A2DA8E4790EDE12566649D0C2F97716B8518,100000000000000000000000000ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9,10000000000000000000000000000000000000000weth-wei,10000000000000000000000000000ucgold,1000000000000000000000000000ucmst --keyring-backend test
comdex gentx cooluser 1000000000stake --chain-id test-1 --keyring-backend test


minimum-gas-prices = "0ucmdx"
allow cors

comdex collect-gentxs
comdex start


# Comdex Prediction Market v2 Commands and Queries

## Instantiation

## Contract Address
comdex14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9spunaxy
comdex14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9spunaxy
http://95.216.154.108:1317/cosmwasm/wasm/v1/contract/comdex14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9spunaxy/smart/eyJnZXRfY29uZmlnIjoge319
### Instantiate Contract
```bash

comdex tx wasm store exchange_v2.wasm \
--from cooluser --keyring-backend test \
--chain-id test-1  \
-y --fees 200000000ucmdx --gas 100000000



osmosisd tx wasm store artifacts/exchange_v2.wasm \
--from cooluser --keyring-backend test \
--chain-id osmo-test-5  \
-y --fees 2000000uosmo --gas 20000000 --node https://rpc.testnet.osmosis.zone 

comdex tx wasm migrate comdex14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9spunaxy 2 "{}" --from cooluser --keyring-backend test --chain-id test-1 -y --fees 4000000ucmdx

INSTANTIATE_MSG='{
    "admin": "comdex1c0vnmvdpkn8h4phaejmj2kpgqunxcxnjjxd7n3",
    "token_denom": "ucmdx",
    "platform_fee": "100",
    "treasury": "comdex1k0d2q7lpfjchfhlxmajekzmnlsa6q8yqtmch8u",
    "challenging_period": 86400,
    "voting_period": 43200,
    "min_bet": "1000000",
    "whitelist_enabled": true
}'



comdex tx wasm instantiate 1 "$INSTANTIATE_MSG" --from cooluser --keyring-backend test --label "AIB Exchange V2" --gas auto --gas-adjustment 1.3  --chain-id test-1 --admin comdex1c0vnmvdpkn8h4phaejmj2kpgqunxcxnjjxd7n3 -y --fees 200000000ucmdx

11229

6BE562396A549DB59C24283FBC63DF3F218E4398D907B9DDB7913D730064BB0C -store
EBA7790C32ADC0EEAB89FB1D815175C30EF0DDD05F1C70FE3E43F37829DBA000 -instantiate
osmo1vtj94ynfg8mdk4hvr56v7z6u3sxzuxmxypkgmjsvw5r5tx0752ns8hmjvw -contract Address
INSTANTIATE_MSG='{
    "admin": "osmo1c0vnmvdpkn8h4phaejmj2kpgqunxcxnjajuvu5",
    "token_denom": "uosmo",
    "platform_fee": "100",
    "treasury": "osmo1c0vnmvdpkn8h4phaejmj2kpgqunxcxnjajuvu5",
    "challenging_period": 86400,
    "voting_period": 43200,
    "min_bet": "1000000",
    "whitelist_enabled": true
}'
osmosisd tx wasm instantiate 11229 "$INSTANTIATE_MSG" --from cooluser --keyring-backend test --label "Prediction Market Exchange V2" --gas auto --gas-adjustment 1.3  --chain-id osmo-test-5 --admin osmo1c0vnmvdpkn8h4phaejmj2kpgqunxcxnjajuvu5 -y --fees 2000000uosmo --node https://rpc.testnet.osmosis.zone 
```

Note: Replace `[CODE_ID]` with the actual code ID of your uploaded contract, and `[ADMIN_ADDRESS]` with the address of the admin account.

## Configuration

### Update Config
```bash
UPDATE_CONFIG='{
    "update_config": {
        "field": "platform_fee",
        "value": "50"
    }
}'

comdex tx wasm execute [CONTRACT_ADDRESS] "$UPDATE_CONFIG" --from [ADMIN_ADDRESS] --keyring-backend test --gas auto --gas-adjustment 1.3 -y --fees 200000000ucmdx
```

## Market Operations

### Create Market
```bash
CREATE_MARKET='{
    "create_market": {
        "category": "Sports",
        "question": "Which team will win the 2024 FIFA World Cup?",
        "description": "Predict the winner of the 2024 FIFA World Cup Final",
        "options": ["Brazil", "Germany", "France", "Spain"],
        "start_time": "1718182295",
        "end_time": "1718192295",
        "resolution_bond": "100000000",
        "resolution_reward": "5000000"
    }
}'

comdex tx wasm execute comdex14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9spunaxy "$CREATE_MARKET" --from cooluser --keyring-backend test --gas auto --gas-adjustment 1.3 -y --fees 200000000ucmdx
```

### Cancel Market
```bash
CANCEL_MARKET='{
    "cancel_market": {
        "market_id": 1
    }
}'

comdex tx wasm execute [CONTRACT_ADDRESS] "$CANCEL_MARKET" --from [ADMIN_ADDRESS] --keyring-backend test --gas auto --gas-adjustment 1.3 -y --fees 200000000ucmdx
```

### Close Market
```bash
CLOSE_MARKET='{
    "close_market": {
        "market_id": 1
    }
}'

comdex tx wasm execute [CONTRACT_ADDRESS] "$CLOSE_MARKET" --from [ADMIN_ADDRESS] --keyring-backend test --gas auto --gas-adjustment 1.3 -y --fees 200000000ucmdx
```

### Propose Result
```bash
PROPOSE_RESULT='{
    "propose_result": {
        "market_id": 1,
        "winning_outcome": 2
    }
}'

comdex tx wasm execute [CONTRACT_ADDRESS] "$PROPOSE_RESULT" --from [USER_ADDRESS] --keyring-backend test --gas auto --gas-adjustment 1.3 -y --fees 200000000ucmdx
```

## Order Operations

### Place Order
```bash
PLACE_ORDER='{
    "place_order": {
        "market_id": 1,
        "option_id": 0,
        "order_type": "Limit",
        "side": "Back",
        "amount": "1000000",
        "odds": 200
    }
}'

comdex tx wasm execute [CONTRACT_ADDRESS] "$PLACE_ORDER" --amount 1000000ucmdx --from [USER_ADDRESS] --keyring-backend test -y --fees 200000000ucmdx
```

### Cancel Order
```bash
CANCEL_ORDER='{
    "cancel_order": {
        "order_id": 1
    }
}'

comdex tx wasm execute [CONTRACT_ADDRESS] "$CANCEL_ORDER" --from [USER_ADDRESS] --keyring-backend test -y --fees 200000000ucmdx
```

### Redeem Winnings
```bash
REDEEM_WINNINGS='{
    "redeem_winnings": {
        "matched_bet_id": 1
    }
}'

comdex tx wasm execute [CONTRACT_ADDRESS] "$REDEEM_WINNINGS" --from [USER_ADDRESS] --keyring-backend test -y --fees 200000000ucmdx
```

## Whitelist Operations

### Add to Whitelist
```bash
ADD_TO_WHITELIST='{
    "add_to_whitelist": {
        "address": "comdex1..."
    }
}'

comdex tx wasm execute [CONTRACT_ADDRESS] "$ADD_TO_WHITELIST" --from [ADMIN_ADDRESS] --keyring-backend test -y --fees 200000000ucmdx
```

### Remove from Whitelist
```bash
REMOVE_FROM_WHITELIST='{
    "remove_from_whitelist": {
        "address": "comdex1..."
    }
}'

comdex tx wasm execute [CONTRACT_ADDRESS] "$REMOVE_FROM_WHITELIST" --from [ADMIN_ADDRESS] --keyring-backend test -y --fees 200000000ucmdx
```

## Dispute Operations

### Raise Dispute
```bash
RAISE_DISPUTE='{
    "raise_dispute": {
        "market_id": 1,
        "proposed_outcome": 3,
        "evidence": "Evidence supporting the proposed outcome"
    }
}'

comdex tx wasm execute [CONTRACT_ADDRESS] "$RAISE_DISPUTE" --from [USER_ADDRESS] --keyring-backend test -y --fees 200000000ucmdx
```

### Cast Vote
```bash
CAST_VOTE='{
    "cast_vote": {
        "market_id": 1,
        "outcome": 3
    }
}'

comdex tx wasm execute [CONTRACT_ADDRESS] "$CAST_VOTE" --from [USER_ADDRESS] --keyring-backend test -y --fees 200000000ucmdx
```

### Resolve Dispute
```bash
RESOLVE_DISPUTE='{
    "resolve_dispute": {
        "market_id": 1
    }
}'

comdex tx wasm execute [CONTRACT_ADDRESS] "$RESOLVE_DISPUTE" --from [ADMIN_ADDRESS] --keyring-backend test -y --fees 200000000ucmdx
```

## Queries

### Query Config
```bash
QUERY_CONFIG='{
    "config": {}
}'

comdex query wasm contract-state smart comdex14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9spunaxy "$QUERY_CONFIG"
```

### Query Market
```bash
QUERY_MARKET='{
    "market": {
        "market_id": 1
    }
}'

comdex query wasm contract-state smart [CONTRACT_ADDRESS] "$QUERY_MARKET"
```

### Query Markets
```bash
QUERY_MARKETS='{
    "markets": {
        "status": "Active",
        "start_after": 0,
        "limit": 10
    }
}'

comdex query wasm contract-state smart [CONTRACT_ADDRESS] "$QUERY_MARKETS"
```

### Query Order
```bash
QUERY_ORDER='{
    "order": {
        "order_id": 1
    }
}'

comdex query wasm contract-state smart [CONTRACT_ADDRESS] "$QUERY_ORDER"
```

### Query User Orders
```bash
QUERY_USER_ORDERS='{
    "user_orders": {
        "user": "comdex1...",
        "market_id": 1,
        "start_after": 0,
        "limit": 10
    }
}'

comdex query wasm contract-state smart [CONTRACT_ADDRESS] "$QUERY_USER_ORDERS"
```

### Query Market Orders
```bash
QUERY_MARKET_ORDERS='{
    "market_orders": {
        "market_id": 1,
        "side": "Back",
        "start_after": 0,
        "limit": 10
    }
}'

comdex query wasm contract-state smart [CONTRACT_ADDRESS] "$QUERY_MARKET_ORDERS"
```

### Query Matched Bets
```bash
QUERY_MATCHED_BETS='{
    "matched_bets": {
        "market_id": 1,
        "user": "comdex1...",
        "start_after": 0,
        "limit": 10
    }
}'

comdex query wasm contract-state smart [CONTRACT_ADDRESS] "$QUERY_MATCHED_BETS"
```

### Query Resolution Proposal
```bash
QUERY_RESOLUTION_PROPOSAL='{
    "resolution_proposal": {
        "market_id": 1
    }
}'

comdex query wasm contract-state smart [CONTRACT_ADDRESS] "$QUERY_RESOLUTION_PROPOSAL"
```

### Query Dispute
```bash
QUERY_DISPUTE='{
    "dispute": {
        "market_id": 1
    }
}'

comdex query wasm contract-state smart [CONTRACT_ADDRESS] "$QUERY_DISPUTE"
```

### Query Votes
```bash
QUERY_VOTES='{
    "votes": {
        "market_id": 1
    }
}'

comdex query wasm contract-state smart [CONTRACT_ADDRESS] "$QUERY_VOTES"
```

### Query Is Whitelisted
```bash
QUERY_IS_WHITELISTED='{
    "is_whitelisted": {
        "user": "comdex1c0vnmvdpkn8h4phaejmj2kpgqunxcxnjjxd7n3"
    }
}'

comdex query wasm contract-state smart comdex14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9spunaxy "$QUERY_IS_WHITELISTED"
```

### Query Market Statistics
```bash
QUERY_MARKET_STATISTICS='{
    "market_statistics": {
        "market_id": 1
    }
}'

comdex query wasm contract-state smart [CONTRACT_ADDRESS] "$QUERY_MARKET_STATISTICS"
```

### Query Whitelisted Addresses
```bash
QUERY_WHITELISTED_ADDRESSES='{
    "whitelisted_addresses": {
        "start_after": null,
        "limit": 10
    }
}'

comdex query wasm contract-state smart [CONTRACT_ADDRESS] "$QUERY_WHITELISTED_ADDRESSES"
```