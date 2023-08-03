#!/bin/bash

WALLET=$1
CREMATION_COIN_PATH=$2

NOT_CARE_ADDR="terra1lx37m2rhekrxh3fhx8edymaf2hq0lqe5gvm5vm"
TXFLAG="--chain-id localterra --gas auto --gas-adjustment 1.2"

if [ -z "$WALLET" ]; then
    echo "Wallet address is required"
    exit 1
fi

if [ -z "$CREMATION_COIN_PATH" ]; then
    echo "Cremation coin path is required"
    exit 1
fi

MAX_ATTEMPTS=2
SLEEP_TIME=5

STORAGE_PATH="store/local/cremation"
mkdir -p $STORAGE_PATH

store_contract_code() {
    CONTRACT_NAME=$1
    TX=$(terrad tx wasm store $CREMATION_COIN_PATH/$CONTRACT_NAME.wasm --from $WALLET $TXFLAG --output json -y)
    TX_HASH=$(echo $TX | jq -r '.txhash')
    echo $TX_HASH
}

write_code_id_to_file() {
    CONTRACT_NAME=$1
    TX=$2
    QUERY=$(terrad query tx $TX --output json)
    CODE_ID=$(echo $QUERY | jq -r '.logs[0].events[] | select(.type == "store_code") | .attributes[] | select(.key == "code_id") | .value')
    CODE_CHECKSUM=$(echo $QUERY | jq -r '.logs[0].events[] | select(.type == "store_code") | .attributes[] | select(.key == "code_checksum") | .value')
    STORE_DATA="{\"tx\":\"$TX\",\"code_id\":$CODE_ID,\"code_checksum\":\"$CODE_CHECKSUM\"}"
    echo $STORE_DATA > $STORAGE_PATH/${CONTRACT_NAME}-store-data.json
}

# ===== Step 1: Store code =====
echo -e "\nStoring code cremation contracts..."
CONTRACTS=("cremation_token" "cremation_lock" "cremation_stake")
TX_HASH_LIST=()
for CONTRACT_NAME in "${CONTRACTS[@]}"
do
    TX_HASH=$(store_contract_code "$CONTRACT_NAME")
    echo "Stored $CONTRACT_NAME contract with tx hash: $TX_HASH"
    TX_HASH_LIST+=("$TX_HASH")
    sleep 4
done

sleep 2

echo -e "\nWriting code info to file..."
for idx in "${!CONTRACTS[@]}"
do
    TX_HASH=${TX_HASH_LIST[$idx]}
    CONTRACT_NAME=${CONTRACTS[$idx]}
    write_code_id_to_file $CONTRACT_NAME $TX_HASH
done
echo -e "\nDone"