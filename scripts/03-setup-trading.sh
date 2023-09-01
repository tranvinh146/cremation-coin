#!/bin/bash

WALLET=$1
STORAGE_PATH=$2

if [ -z "$WALLET" ]; then
    echo "Wallet address is required"
    exit 1
fi

if [ -z "$STORAGE_PATH" ]; then
    echo "Storage path is required"
    exit 1
fi

OWNER="owner"
TXFLAG="--chain-id localterra --gas auto --gas-adjustment 1.2"
CREMAT_TOKEN_ADDR=$(cat $STORAGE_PATH/cremation/cremation-contracts.json | jq -r '.token_addr')
CREMAT_LOCK_ADDR=$(cat $STORAGE_PATH/cremation/cremation-contracts.json | jq -r '.lock_addr')
TERRASWAP_FACTORY_ADDR=$(cat $STORAGE_PATH/terraswap/terraswap-contracts.json | jq -r '.factory_addr')
TERRASWAP_ROUTER_ADDR=$(cat $STORAGE_PATH/terraswap/terraswap-contracts.json | jq -r '.router_addr')

# ===== Step 1: Create Pair =====
echo -e "\nCreating pair: CREMATLUNC - LUNC..."

LUNC_ASSET="{\"amount\":\"0\",\"info\":{\"native_token\":{\"denom\":\"uluna\"}}}"
CREMAT_ASSET="{\"amount\":\"0\",\"info\":{\"token\":{\"contract_addr\":\"$CREMAT_TOKEN_ADDR\"}}}"

CREATE_PAIR_ARGS="{\"create_pair\":{\"assets\":[$LUNC_ASSET,$CREMAT_ASSET]}}"
CREATE_PAIR_TX=$(terrad tx wasm execute $TERRASWAP_FACTORY_ADDR $CREATE_PAIR_ARGS --from $WALLET $TXFLAG -y --output json)
CREATE_PAIR_TX_HASH=$(echo $CREATE_PAIR_TX | jq -r .txhash)
echo "Create pair tx hash: $CREATE_PAIR_TX_HASH"

sleep 6
PAIR_ADDR=$(terrad query tx $CREATE_PAIR_TX_HASH --output json | jq -r '.logs[0].events[] | select(.type == "wasm") | .attributes[] | select (.key == "pair_contract_addr") | .value')
LIQUIDITY_TOKEN_ADDR=$(terrad query tx $CREATE_PAIR_TX_HASH --output json | jq -r '.logs[0].events[] | select(.type == "wasm") | .attributes[] | select (.key == "liquidity_token_addr") | .value' | head -n 1)
echo "{\"pair_addr\":\"$PAIR_ADDR\",\"liquidity_token_addr\":\"$LIQUIDITY_TOKEN_ADDR\"}" > $STORAGE_PATH/cremation/trading-pair-contracts.json
echo -e "\nStored trading pair contracts in $STORAGE_PATH/cremation/trading-pair-contracts.json"


# ===== Step 2: Set Config Cremation Token =====
echo -e "\nSetting config cremation token..."
SET_CONFIG_ARGS="{\"set_config\":{\"terraswap_router\":\"$TERRASWAP_ROUTER_ADDR\",\"terraswap_pair\":\"$PAIR_ADDR\"}}"
SET_CONFIG_TX=$(terrad tx wasm execute $CREMAT_TOKEN_ADDR $SET_CONFIG_ARGS --from $WALLET $TXFLAG -y --output json)
SET_CONFIG_TX_HASH=$(echo $SET_CONFIG_TX | jq -r .txhash)
echo "Set config tx hash: $SET_CONFIG_TX_HASH"
sleep 6

# ===== Step 3: Provide Liquidity =====
PAIR_ADDR=$(cat $STORAGE_PATH/cremation/trading-pair-contracts.json | jq -r '.pair_addr')
LIQUIDITY_TOKEN_ADDR=$(cat $STORAGE_PATH/cremation/trading-pair-contracts.json | jq -r '.liquidity_token_addr')

