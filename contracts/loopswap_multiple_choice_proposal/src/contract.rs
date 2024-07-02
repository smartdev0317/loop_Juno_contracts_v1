use std::fmt::format;
use std::ops::Add;

#[cfg(not(feature = "library"))]
use crate::msg::{ExecuteMsg, InstantiateMsg, MultipleChoiceOptionMsg, QueryMsg};
use crate::proposal::{
    advance_proposal_id, advance_proposal_version, MultipleChoiceOption, MultipleChoiceProposal,
    MultipleChoiceVote,
};
use crate::query::{
    ProposalListResponse, ProposalResponse, VoteInfo, VoteListResponse, VoteResponse,
};
use crate::state::{
    Ballot, Config, BALLOTS, CLOSED_STATUS, CONFIG, EXECUTED_STATUS, FAILED_STATUS, OPEN_STATUS,
    PASSED_STATUS, POOL_AMOUNTS, PROPOSALS, PROPOSAL_COUNT, PROPOSAL_VERSION, PROPOSERS_INFO,
    VOTING_CLOSED_STATUS,
};

use crate::status::Status;
use crate::threshold::Threshold;
use crate::voting::{self, validate_voting_period, Vote, Votes};
use cosmwasm_std::{
    coins, entry_point, to_binary, Addr, BankMsg, Binary, CosmosMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, QuerierWrapper, QueryRequest, Response, StdError, StdResult, Storage, Uint128,
    WasmQuery,
};
use cw2::set_contract_version;
use cw20::BalanceResponse;
use cw_storage_plus::Bound;
use cw_utils::Duration;
use loopswap::factory::MigrateMsg;
use loopswap::staking::QueryMsg as stakingMsg;
use loopswap_staking::{msg::Cw20QueryMsg as stakingMsg_, state::Config as StakingConfig};
// use loopswap_staking::{msg::QueryMsg as QueryStakingMsg, state::Config as StakingConfig};

pub const DEFAULT_LIMIT: u64 = 30;
pub const MAX_PROPOSAL_SIZE: u64 = 30_000;

pub(crate) const CONTRACT_NAME: &str = "crates.io:cwd-proposal-single";
pub(crate) const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // msg.threshold.validate()?;

    let (min_voting_period, max_voting_period) =
        validate_voting_period(msg.min_voting_period, msg.max_voting_period)?;

    // let (initial_policy, pre_propose_messages) = msg
    //     .pre_propose_info
    //     .into_initial_policy_and_messages(dao.clone())?;

    let config = Config {
        // threshold: msg.threshold,
        max_voting_period,
        min_voting_period,
        dao: msg.dao,
        admin: info.sender.to_string(),
        proposal_creation_token_limit: msg.proposal_creation_token_limit,
        token_hold_duration: msg.token_hold_duration,
    };

    // Initialize proposal count to zero so that queries return zero
    // instead of None.
    PROPOSAL_COUNT.save(deps.storage, &0)?;
    CONFIG.save(deps.storage, &config)?;
    // CREATION_POLICY.save(deps.storage, &initial_policy)?;

    Ok(Response::default()
        // .add_submessages(pre_propose_messages)
        .add_attribute("action", "instantiate")
        .add_attribute("dao", config.dao))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Propose {
            title,
            description,
            options,
            voting_period,
            amount,
        } => execute_propose(
            deps,
            env,
            info.sender,
            title,
            description,
            voting_period,
            options,
            amount,
        ),
        ExecuteMsg::Vote { proposal_id, vote } => execute_vote(deps, env, info, proposal_id, vote),
        ExecuteMsg::Execute { proposal_id } => execute_execute(deps, env, info, proposal_id),
        ExecuteMsg::Close { proposal_id } => execute_close(deps, env, info, proposal_id),
        ExecuteMsg::UpdateConfig {
            max_voting_period,
            min_voting_period,
            dao,
            token_hold_duration,
            proposal_creation_token_limit,
        } => execute_update_config(
            deps,
            info,
            max_voting_period,
            min_voting_period,
            dao,
            token_hold_duration,
            proposal_creation_token_limit,
        ),
        ExecuteMsg::AddMultipleChoiceOptions {
            proposal_id,
            options,
        } => execute_add_multiple_choice_options(deps, info, proposal_id, options),
        ExecuteMsg::UpdateProposalTime {
            proposal_id,
            voting_period,
        } => execute_update_proposal_time(deps, info, env, proposal_id, voting_period),
    }
}

