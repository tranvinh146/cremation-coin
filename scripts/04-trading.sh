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
TERRASWAP_ROUTER_ADDR=$(cat $STORAGE_PATH/terraswap/terraswap-contracts.json | jq -r '.router_addr')

query_balances() {
    # Query trader CREMAT token 
    QUERY_TRADER_BALANCE_ARGS="{\"balance\":{\"address\":\"$(terrad keys show $WALLET -a)\"}}"
    TRADER_BALANCE=$(terrad query wasm contract-state smart $CREMAT_TOKEN_ADDR $QUERY_TRADER_BALANCE_ARGS --output json | jq -r .data.balance)
    echo "Trader balance: $TRADER_BALANCE"

    # Query tax collector CREMAT token
    QUERY_TAX_COLLECTOR_BALANCE_ARGS="{\"balance\":{\"address\":\"$(terrad keys show $OWNER -a)\"}}"
    TAX_COLLECTOR_BALANCE=$(terrad query wasm contract-state smart $CREMAT_TOKEN_ADDR $QUERY_TAX_COLLECTOR_BALANCE_ARGS --output json | jq -r .data.balance)
    echo "Tax collector balance: $TAX_COLLECTOR_BALANCE"
}

# ====== Buy CREMAT token ======
echo -e "\nBuying CREMAT token..."
BUY_OP_ARGS="{\"execute_swap_operations\":{\"operations\":[{\"terra_swap\":{\"offer_asset_info\":{\"native_token\":{\"denom\":\"uluna\"}},\"ask_asset_info\":{\"token\":{\"contract_addr\":\"$CREMAT_TOKEN_ADDR\"}}}}]}}"
BUY_OP_TX=$(terrad tx wasm execute $TERRASWAP_ROUTER_ADDR $BUY_OP_ARGS --from $WALLET --amount 10000uluna $TXFLAG -y --output json)
BUY_OP_TX_HASH=$(echo $BUY_OP_TX | jq -r '.txhash')
echo "Buy CREMAT token tx hash: $BUY_OP_TX_HASH"
sleep 6

query_balances

# ====== Sell CREMAT token ======
echo -e "\nSelling CREMAT token..."
SWAP_OP=$(echo "{\"execute_swap_operations\":{\"operations\":[{\"terra_swap\":{\"offer_asset_info\":{\"token\":{\"contract_addr\":\"$CREMAT_TOKEN_ADDR\"}},\"ask_asset_info\":{\"native_token\":{\"denom\":\"uluna\"}}}}]}}" | base64 -w 0)
SELL_OP_ARGS="{\"send\":{\"amount\":\"1000\",\"contract\":\"$TERRASWAP_ROUTER_ADDR\",\"msg\":\"$SWAP_OP\"}}"

SELL_OP_TX=$(terrad tx wasm execute $CREMAT_TOKEN_ADDR $SELL_OP_ARGS --from $WALLET $TXFLAG -y --output json)
SELL_OP_TX_HASH=$(echo $SELL_OP_TX | jq -r '.txhash')
echo "Sell CREMAT token tx hash: $SELL_OP_TX_HASH"
sleep 6

query_balances