QUERY_OWNER_BALANCE_ARGS="{\"balance\":{\"address\":\"$(terrad keys show $OWNER -a)\"}}"
OWNER_BALANCE=$(terrad query wasm contract-state smart $CREMAT_TOKEN_ADDR $QUERY_OWNER_BALANCE_ARGS --output json | jq -r .data.balance)

# Allowance CREMAT token for pair contract
echo -e "\nAllow Pair contract to spend owner's CREMAT token..."
INCREASE_ALLOWANCE_ARGS="{\"increase_allowance\":{\"spender\":\"$PAIR_ADDR\",\"amount\":\"$OWNER_BALANCE\"}}"
INCREASE_ALLOWANCE_TX=$(terrad tx wasm execute $CREMAT_TOKEN_ADDR $INCREASE_ALLOWANCE_ARGS --from $OWNER $TXFLAG -y --output json)
INCREASE_ALLOWANCE_TX_HASH=$(echo $INCREASE_ALLOWANCE_TX | jq -r .txhash)
echo "Increase allowance tx hash: $INCREASE_ALLOWANCE_TX_HASH"
sleep 6

# Provide Liquidity
echo -e "\nProviding liquidity..."
LUNC_AMOUNT=100000000
PROVIDE_LIQUIDITY_ARGS="{\"provide_liquidity\":{\"assets\":[{\"amount\":\"$LUNC_AMOUNT\",\"info\":{\"native_token\":{\"denom\":\"uluna\"}}},{\"amount\":\"$OWNER_BALANCE\",\"info\":{\"token\":{\"contract_addr\":\"$CREMAT_TOKEN_ADDR\"}}}]}}"
PROVIDE_LIQUIDITY_TX=$(terrad tx wasm execute $PAIR_ADDR $PROVIDE_LIQUIDITY_ARGS --from $OWNER --amount ${LUNC_AMOUNT}uluna $TXFLAG -y --output json)
PROVIDE_LIQUIDITY_TX_HASH=$(echo $PROVIDE_LIQUIDITY_TX | jq -r .txhash)
echo "Provide liquidity tx hash: $PROVIDE_LIQUIDITY_TX_HASH"
sleep 6

# ===== Step 4: Lock Liquidity Token =====
echo -e "\nLocking liquidity token..."
QUERY_OWNER_LIQUIDITY_TOKEN_ARGS="{\"balance\":{\"address\":\"$(terrad keys show $OWNER -a)\"}}"
OWNER_LIQUIDITY_TOKEN=$(terrad query wasm contract-state smart $LIQUIDITY_TOKEN_ADDR $QUERY_OWNER_LIQUIDITY_TOKEN_ARGS --output json | jq -r .data.balance)
echo "Owner liquidity token: $OWNER_LIQUIDITY_TOKEN"
LOCK_LIQUIDITY_ARGS="{\"transfer\":{\"recipient\":\"$CREMAT_LOCK_ADDR\",\"amount\":\"$OWNER_LIQUIDITY_TOKEN\"}}"
LOCK_LIQUIDITY_TX=$(terrad tx wasm execute $LIQUIDITY_TOKEN_ADDR $LOCK_LIQUIDITY_ARGS --from $OWNER $TXFLAG -y --output json)
LOCK_LIQUIDITY_TX_HASH=$(echo $LOCK_LIQUIDITY_TX | jq -r .txhash)
echo "Lock liquidity transaction tx hash: $LOCK_LIQUIDITY_TX_HASH"
sleep 6

QUERY_LOCKED_LIQUIDITY_TOKEN_ARGS="{\"locked_token_amount\":{\"token_address\":\"$LIQUIDITY_TOKEN_ADDR\"}}"
LOCKED_LIQUIDITY_TOKEN=$(terrad query wasm contract-state smart $CREMAT_LOCK_ADDR $QUERY_LOCKED_LIQUIDITY_TOKEN_ARGS --output json | jq -r .data.amount)
echo "Locked liquidity token: $LOCKED_LIQUIDITY_TOKEN"