pub fn execute_propose(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    title: String,
    description: String,
    voting_period: Duration,
    options: Vec<MultipleChoiceOptionMsg>,
    amount: Uint128,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    if !verifying_voting_period(
        &voting_period,
        &config.min_voting_period,
        &config.max_voting_period,
    ) {
        return Err(StdError::generic_err("invalid voting period"));
    }

    let vote_power = get_voting_power(
        &deps.querier,
        deps.storage,
        sender.clone(),
        config.dao.to_string(),
        0u64,
    )?;
    if vote_power.is_zero() {
        return Err(StdError::generic_err("Power is zero can't create proposal"));
    }

    let expiration = voting_period.after(&env.block);
    let mut locked_amount = PROPOSERS_INFO
        .load(deps.storage, sender.to_string())
        .unwrap_or(Uint128::zero());

    if vote_power - locked_amount < config.proposal_creation_token_limit {
        return Err(StdError::generic_err(
            "Power is not enough to create proposal",
        ));
    }
    locked_amount += config.proposal_creation_token_limit;

    PROPOSERS_INFO.save(deps.storage, sender.to_string(), &locked_amount)?;
    let multiple_choice_options = get_multiple_choice_options(options);
    let proposal = {
        // Limit mutability to this block.
        let mut proposal = MultipleChoiceProposal {
            title,
            description,
            proposer: sender.clone(),
            expiration,
            total_power: Uint128::zero(),
            status: Status::Open,
            allow_revoting: false,
            voting_start_time: env.block.time.seconds(),
            multiple_choice_options,
            amount,
            voting_period,
        };
        // Update the proposal's status. Addresses case where proposal
        // expires on the same block as it is created.
        proposal.update_status(&env.block)?;
        proposal
    };
    let id = advance_proposal_id(deps.storage)?;
    advance_proposal_version(deps.storage, id.clone())?;
    // Limit the size of proposals.
    //
    // The Juno mainnet has a larger limit for data that can be
    // uploaded as part of an execute message than it does for data
    // that can be queried as part of a query. This means that without
    // this check it is possible to create a proposal that can not be
    // queried.
    //
    // The size selected was determined by uploading versions of this
    // contract to the Juno mainnet until queries worked within a
    // reasonable margin of error.
    //
    // `to_vec` is the method used by cosmwasm to convert a struct
    // into it's byte representation in storage.
    let proposal_size = cosmwasm_std::to_vec(&proposal)?.len() as u64;
    if proposal_size > MAX_PROPOSAL_SIZE {
        return Err(StdError::generic_err("Proposal size is too large"));
    }

    PROPOSALS.save(deps.storage, id, &proposal)?;

    Ok(Response::default()
        .add_attribute("action", "propose")
        .add_attribute("sender", sender)
        .add_attribute("proposal_id", id.to_string())
        .add_attribute("status", proposal.status.to_string()))
}
pub fn get_multiple_choice_options(
    choices: Vec<MultipleChoiceOptionMsg>,
) -> Vec<MultipleChoiceOption> {
    let mut multiple_choice_options: Vec<MultipleChoiceOption> = vec![];
    for choice in choices {
        let multiple_choice_option = MultipleChoiceOption {
            address: choice.address,
            description: choice.description,
            msgs: choice.msgs,
            pool: choice.pool,
            reward_token: choice.reward_token,
            title: choice.title,
            votes: Votes {
                power: Uint128::zero(),
            },
        };
        multiple_choice_options.push(multiple_choice_option);
    }

    multiple_choice_options
}

pub fn execute_update_proposal_time(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    proposal_id: u64,
    voting_period: Duration,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut prop: MultipleChoiceProposal = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(StdError::generic_err("No Such Proposal"))?;

    if info.sender != prop.proposer {
        return Err(StdError::generic_err("Unauthorized"));
    }

    if !verifying_voting_period(
        &voting_period,
        &config.min_voting_period,
        &config.max_voting_period,
    ) {
        return Err(StdError::generic_err("invalid voting period"));
    }
    prop.voting_period = voting_period;
    prop.expiration = voting_period.after(&env.block);

    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    Ok(Response::default().add_attribute("action", "update proposal time"))
}

