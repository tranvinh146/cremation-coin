use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

#[cw_serde]
pub struct RewardInfo {
    pub token: String,
    pub reward_ratio: Decimal,
}

#[cw_serde]
pub enum Cw20HookMsg {
    Burn { amount: Uint128 },
}

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    AddToRewardWhitelist { reward_info: RewardInfo },
    RemoveFromRewardWhitelist { token: String },
    UpdateRewardInfo { reward_info: RewardInfo },
    Burn {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(OwnerResponse)]
    Owner {},
    #[returns(RewardWhiteListResponse)]
    RewardWhiteList {},
    #[returns(BurnedAmountResponse)]
    BurnedAmount {},
}

#[cw_serde]
pub struct OwnerResponse {
    pub owner: Addr,
}

#[cw_serde]
pub struct RewardWhiteListResponse {
    pub reward_whitelist: Vec<RewardInfo>,
}

#[cw_serde]
pub struct BurnedAmountResponse {
    pub burned_amount: Uint128,
}
