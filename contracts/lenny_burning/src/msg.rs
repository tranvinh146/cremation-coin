use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
    pub reward_address: String,
    pub reward_info: RewardInfo,
    pub burn_limit: BurnLimit,
}

#[cw_serde]
pub enum Cw20HookMsg {
    Burn {},
}

#[cw_serde]
pub struct RewardInfo {
    pub refund_ratio: Decimal,
    pub reward_ratio: Decimal,
}

#[cw_serde]
pub struct BurnLimit {
    pub total: Uint128,
    pub per_address: Uint128,
    pub duration: u64,
}
#[cw_serde]
pub struct BurnedToday {
    pub amount: Uint128,
    pub latest_burned: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateOwner {
        owner: String,
    },
    UpdateRewardAddress {
        address: String,
    },
    UpdateRewardInfo {
        refund_ratio: Option<Decimal>,
        reward_ratio: Option<Decimal>,
    },
    UpdateBurnLimit {
        total: Option<Uint128>,
        per_address: Option<Uint128>,
        duration: Option<u64>,
    },
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(OwnerResponse)]
    Owner {},
    #[returns(RewardAddressResponse)]
    RewardAddress {},
    #[returns(RewardInfoResponse)]
    RewardInfo {},
    #[returns(BurnLimitResponse)]
    BurnLimit {},
    #[returns(TotalBurnedTodayResponse)]
    TotalBurnedToday {},
    #[returns(BurnedTodayByAddressResponse)]
    BurnedTodayByAddress { address: String },
    #[returns(BurnedAmountResponse)]
    BurnedAmount {},
}

#[cw_serde]
pub struct OwnerResponse {
    pub owner: Addr,
}

#[cw_serde]
pub struct RewardAddressResponse {
    pub address: Addr,
}

#[cw_serde]
pub struct RewardInfoResponse(pub RewardInfo);

#[cw_serde]
pub struct BurnLimitResponse(pub BurnLimit);

#[cw_serde]
pub struct TotalBurnedTodayResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct BurnedTodayByAddressResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct BurnedAmountResponse {
    pub burned_amount: Uint128,
}
