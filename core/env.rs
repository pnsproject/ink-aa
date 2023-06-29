use ink::env::{DefaultEnvironment, Environment};

/// AccountAbstractionEnvironment
pub type AAEnvironment = DefaultEnvironment;

pub type AABalance = <AAEnvironment as Environment>::Balance;
pub type AAAccountId = <AAEnvironment as Environment>::AccountId;
pub type AAHash = <AAEnvironment as Environment>::Hash;
pub type AATimestamp = <AAEnvironment as Environment>::Timestamp;
pub type AABlockNumber = <AAEnvironment as Environment>::BlockNumber;
