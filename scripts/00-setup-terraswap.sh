#!/bin/bash

WALLET=$1
TERRSWAP_PATH=$2

NOT_CARE_ADDR="terra1lx37m2rhekrxh3fhx8edymaf2hq0lqe5gvm5vm"
TXFLAG="--chain-id localterra --gas auto --gas-adjustment 1.2"

if [ -z "$WALLET" ]; then
    echo "Wallet address is required"
    exit 1
fi

if [ -z "$TERRSWAP_PATH" ]; then
    echo "Terraswap path is required"
    exit 1
fi

STORAGE_PATH="store/local/terraswap"
mkdir -p $STORAGE_PATH

# store contracts
store_contract_code() {
    CONTRACT_NAME=$1
    TX=$(terrad tx wasm store $TERRSWAP_PATH/$CONTRACT_NAME.wasm --from $WALLET $TXFLAG --output json -y)
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
echo -e "\nStoring code terraswap contracts..."
CONTRACTS=("terraswap_token" "terraswap_pair" "terraswap_factory" "terraswap_router")
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
echo "Writed code info into \"$STORAGE_PATH\" directory"

# ===== Step 2: Instantiate contracts =====
# instantiate terraswap_factory contracts
echo -e "\nInstantiating terraswap_factory contract..."
FACTORY_CODE_ID=$(cat $STORAGE_PATH/terraswap_factory-store-data.json | jq -r '.code_id')
PAIR_CODE_ID=$(cat $STORAGE_PATH/terraswap_pair-store-data.json | jq -r '.code_id')
TOKEN_CODE_ID=$(cat $STORAGE_PATH/terraswap_token-store-data.json | jq -r '.code_id')
FACTORY_INIT_MSG="{\"pair_code_id\":$PAIR_CODE_ID,\"token_code_id\":$TOKEN_CODE_ID}"
FACTORY_INIT_TX=$(terrad tx wasm instantiate $FACTORY_CODE_ID "$FACTORY_INIT_MSG" --no-admin --label "terraswap_factory" --from $WALLET $TXFLAG -y --output json)
FACTORY_INIT_TX_HASH=$(echo $FACTORY_INIT_TX | jq -r .txhash)
echo "Instantiated terraswap_factory contract with tx hash: $FACTORY_INIT_TX_HASH"
sleep 5
FACTORY_ADDR=$(terrad query tx $FACTORY_INIT_TX_HASH --output json | jq -r '.logs[0].events[] | select(.type == "instantiate") | .attributes[] | select (.key == "_contract_address") | .value')

# instantiate terraswap_router contracts
echo -e "\nInstantiating terraswap_router contract..."
ROUTER_CODE_ID=$(cat $STORAGE_PATH/terraswap_router-store-data.json | jq -r '.code_id')
ROUTER_INIT_MSG="{\"terraswap_factory\":\"$FACTORY_ADDR\",\"loop_factory\":\"$NOT_CARE_ADDR\",\"astroport_factory\":\"$NOT_CARE_ADDR\"}"
ROUTER_INIT_TX=$(terrad tx wasm instantiate $ROUTER_CODE_ID "$ROUTER_INIT_MSG" --no-admin --label "terraswap_router" --from $WALLET $TXFLAG -y --output json)
ROUTER_INIT_TX_HASH=$(echo $ROUTER_INIT_TX | jq -r .txhash)
echo "Instantiated terraswap_router contract with tx hash: $ROUTER_INIT_TX_HASH"
sleep 5
ROUTER_ADDR=$(terrad query tx $ROUTER_INIT_TX_HASH --output json | jq -r '.logs[0].events[] | select(.type == "instantiate") | .attributes[] | select (.key == "_contract_address") | .value')

# write FACTOR_ADDR and ROUTER_ADDR into json
echo -e "\nWriting FACTORY_ADDR and ROUTER_ADDR into json..."
echo "{\"factory_addr\":\"$FACTORY_ADDR\",\"router_addr\":\"$ROUTER_ADDR\"}" > $STORAGE_PATH/terraswap-contracts.json
echo "Writed FACTORY_ADDR and ROUTER_ADDR to json"

# ===== Step 3: Add native token decimals =====
echo -e "\nAdding native token decimals (uluna) to terraswap_factory..."
FACTORY_ADDR=$(cat $STORAGE_PATH/terraswap-contracts.json | jq -r '.factory_addr')

# First, send native token to factory
SEND_TOKEN_TX=$(terrad tx bank send $WALLET $FACTORY_ADDR 100uluna --from $WALLET $TXFLAG -y)
sleep 6

TX_AGRS="{\"add_native_token_decimals\":{\"denom\":\"uluna\",\"decimals\":6}}"
TX=$(terrad tx wasm execute $FACTORY_ADDR "$TX_AGRS" --from $WALLET $TXFLAG -y --output json)
TX_HASH=$(echo $TX | jq -r .txhash)
echo "Tx hash: $TX_HASH"

