use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cremation_token::msg::AssetInfo;
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct RewardInfo {
    pub token: String,
    pub reward_ratio: Decimal,
}

#[cw_serde]
pub enum Cw20HookMsg {
    SwapAndBurn { swap_paths: Vec<AssetInfo> },
}

#[cw_serde]
pub struct DevelopmentConfig {
    pub beneficiary: String,
    pub fee_ratio: Decimal,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub development_config: DevelopmentConfig,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateDevelopmentConfig {
        beneficiary: Option<String>,
        fee_ratio: Option<Decimal>,
    },
    AddToRewardWhitelist {
        reward_info: RewardInfo,
    },
    RemoveFromRewardWhitelist {
        token: String,
    },
    UpdateRewardInfo {
        reward_info: RewardInfo,
    },
    Burn {},
    SetSwapRouter {
        router: String,
    },
    SwapAndBurn {
        denom: String,
    },
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(OwnerResponse)]
    Owner {},
    #[returns(DevelopmentConfigResponse)]
    DevelopmentConfig {},
    #[returns(RewardWhitelistResponse)]
    RewardWhitelist {},
    #[returns(BurnedAmountResponse)]
    BurnedAmount {},
    #[returns(SwapRouterResponse)]
    SwapRouter {},
}

#[cw_serde]
pub struct OwnerResponse {
    pub owner: Addr,
}

#[cw_serde]
pub struct DevelopmentConfigResponse(pub DevelopmentConfig);

#[cw_serde]
pub struct RewardWhitelistResponse {
    pub reward_whitelist: Vec<RewardInfo>,
}

#[cw_serde]
pub struct BurnedAmountResponse {
    pub burned_amount: Uint128,
}

#[cw_serde]
pub struct SwapRouterResponse {
    pub swap_router: Option<Addr>,
}
