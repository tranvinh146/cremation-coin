#!/bin/bash

WALLET=$1
STORAGE_PATH=$2

OWNER="owner"
MOCK_STAKING_CONTRACT="$(terrad keys show $WALLET -a)"
STAKING_CONTRACT=$MOCK_STAKING_CONTRACT
TXFLAG="--chain-id localterra --gas auto --gas-adjustment 1.2"

if [ -z "$WALLET" ]; then
    echo "Wallet is required"
    exit 1
fi

if [ -z "$STORAGE_PATH" ]; then
    echo "Storage path is required"
    exit 1
fi

instantiate_contract() {
    CONTRACT_NAME=$1
    INIT_MSG=$2
    CODE_ID=$(cat $STORAGE_PATH/cremation/${CONTRACT_NAME}-store-data.json | jq -r '.code_id')
    TX=$(terrad tx wasm instantiate $CODE_ID "$INIT_MSG" --admin $(terrad keys show $WALLET -a) --label $CONTRACT_NAME --from $WALLET $TXFLAG --output json -y)
    TX_HASH=$(echo $TX | jq -r '.txhash')
    echo $TX_HASH
}

# instantiate cremation token contract
echo -e "\nInstantiating cremation token contract..."
OWNER="$(terrad keys show $OWNER -a)"
TAX_INFO="{\"buy_tax\": {\"numerator\": \"8\",\"denominator\": \"100\"},\"sell_tax\": {\"numerator\": \"8\",\"denominator\": \"100\"}}"
CW20_INIT_MSG="{\"name\":\"Cremation Token\",\"symbol\":\"CREMATLUNC\",\"decimals\":6,\"initial_balances\":[{\"address\":\"$OWNER\",\"amount\":\"850000000000000000\"}],\"mint\":{\"minter\":\"$STAKING_CONTRACT\",\"cap\":\"1000000000000000000\"}}"

TOKEN_INIT_MSG="{\"owner\":\"$OWNER\",\"tax_info\":$TAX_INFO,\"cw20_instantiate_msg\":$CW20_INIT_MSG}"
TOKEN_INIT_TX_HASH=$(instantiate_contract "cremation_token" "$TOKEN_INIT_MSG")
echo "Instantiated cremation token contract with tx hash: $TOKEN_INIT_TX_HASH"
sleep 6
TOKEN_ADDRESS=$(terrad query tx $TOKEN_INIT_TX_HASH --output json | jq -r '.logs[0].events[] | select(.type == "instantiate") | .attributes[] | select (.key == "_contract_address") | .value')

# instantiate cremation lock contract
echo -e "\nInstantiating cremation lock contract..."
LOCK_INIT_MSG="{\"owner\":\"$OWNER\"}"
LOCK_INIT_TX_HASH=$(instantiate_contract "cremation_lock" "$LOCK_INIT_MSG")
echo "Instantiated cremation lock contract with tx hash: $LOCK_INIT_TX_HASH"
sleep 6
LOCK_ADDRESS=$(terrad query tx $LOCK_INIT_TX_HASH --output json | jq -r '.logs[0].events[] | select(.type == "instantiate") | .attributes[] | select (.key == "_contract_address") | .value')

# instantiate cremation stake contract
echo -e "\nInstantiating cremation stake contract..."
STAKE_INIT_MSG="{\"token_address\":\"$TOKEN_ADDRESS\"}"
STAKE_INIT_TX_HASH=$(instantiate_contract "cremation_stake" "$STAKE_INIT_MSG")
echo "Instantiated cremation lock contract with tx hash: $STAKE_INIT_TX_HASH"
sleep 6
STAKE_ADDRESS=$(terrad query tx $STAKE_INIT_TX_HASH --output json | jq -r '.logs[0].events[] | select(.type == "instantiate") | .attributes[] | select (.key == "_contract_address") | .value')

# update minter to cremation stake contract
echo -e "\nUpdating minter to cremation stake contract..."
UPDATE_MINTER_MSG="{\"update_minter\":{\"new_minter\":\"$STAKE_ADDRESS\"}}"
UPDATE_MINTER_TX=$(terrad tx wasm execute $TOKEN_ADDRESS $UPDATE_MINTER_MSG --from $WALLET $TXFLAG -y --output json)
UPDATE_MINTER_TX_HASH=$(echo $UPDATE_MINTER_TX | jq -r .txhash)
echo "Update minter tx hash: $UPDATE_MINTER_TX_HASH"

# write TOKEN_ADDRESS and LOCK_ADDRESS to file
echo -e "\nWriting contract addresses to file..."
echo "{\"token_addr\":\"$TOKEN_ADDRESS\",\"lock_addr\":\"$LOCK_ADDRESS\",\"stake_addr\":\"$STAKE_ADDRESS\"}" > $STORAGE_PATH/cremation/cremation-contracts.json
echo "Writed contract addresses into \"$STORAGE_PATH/cremation\" directory"