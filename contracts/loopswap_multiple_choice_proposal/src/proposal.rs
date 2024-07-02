use crate::query::ProposalResponse;
use crate::state::PROPOSAL_COUNT;
use crate::state::{
    PassedResponse, CLOSED_STATUS, EXECUTED_STATUS, FAILED_STATUS, OPEN_STATUS, PASSED_STATUS,
    PROPOSAL_VERSION, VOTING_CLOSED_STATUS,
};
use crate::status::Status;
use crate::threshold::{PercentageThreshold, Threshold};
use crate::voting::{does_vote_count_fail, does_vote_count_pass, Votes};
use cosmwasm_std::{
    to_binary, Addr, BlockInfo, CosmosMsg, Decimal, Empty, StdError, StdResult, Storage, Uint128,
    WasmMsg,
};
use cw20_base::msg::ExecuteMsg as Cw20ExecuteMsg;
use cw_utils::Duration;
use cw_utils::Expiration;
use loopswap::farming::ExecuteMsg as FarmingExecuteMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct MultipleChoiceProposal {
    pub title: String,
    pub description: String,
    /// The address that created this proposal.
    pub proposer: Addr,
    /// The minimum amount of time this proposal must remain open for
    /// voting. The proposal may not pass unless this is expired or
    /// None.
    // pub min_voting_period: Option<Expiration>,
    /// The the time at which this proposal will expire and close for
    /// additional votes.
    pub expiration: Expiration,
    pub voting_period: Duration,
    /// The total amount of voting power at the time of this
    /// proposal's creation.
    // pub total_power: Uint128,
    /// The messages that will be executed should this proposal pass.
    pub status: Status,
    pub allow_revoting: bool,
    /// The total amount of voting power at the time of this
    /// proposal's creation.
    pub total_power: Uint128,
    pub voting_start_time: u64,

    pub multiple_choice_options: Vec<MultipleChoiceOption>,
    pub amount: Uint128,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct MultipleChoiceVote {
    // A vote indicates which option the user has selected.
    pub option_id: u32,
    pub percentage: u32,
}

impl std::fmt::Display for MultipleChoiceVote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.option_id)
    }
}

pub fn advance_proposal_version(store: &mut dyn Storage, proposal_id: u64) -> StdResult<u128> {
    let id: u128 = PROPOSAL_VERSION
        .may_load(store, proposal_id.clone())?
        .unwrap_or(0u128)
        + 1;
    PROPOSAL_VERSION.save(store, proposal_id, &id)?;
    Ok(id)
}

pub fn advance_proposal_id(store: &mut dyn Storage) -> StdResult<u64> {
    let id: u64 = PROPOSAL_COUNT.may_load(store)?.unwrap_or_default() + 1;
    PROPOSAL_COUNT.save(store, &id)?;
    Ok(id)
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct MultipleChoiceOption {
    pub title: String,
    pub description: String,
    pub msgs: Option<Vec<CosmosMsg<Empty>>>,
    pub address: Option<String>,
    pub pool: Option<String>,
    pub reward_token: Option<String>,
    pub votes: Votes,
}

impl MultipleChoiceOption {
    pub fn add_vote(&mut self, power: Uint128, percentage: u32) {
        let assigned_power =
            power.multiply_ratio(Uint128::from(percentage), Uint128::from(100u128));
        self.votes.power += assigned_power;
    }

    pub fn remove_vote(&mut self, power: Uint128) {
        self.votes.power -= power;
    }
}

impl MultipleChoiceProposal {
    /// Consumes the proposal and returns a version which may be used
    /// in a query response. The difference being that proposal
    /// statuses are only updated on vote, execute, and close
    /// events. It is possible though that since a vote has occured
    /// the proposal expiring has changed its status. This method
    /// recomputes the status so that queries get accurate
    /// information.
    pub fn into_response(mut self, block: &BlockInfo, id: u64) -> StdResult<ProposalResponse> {
        self.update_status(block)?;
        Ok(ProposalResponse { id, proposal: self })
    }

    /// Gets the current status of the proposal.
    pub fn current_status(&self, block: &BlockInfo) -> StdResult<Status> {
        if self.status == Status::Open && (self.expiration.is_expired(block)) {
            Ok(Status::VotingClosed)
        } else {
            Ok(self.status.clone())
        }
    }

    /// Sets a proposals status to its current status.
    pub fn update_status(&mut self, block: &BlockInfo) -> StdResult<()> {
        let new_status = self.current_status(block)?;
        self.status = new_status;
        Ok(())
    }

    /// Returns true iff this proposal is sure to pass (even before
    /// expiration if no future sequence of possible votes can cause
    /// it to fail). Passing in the case of multiple choice proposals
    /// means that quorum has been met,
    /// one of the options that is not "None of the above"
    /// has won the most votes, and there is no tie.
    pub fn get_execution_message(&self) -> StdResult<Vec<CosmosMsg<Empty>>> {
        // Proposal can only pass if quorum has been met.
        let mut msgs: Vec<CosmosMsg<Empty>> = vec![];
        for choice in self.multiple_choice_options.clone() {
            let amount = self.calculate_vote_result(&choice);
            let message = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: choice.address.unwrap(),
                msg: to_binary(&FarmingExecuteMsg::UpdateReward {
                    pool: choice.pool.unwrap(),
                    rewards: vec![(choice.reward_token.unwrap(), choice.votes.power)],
                })?,
                funds: vec![],
            });
            if amount != Uint128::zero() {
                msgs.push(message);
            }
        }

        Ok(msgs)
    }

    /// Find the option with the highest vote weight, and note if there is a tie.
    pub fn calculate_vote_result(&self, choice: &MultipleChoiceOption) -> Uint128 {
        if self.total_power == Uint128::zero() {
            Uint128::zero();
        }
        let per = choice
            .votes
            .power
            .multiply_ratio(Uint128::from(100u128), self.total_power);

        self.amount.multiply_ratio(per, Uint128::from(100u128))
    }
}
