use crate::proposal::MultipleChoiceProposal;

use crate::voting::Votes;
use cosmwasm_std::{Addr, Uint128};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct ProposalResponse {
    /// The ID of the proposal being returned.
    pub id: u64,
    pub proposal: MultipleChoiceProposal,
}

/// Information about a vote that was cast.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct VoteInfo {
    /// The address that voted.
    pub voter: String,
    /// Position on the vote.
    pub power: Uint128,

    pub option: u32,
}

/// Information about a vote.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct VoteResponse {
    /// None if no such vote, Some otherwise.
    pub vote: Option<VoteInfo>,
}

/// Information about the votes for a proposal.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct VoteListResponse {
    pub votes: Vec<VoteInfo>,
}

/// A list of proposals returned by `ListProposals` and
/// `ReverseProposals`.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct ProposalListResponse {
    pub proposals: Vec<ProposalResponse>,
}
