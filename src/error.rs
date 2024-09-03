use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid field: {field}")]
    InvalidField { field: String },

    #[error("Address is not whitelisted")]
    NotWhitelisted {},

    #[error("Invalid options provided")]
    InvalidOptions {},

    #[error("Invalid time range")]
    InvalidTimeRange {},

    #[error("Invalid market state for this operation")]
    InvalidMarketState {},

    #[error("Market has not ended yet")]
    MarketNotEnded {},

    #[error("Market is not active")]
    MarketNotActive {},

    #[error("Market is closed")]
    MarketClosed {},

    #[error("Bet amount is below the minimum allowed")]
    BetTooSmall {},

    #[error("Invalid odds provided")]
    InvalidOdds {},

    #[error("No funds sent with the transaction")]
    NoFundsSent {},

    #[error("Insufficient funds for the operation")]
    InsufficientFunds {},

    #[error("Order cannot be cancelled")]
    OrderNotCancellable {},

    #[error("Market is not resolved yet")]
    MarketNotResolved {},

    #[error("Winnings have already been redeemed")]
    AlreadyRedeemed {},

    #[error("Challenge period has ended")]
    ChallengePeriodEnded {},

    #[error("Dispute already exists for this market")]
    DisputeAlreadyExists {},

    #[error("Voting period has ended")]
    VotingPeriodEnded {},

    #[error("User has already voted")]
    AlreadyVoted {},

    #[error("Proposal already exists for this market")]
    ProposalAlreadyExists {},

    #[error("Incorrect bond amount sent")]
    IncorrectBondAmount {},

    #[error("Voting period has not ended yet")]
    VotingPeriodNotEnded {},

    #[error("Challenge Period has Not Ended yet")]
    ChallengePeriodNotEnded {},

    #[error("No Votes found on the dispute")]
    NoVotes {},

    #[error("your proposed answer was wrong, you are not the winner")]
    NotWinner {},

    #[error("Invalid Time Format")]
    InvalidTimeFormat {},

    #[error("Invalid proposal state for this operation")]
    InvalidProposalState {},
    
    #[error("Address is already whitelisted")]
    AlreadyWhitelisted {},
    
    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
}