#!/bin/bash

WALLET=$1
shift 1

CHAIN_ID="local"
RPC="http://localhost:26657"

while [[ $# -gt 0 ]]; do
    key="$1"

    case $key in
        --network)
            if [ "$2" = "testnet" ]; then
                CHAIN_ID="bajor-1"
                RPC="http://85.214.56.241:26657"
            fi
            shift
            ;;
        --config)
            if [ "$2" = "testnet" ]; then
                CHAIN_ID="bajor-1"
                RPC="http://"
            ;;
        --cremation-lock)
            CREMATION_LOCK_PATH=$2
            shift
            ;;
        --cremation-token)
            CREMATION_TOKEN=$2
            shift
            ;;
        *)
            ;;
    esac

    shift
done

NODE="--node $RPC"
TXFLAG="$NODE --chain-id $CHAIN_ID"

MAX_ATTEMPTS=2
SLEEP_TIME=5

mkdir -p store

if [  ]

if [ -n "$CREMATION_LOCK_PATH" ]; then
    echo -e "\nStoring code cremation-lock..."
    TX=$(terrad tx wasm store $CREMATION_LOCK_PATH --from $WALLET $TXFLAG --output json -y)
    echo $TX
    TX=$(echo $TX | jq -r '.txhash')
    echo "Store cremation lock contract tx hash: $TX"

    attempts=0
    success=false

    while [ $attempts -lt $MAX_ATTEMPTS ] && [ "$success" = false ]; do
        sleep $SLEEP_TIME
        
        QUERY=$(terrad query tx $TX $NODE --output json)
        CODE_ID=$(echo $QUERY | jq -r '.logs[0].events[] | select(.type == "store_code") | .attributes[] | select(.key == "code_id") | .value')
        CODE_CHECKSUM=$(echo $QUERY | jq -r '.logs[0].events[] | select(.type == "store_code") | .attributes[] | select(.key == "code_checksum") | .value')
        STORE_DATA="{\"tx\":\"$TX\",\"code_id\":$CODE_ID,\"code_checksum\":\"$CODE_CHECKSUM\"}"
       
        if [ -n "$CODE_ID" ]; then
            echo $STORE_DATA > store/cremation-lock-store-data.json
            echo "Store info cremation lock contract: ./store/cremation-lock-store-data.json"
            success=true
        else
            attempts=$((attempts + 1))
        fi
    done
    if [ "$success" = false ]; then
        echo "Exceeded maximum attempts. Unable to retrieve tx hash cremation lock contract."
        exit 1
    fi
fi