pub fn execute_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    let mut prop: MultipleChoiceProposal = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(StdError::generic_err("No Such Proposal"))?;
    let vote_power = get_voting_power(
        &deps.querier,
        deps.storage,
        info.sender.clone(),
        config.dao.to_string(),
        proposal_id,
    )?;
    if vote_power.is_zero() {
        return Err(StdError::generic_err("Unauthorized"));
    }

    // Check here that the proposal is passed. Allow it to be executed
    // even if it is expired so long as it passed during its voting
    // period.

    // let mut locked_amount = PROPOSERS_INFO
    //     .load(deps.storage, prop.proposer.to_string())
    //     .unwrap_or(Uint128::zero());
    // locked_amount -= config.proposal_creation_token_limit;

    // PROPOSERS_INFO.save(deps.storage, prop.proposer.to_string(), &locked_amount)?;
    let msgs = prop.get_execution_message()?;
    let status = prop.current_status(&env.block)?;
    if status != Status::VotingClosed {
        return Err(StdError::generic_err("Voting not closed yet"));
    }
    prop.status = Status::Open;
    prop.total_power = Uint128::zero();
    prop.voting_start_time = env.block.time.seconds();
    prop.expiration = prop.voting_period.after(&env.block);
    let mut choices: Vec<MultipleChoiceOption> = vec![];
    for mut choice in prop.multiple_choice_options.iter_mut() {
        choice.votes.power = Uint128::zero();
        choices.push(choice.clone());
    }
    prop.multiple_choice_options = choices;
    println!("msgs {:?}", msgs);
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;
    advance_proposal_version(deps.storage, proposal_id.clone())?;
    let response = Response::new()
        .add_attribute("action", "execute")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("dao", config.dao)
        .add_messages(msgs);

    Ok(response)
}

pub fn execute_add_multiple_choice_options(
    deps: DepsMut,
    info: MessageInfo,
    proposal_id: u64,
    options: Vec<MultipleChoiceOptionMsg>,
) -> StdResult<Response> {
    let mut prop: MultipleChoiceProposal = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(StdError::generic_err("No Such Proposal"))?;
    for option in options {
        let choice = MultipleChoiceOption {
            address: option.address,
            description: option.description,
            msgs: option.msgs,
            pool: option.pool,
            reward_token: option.reward_token,
            title: option.title,
            votes: Votes {
                power: Uint128::zero(),
            },
        };
        prop.multiple_choice_options.push(choice);
    }
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    Ok(Response::default())
}
pub fn execute_vote(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
    votes: Vec<MultipleChoiceVote>,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    let mut prop: MultipleChoiceProposal = PROPOSALS
        .may_load(deps.storage, proposal_id)?
        .ok_or(StdError::generic_err("No such proposal"))?;

    let vote_power = get_voting_power(
        &deps.querier,
        deps.storage,
        info.sender.clone(),
        config.dao.to_string(),
        proposal_id,
    )?;
    validate_options(&votes, &prop)?;
    if vote_power.is_zero() {
        let mut message = "Not Registered".to_string();
        message.push_str(&format!(
            "sender {} dao {}",
            info.sender.clone(),
            config.dao.to_string(),
        ));
        return Err(StdError::generic_err(&message));
    }

    if prop.expiration.is_expired(&env.block) {
        return Err(StdError::generic_err("Voting time expired"));
    } else if prop.status == Status::Executed || prop.status == Status::Closed {
        return Err(StdError::generic_err(format!(
            "Proposal is in {} state",
            prop.status
        )));
    }
    let version = PROPOSAL_VERSION.load(deps.storage, proposal_id)?;
    let mut proposal_version = String::from(&proposal_id.to_string());
    proposal_version.push_str(".");
    proposal_version.push_str(&version.to_string());
    validate_votes_percentage(votes.clone())?;
    for options in votes {
        BALLOTS.update(
            deps.storage,
            (
                proposal_version.clone(),
                info.sender.clone().to_string(),
                options.option_id,
            ),
            |bal| match bal {
                Some(current_ballot) => {
                    if prop.allow_revoting {
                        if current_ballot.vote == options {
                            // Don't allow casting the same vote more than
                            // once. This seems liable to be confusing
                            // behavior.
                            Err(StdError::generic_err("Already Casted"))
                        } else {
                            // Remove the old vote if this is a re-vote.
                            prop.multiple_choice_options
                                .get_mut(options.option_id as usize - 1usize)
                                .unwrap()
                                .remove_vote(current_ballot.power);
                            Ok(Ballot {
                                power: vote_power,
                                vote: options.clone(),
                            })
                        }
                    } else {
                        prop.multiple_choice_options
                            .get_mut(options.option_id as usize - 1usize)
                            .unwrap()
                            .remove_vote(current_ballot.power);
                        Ok(Ballot {
                            power: vote_power,
                            vote: options.clone(),
                        })
                    }
                }
                None => Ok(Ballot {
                    power: vote_power,
                    vote: options.clone(),
                }),
            },
        )?;
        prop.multiple_choice_options
            .get_mut(options.option_id as usize - 1usize)
            .unwrap()
            .add_vote(vote_power.clone(), options.percentage.clone());
    }

    prop.total_power += vote_power;
    prop.update_status(&env.block)?;

    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    // let new_status = prop.clone().status;

    Ok(Response::default()
        .add_attribute("action", "vote")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string())
        .add_attribute("status", prop.status.to_string())
        .add_attribute("power added", vote_power))
}

