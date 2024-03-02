use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cremation_token::{msg::AssetInfo, state::FractionFormat};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum Cw20HookMsg {
    Swap {
        ask_asset: AssetInfo,
        swap_paths: Vec<AssetInfo>,
    },
}

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub swap_router: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner {
        new_owner: String,
    },
    UpdateSwapRouter {
        router: String,
    },
    SetTokenBuyTax {
        token_address: String,
        buy_tax: FractionFormat,
    },
    Receive(Cw20ReceiveMsg),
    Swap {
        ask_asset: AssetInfo,
        swap_paths: Vec<AssetInfo>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(OwnerResponse)]
    Owner {},
    #[returns(TokenBuyTaxResponse)]
    TokenTaxInfo { token_address: String },
    #[returns(SwapRouterResponse)]
    SwapRouter {},
}

#[cw_serde]
pub struct OwnerResponse {
    pub owner: Addr,
}

#[cw_serde]
pub struct TokenBuyTaxResponse {
    pub token_address: Addr,
    pub buy_tax: FractionFormat,
}

#[cw_serde]
pub struct SwapRouterResponse {
    pub router: Addr,
}
