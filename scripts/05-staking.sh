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
STAKE_ADDR=$(cat $STORAGE_PATH/cremation/cremation-contracts.json | jq -r '.stake_addr')

# ====== Stake CREMAT token ======
echo -e "\nStaking CREMAT token..."
STAKE_MSG=$(echo "{\"stake\":{\"staking_period\":\"medium\"}}" | base64 -w 0)
STAKE_ARGS="{\"send\":{\"amount\":\"10000000\",\"contract\":\"$STAKE_ADDR\",\"msg\":\"$STAKE_MSG\"}}"
STAKE_TX=$(terrad tx wasm execute $CREMAT_TOKEN_ADDR $STAKE_ARGS --from $WALLET $TXFLAG -y --output json)
STAKE_TX_HASH=$(echo $STAKE_TX | jq -r '.txhash')
echo "Stake CREMAT token tx hash: $STAKE_TX_HASH"
sleep 6

# query staking status
QUERY_STAKING_STATUS_ARGS="{\"staked\":{\"address\":\"$(terrad keys show $WALLET -a)\"}}"
STAKING_STATUS=$(terrad query wasm contract-state smart $STAKE_ADDR $QUERY_STAKING_STATUS_ARGS --output json | jq -r .data)
echo "Staking status: $STAKING_STATUS"