pub fn validate_votes_percentage(
    votes: Vec<MultipleChoiceVote>,
) -> StdResult<bool> {
    let mut vote_percentage = 0u32;
    for vote in votes {
        vote_percentage += vote.percentage;
    }
    if vote_percentage > 100u32 {
        return Err(StdError::generic_err("total percenatge voting is invalid"));
    }
    Ok(true)
}

pub fn validate_options(
    vote: &Vec<MultipleChoiceVote>,
    prop: &MultipleChoiceProposal,
) -> StdResult<()> {
    if vote.len() > prop.multiple_choice_options.len() {
        return Err(StdError::generic_err("not valid options provided"));
    }
    Ok(())
}
pub fn execute_close(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    proposal_id: u64,
) -> StdResult<Response> {
    let mut prop: MultipleChoiceProposal = PROPOSALS.load(deps.storage, proposal_id)?;
    let config: Config = CONFIG.load(deps.storage)?;

    // Update status to ensure that proposals which were open and have
    // expired are moved to "rejected."

    let mut locked_amount = PROPOSERS_INFO
        .load(deps.storage, prop.proposer.to_string())
        .unwrap_or(Uint128::zero());
    if prop.voting_start_time + config.token_hold_duration < env.block.time.seconds() {
        return Err(StdError::generic_err(
            "tokens are hold in the contract kindly wait for the specified time",
        ));
    }

    prop.update_status(&env.block)?;
    locked_amount -= config.proposal_creation_token_limit;

    PROPOSERS_INFO.save(deps.storage, prop.proposer.to_string(), &locked_amount)?;

    prop.status = Status::Closed;
    PROPOSALS.save(deps.storage, proposal_id, &prop)?;

    // Add prepropose / deposit module hook which will handle deposit refunds.

    Ok(Response::default()
        .add_attribute("action", "close")
        .add_attribute("sender", info.sender)
        .add_attribute("proposal_id", proposal_id.to_string()))
}

pub fn get_voting_power(
    querier: &QuerierWrapper,
    store: &mut dyn Storage,
    sender: Addr,
    dao: String,
    proposal_id: u64,
) -> StdResult<Uint128> {
    let mut total_power = Uint128::zero();

    let config: StakingConfig = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: dao.to_string(),
        msg: to_binary(&stakingMsg::QueryConfig {})?,
    }))?;

    let durations = config.duration_values_vector;
    for duration in durations {
        let power: Uint128 = querier
            .query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: dao.to_string(),
                msg: to_binary(&stakingMsg_::BalanceByDuration {
                    address: sender.to_string(),
                    duration,
                })?,
            }))
            .unwrap_or(Uint128::from(0u128));
        total_power += power;
        POOL_AMOUNTS.save(store, (proposal_id, duration), &power)?;
    }
    Ok(total_power)
}

