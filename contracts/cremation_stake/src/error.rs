use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),
    #[error("Already Staked")]
    AlreadyStaked {},
    #[error("Not Staked")]
    NotStaked {},
    #[error("Insufficient Rewards")]
    InsufficientRewards {},
    #[error("Unsupported Token")]
    UnsupportedToken {},
    #[error("Staking Unavailable")]
    StakingUnavailable {},
    #[error("Invalid Stake Amount")]
    InvalidStakeAmount {},
    #[error("Insufficient Balance")]
    InsufficientBalance {},
}
