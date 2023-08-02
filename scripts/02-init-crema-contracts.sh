#!/bin/bash

WALLET=$1
STORAGE_PATH=$2

OWNER="owner"
MOCK_STAKING_CONTRACT="$(terrad keys show $WALLET -a)"
STAKING_CONTRACT=$MOCK_STAKING_CONTRACT
TXFLAG="--chain-id localterra --gas auto --gas-adjustment 1.2 --admin $(terrad keys show $WALLET -a)"

if [ -z "$WALLET" ]; then
    echo "Wallet address is required"
    exit 1
fi

if [ -z "$STORAGE_PATH" ]; then
    echo "Storage path is required"
    exit 1
fi

# instantiate cremation token contract
echo -e "\nInstantiating cremation token contract..."
TOKEN_CODE_ID=$(cat $STORAGE_PATH/cremation_token-store-data.json | jq -r '.code_id')

OWNER="$(terrad keys show $OWNER -a)"
TAX_INFO="{\"buy_tax\": {\"numerator\": \"8\",\"denominator\": \"100\"},\"sell_tax\": {\"numerator\": \"8\",\"denominator\": \"100\"}}"
CW20_INIT_MSG="{\"name\":\"Cremation Token\",\"symbol\":\"CREMATLUNC\",\"decimals\":6,\"initial_balances\":[{\"address\":\"$OWNER\",\"amount\":\"850000000000000000\"}],\"mint\":{\"minter\":\"$STAKING_CONTRACT\",\"cap\":\"1000000000000000000\"}}"

TOKEN_INIT_MSG="{\"owner\":\"$OWNER\",\"tax_info\":$TAX_INFO,\"cw20_instantiate_msg\":$CW20_INIT_MSG}"
TOKEN_INIT_TX=$(terrad tx wasm instantiate $TOKEN_CODE_ID "$TOKEN_INIT_MSG" --label "cremation_token" --from $WALLET $TXFLAG --output json -y)
TOKEN_INIT_TX_HASH=$(echo $TOKEN_INIT_TX | jq -r '.txhash')
echo "Instantiated cremation token contract with tx hash: $TOKEN_INIT_TX_HASH"
sleep 6
TOKEN_ADDRESS=$(terrad query tx $TOKEN_INIT_TX_HASH --output json | jq -r '.logs[0].events[] | select(.type == "instantiate") | .attributes[] | select (.key == "_contract_address") | .value')

# instantiate cremation lock contract
echo -e "\nInstantiating cremation lock contract..."
LOCK_CODE_ID=$(cat $STORAGE_PATH/cremation_lock-store-data.json | jq -r '.code_id')
LOCK_INIT_MSG="{\"owner\":\"$OWNER\"}"
LOCK_INIT_TX=$(terrad tx wasm instantiate $LOCK_CODE_ID "$LOCK_INIT_MSG" --label "cremation_lock" --from $WALLET $TXFLAG --output json -y)
LOCK_INIT_TX_HASH=$(echo $LOCK_INIT_TX | jq -r '.txhash')
echo "Instantiated cremation lock contract with tx hash: $LOCK_INIT_TX_HASH"
sleep 6
LOCK_ADDRESS=$(terrad query tx $LOCK_INIT_TX_HASH --output json | jq -r '.logs[0].events[] | select(.type == "instantiate") | .attributes[] | select (.key == "_contract_address") | .value')

# write TOKEN_ADDRESS and LOCK_ADDRESS to file
echo -e "\nWriting contract addresses to file..."
echo "{\"token_addr\":\"$TOKEN_ADDRESS\",\"lock_addr\":\"$LOCK_ADDRESS\"}" > $STORAGE_PATH/cremation-contracts.json
echo "Writed contract addresses into \"$STORAGE_PATH\" directory"