#[allow(clippy::too_many_arguments)]
pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    // threshold_: Option<Threshold>,
    max_voting_period_: Option<Duration>,
    min_voting_period_: Option<Duration>,
    dao_: Option<String>,
    token_hold_duration_: Option<u64>,
    proposal_creation_token_limit_: Option<Uint128>,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    // Only the DAO may call this method.
    if info.sender != config.admin {
        return Err(StdError::generic_err("Unauthorized"));
    }
    let mut min_voting_period = config.min_voting_period;
    let mut max_voting_period = config.max_voting_period;
    if min_voting_period_.is_some() {
        min_voting_period = min_voting_period_.unwrap();
    }
    if max_voting_period_.is_some() {
        max_voting_period = max_voting_period_.unwrap();
    }
    let (min_voting_period, max_voting_period) =
        validate_voting_period(min_voting_period, max_voting_period)?;
    let mut dao = config.dao;
    if dao_.is_some() {
        dao = dao_.unwrap();
    }
    let mut token_hold_duration = config.token_hold_duration;
    if token_hold_duration_.is_some() {
        token_hold_duration = token_hold_duration_.unwrap();
    }

    let mut proposal_creation_token_limit = config.proposal_creation_token_limit;
    if proposal_creation_token_limit_.is_some() {
        proposal_creation_token_limit = proposal_creation_token_limit_.unwrap();
    }

    CONFIG.save(
        deps.storage,
        &Config {
            // threshold,
            max_voting_period,
            min_voting_period,
            dao,
            admin: config.admin,
            token_hold_duration,
            proposal_creation_token_limit,
        },
    )?;

    Ok(Response::default()
        .add_attribute("action", "update_config")
        .add_attribute("sender", info.sender))
}

pub fn verifying_voting_period(
    voting_period: &Duration,
    min_vp: &Duration,
    max_vp: &Duration,
) -> bool {
    match (voting_period, min_vp, max_vp) {
        // compare if both height or both time
        (Duration::Time(vp), Duration::Time(mvp1), Duration::Time(mvp2)) => {
            vp >= mvp1 && vp <= mvp2
        }

        // if they are mis-matched finite ends, no compare possible
        _ => false,
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => query_config(deps),
        QueryMsg::Dao {} => query_dao(deps),
        QueryMsg::Proposal { proposal_id } => query_proposal(deps, env, proposal_id),
        QueryMsg::ListProposals { start_after, limit } => {
            query_list_proposals(deps, env, start_after, limit)
        }
        QueryMsg::ProposalCount {} => query_proposal_count(deps),
        QueryMsg::GetVote { proposal_id, voter } => query_user_list_vote(deps, proposal_id, voter),
        QueryMsg::ListVotes {
            proposal_id,
            start_after,
            limit,
        } => query_list_votes(deps, proposal_id, start_after, limit),
        QueryMsg::Info {} => query_info(deps),
        QueryMsg::HoldAmount { address } => query_hold_amount(deps, address),
        // QueryMsg::ReverseProposals {
        //     start_before,
        //     limit,
        // } => query_reverse_proposals(deps, env, start_before, limit),
        // QueryMsg::ProposalCreationPolicy {} => query_creation_policy(deps),
        // QueryMsg::ProposalHooks {} => to_binary(&PROPOSAL_HOOKS.query_hooks(deps)?),
        // QueryMsg::VoteHooks {} => to_binary(&VOTE_HOOKS.query_hooks(deps)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_binary(&config)
}

pub fn query_dao(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_binary(&config.dao)
}

pub fn query_proposal(deps: Deps, env: Env, id: u64) -> StdResult<Binary> {
    let proposal = PROPOSALS.load(deps.storage, id)?;
    to_binary(&proposal.into_response(&env.block, id)?)
}

pub fn query_hold_amount(deps: Deps, address: String) -> StdResult<Binary> {
    to_binary(
        &PROPOSERS_INFO
            .load(deps.storage, address)
            .unwrap_or(Uint128::zero()),
    )
}

// pub fn query_creation_policy(deps: Deps) -> StdResult<Binary> {
//     let policy = CREATION_POLICY.load(deps.storage)?;
//     to_binary(&policy)
// }

pub fn query_list_proposals(
    deps: Deps,
    env: Env,
    start_after: Option<u64>,
    limit: Option<u64>,
) -> StdResult<Binary> {
    let min = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    let props: Vec<ProposalResponse> = PROPOSALS
        .range(deps.storage, min, None, cosmwasm_std::Order::Ascending)
        .take(limit as usize)
        .collect::<Result<Vec<(u64, MultipleChoiceProposal)>, _>>()?
        .into_iter()
        .map(|(id, proposal)| proposal.into_response(&env.block, id))
        .collect::<StdResult<Vec<ProposalResponse>>>()?;

    to_binary(&ProposalListResponse { proposals: props })
}

pub fn query_reverse_proposals(
    deps: Deps,
    env: Env,
    start_before: Option<u64>,
    limit: Option<u64>,
) -> StdResult<Binary> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    let max = start_before.map(Bound::exclusive);
    let props: Vec<ProposalResponse> = PROPOSALS
        .range(deps.storage, None, max, cosmwasm_std::Order::Descending)
        .take(limit as usize)
        .collect::<Result<Vec<(u64, MultipleChoiceProposal)>, _>>()?
        .into_iter()
        .map(|(id, proposal)| proposal.into_response(&env.block, id))
        .collect::<StdResult<Vec<ProposalResponse>>>()?;

    to_binary(&ProposalListResponse { proposals: props })
}

