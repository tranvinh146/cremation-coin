use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    ChangeOwner { new_owner: Addr },
    Withdraw { token_address: Addr },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(LockedTokenListResponse)]
    LockedTokenList {},
    #[returns(LockedTokenAmountResponse)]
    LockedTokenAmount { token_address: Addr },
    #[returns(OwnerResponse)]
    Owner {},
    #[returns(UnlockTimeResponse)]
    UnlockTime {},
}

#[cw_serde]
pub struct LockedTokenListResponse {
    pub locked_token_list: Vec<Addr>,
}

#[cw_serde]
pub struct LockedTokenAmountResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct OwnerResponse {
    pub owner: Addr,
}

#[cw_serde]
pub struct UnlockTimeResponse {
    pub unlock_time: Timestamp,
}
