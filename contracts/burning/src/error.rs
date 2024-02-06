use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    StdError(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Fee ratio must be less than 1")]
    FeeRatioMustBeLessThanOne {},
    #[error("Already exists")]
    AlreadyExists {},
    #[error("Not in whitelist")]
    NotInWhitelist {},
    #[error("Zero amount")]
    ZeroAmount {},
    #[error("Zero ratio")]
    ZeroRatio {},
    #[error("Invalid Reply Message")]
    InvalidReplyMsg {},
    #[error("Locked")]
    Locked {},
    #[error("Already unlocked")]
    AlreadyUnlocked {},
}