pub fn query_proposal_count(deps: Deps) -> StdResult<Binary> {
    let proposal_count = PROPOSAL_COUNT.load(deps.storage)?;
    to_binary(&proposal_count)
}

pub fn query_user_list_vote(deps: Deps, proposal_id: u64, voter: String) -> StdResult<Binary> {
    // let voter = deps.api.addr_validate(&voter)?;
    // let limit = limit.unwrap_or(DEFAULT_LIMIT);
    // let start_after = start_after
    //     .map(|addr| deps.api.addr_validate(&addr))
    //     .transpose()?;
    // let min = start_after.map(Bound::<Addr>::exclusive);
    let version = PROPOSAL_VERSION.load(deps.storage, proposal_id)?;
    let mut proposal_version = String::from(&proposal_id.to_string());
    proposal_version.push_str(".");
    proposal_version.push_str(&version.to_string());
    let votes = BALLOTS
        .prefix((proposal_version, voter.clone()))
        .range(deps.storage, None, None, cosmwasm_std::Order::Ascending)
        .take(30 as usize)
        .map(|item| {
            let (option_id, ballot) = item?;
            Ok(VoteInfo {
                voter: voter.clone(),
                power: ballot.power,
                option: option_id,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    to_binary(&VoteListResponse { votes })
}

pub fn query_list_votes(
    deps: Deps,
    proposal_id: u64,
    start_after: Option<(String, u32)>,
    limit: Option<u64>,
) -> StdResult<Binary> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT);

    let min = start_after.map(Bound::<(String, u32)>::exclusive);
    let version = PROPOSAL_VERSION.load(deps.storage, proposal_id)?;
    let mut proposal_version = String::from(&proposal_id.to_string());
    proposal_version.push_str(".");
    proposal_version.push_str(&version.to_string());
    let votes = BALLOTS
        .sub_prefix(proposal_version)
        .range(deps.storage, min, None, cosmwasm_std::Order::Ascending)
        .take(limit as usize)
        .map(|item| {
            let ((voter, option_id), ballot) = item?;
            Ok(VoteInfo {
                voter,
                power: ballot.power,
                option: option_id,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;

    to_binary(&VoteListResponse { votes })
}

// pub fn query_list_votes(
//     deps: Deps,
//     proposal_id: u64,
//     start_after: Option<String>,
//     limit: Option<u64>,
// ) -> StdResult<Binary> {
//     let limit = limit.unwrap_or(DEFAULT_LIMIT);
//     let start_after = start_after
//         .map(|addr| deps.api.addr_validate(&addr))
//         .transpose()?;
//     let min = start_after.map(Bound::<Addr>::exclusive);

//     let votes = BALLOTS
//         .prefix(proposal_id)
//         .range(deps.storage, min, None, cosmwasm_std::Order::Ascending)
//         .take(limit as usize)
//         .map(|item| {
//             let (voter, ballot) = item?;
//             Ok(VoteInfo {
//                 voter,
//                 vote: ballot.vote,
//                 power: ballot.power,
//             })
//         })
//         .collect::<StdResult<Vec<_>>>()?;

//     to_binary(&VoteListResponse { votes })
// }

pub fn query_info(deps: Deps) -> StdResult<Binary> {
    let info = cw2::get_contract_version(deps.storage)?;
    to_binary(&info)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
