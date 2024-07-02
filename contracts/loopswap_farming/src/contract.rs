//use crate::response::MsgInstantiateContractResponse;
use crate::state::{
    Config, RewardInfo, CONFIG, CURRENT_POOL_ADDRESS, 
    LIQUIDITY_TOKEN_MAP, POOL_COMPOUNDED_INDEX_MAP, POOL_LAST_DISTRIBUTION_TIME_IN_SECONDS,
    POOL_REWARD_INDEX_MAP, POOL_TOTAL_COMPOUNDED_AMOUNT, STAKEABLE_INFOS,
    TOTAL_ACCUMULATED_DISTRIBUTED_AMOUNT_IN_POOL_MAP, TOTAL_REWARDS_IN_POOL, TOTAL_STAKED,
    UNCLAIMED_DISTRIBUTED_TOKEN_AMOUNT_MAP, USER_AUTO_COMPOUND_SUBSCRIPTION_MAP,
    USER_COMPOUNDED_REWARD_INFO_MAP, USER_REWARD_INFO_MAP, USER_REWARD_STARTING_TIME_MAP,
    USER_STAKED_AMOUNT,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Reply,
    ReplyOn, Response, StdError, StdResult, Storage, SubMsg, Uint128, WasmMsg,
};

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};

use cw_storage_plus::Bound;
use loopswap::asset::{StakeablePairedDistributionTokenInfo, StakeableToken};
use loopswap::factory::MigrateMsg;
use loopswap::farming::{
    Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg, QueryRewardResponse,
    QueryUserRewardInPoolResponse,
};
use loopswap::querier::query_token_balance;
use loopswap::token::InstantiateMsg as TokenInstantiateMsg;
// use protobuf::Message;
use crate::parse_reply::parse_reply_instantiate_data;
const REWARD_CALCULATION_DECIMAL_PRECISION: u128 = 1000000000000u128;
const INSTANTIATE_REPLY_ID: u64 = 1;

//Initialize the contract.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: _info.sender.to_string(),
        freeze: false,                                // freeze flag will be used to
        lock_time_frame: 0,                           // lock the user reward for a certain period,
        wait_time_for_distribution_in_seconds: 86400, // waiting time for distrbution in seconds
        second_owner: String::from(""),               // second admin
        default_limit: 10,                            // used for pagination purpose
        max_limit: 30,                                // used for pagination purpose
        lock_time_frame_for_compound_reward: 0, // lock the user compounded reward for a certain period
        reserve_addr: _msg.reserve_addr, // reserve address is a dedicated account if user left it's reward amount we will store the amount in this account
        token_code_id: _msg.token_code_id,
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new())
}

//Execute the handle messages.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig { owner } => execute_update_config(deps, env, info, owner),
        ExecuteMsg::AddSecondOwner {
            second_owner_address,
        } => execute_add_second_owner(deps, info, second_owner_address),
        ExecuteMsg::UpdateFreezeFlag { freeze_flag } => {
            execute_update_freeze_flag(deps, env, info, freeze_flag)
        }
        ExecuteMsg::UpdateLockTimeFrame { lock_time_frame } => {
            execute_update_lock_time_frame(deps, env, info, lock_time_frame)
        }
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateReward { pool, rewards } => {
            execute_update_reward(deps, env, info, pool, rewards)
        }
        ExecuteMsg::DistributeByLimit { start_after, limit } => {
            execute_distribute_by_limit(deps, env, info, start_after, limit)
        }
        ExecuteMsg::AddStakeableToken { token } => {
            execute_add_stakeable_token(deps, env, info, token)
        }
        ExecuteMsg::UpdateWaitTimeForDistribution {
            wait_time_for_distribution_in_seconds,
        } => execute_update_wait_time_for_distributions(
            deps,
            env,
            info,
            wait_time_for_distribution_in_seconds,
        ),
        ExecuteMsg::UpdateStakeableTokenAddress {
            old_address,
            new_address,
        } => execute_update_stakeable_token_address(deps, env, info, old_address, new_address),
        ExecuteMsg::DeleteStakeableToken { address } => {
            execute_delete_stakeable_token(deps, env, info, address)
        }
        ExecuteMsg::DeleteDistributeableToken {
            pool_address,
            token_address,
        } => execute_delete_distributeable_token(deps, env, info, pool_address, token_address),
        ExecuteMsg::ClaimReward { pool_address } => {
            execute_claim_reward(deps, env, info, pool_address)
        }

        ExecuteMsg::UpdateLockTimeFrameForCompundReward {
            lock_time_frame_for_compound_reward,
        } => execute_update_lock_time_frame_for_compound_reward(
            deps,
            env,
            info,
            lock_time_frame_for_compound_reward,
        ),
        ExecuteMsg::OptForAutoCompound { pool_address } => {
            execute_opt_for_auto_compound(deps, env, info, pool_address)
        }
        ExecuteMsg::UpdateReserveAddress { reserve_addr } => {
            execute_update_reserve_addr(deps, info, reserve_addr)
        }
    }
}

/// To update the reserve address in the config, only owner or second_owner can perform the action

pub fn execute_update_reserve_addr(
    deps: DepsMut,
    info: MessageInfo,
    reserve_addr: String,
) -> StdResult<Response> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner
        && (config.second_owner.is_empty()
            || (!config.second_owner.is_empty() && info.sender != config.second_owner))
    {
        return Err(StdError::generic_err("unauthorized"));
    }

    config.reserve_addr = reserve_addr;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update reserve addr"))
}

/// User calls this to opt for auto compounding

pub fn execute_opt_for_auto_compound(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_address: String,
) -> StdResult<Response> {
    if !STAKEABLE_INFOS.has(deps.storage, pool_address.to_string()) {
        return Err(StdError::generic_err("No pool found"));
    }

    let user_address = info.sender.to_string();
    let mut user_pool_key = user_address.to_string();
    user_pool_key.push_str(&pool_address);
    if USER_AUTO_COMPOUND_SUBSCRIPTION_MAP
        .may_load(deps.storage, user_pool_key.to_string())?
        .unwrap_or(false)
    {
        return Ok(Response::new()
            .add_attribute("action", "opting for auto compound")
            .add_attribute("response", "Already Opt for auto compounding"));
    }

    let user_staked =
        get_user_staked_amount_in_pool_from_map_storage(deps.storage, user_pool_key.to_string());

    //Check to allow auto compounding only when users have staked assets
    if user_staked <= Uint128::zero() {
        return Err(StdError::generic_err(
            "No asset staked in the pool, you need to stake asset in the pool first",
        ));
    }
    //Adding amount in the total pool compounded amount
    let mut total_compounded = POOL_TOTAL_COMPOUNDED_AMOUNT
        .may_load(deps.storage, pool_address.to_string())?
        .unwrap_or_else(Uint128::zero);
    total_compounded += user_staked;

    POOL_TOTAL_COMPOUNDED_AMOUNT.save(deps.storage, pool_address.to_string(), &total_compounded)?;

    let stakeable_info = STAKEABLE_INFOS.load(deps.storage, pool_address.to_string())?;

    //Calculating compounded reward index of the pool and user compounded reward index for all distribution tokens of the pool
    for dist_tkn in stakeable_info.distribution.iter() {
        let mut pool_dist_key = pool_address.to_string();
        pool_dist_key.push_str(&dist_tkn.token.to_string());
        let mut user_pool_dist_key = user_address.to_string();
        user_pool_dist_key.push_str(&pool_dist_key);
        let current_compounded_reward_index = POOL_COMPOUNDED_INDEX_MAP
            .may_load(deps.storage, pool_dist_key.to_string())?
            .unwrap_or_else(Uint128::zero);
        let mut user_compounded_reward_index = USER_COMPOUNDED_REWARD_INFO_MAP
            .may_load(deps.storage, user_pool_dist_key.to_string())?
            .unwrap_or_else(|| RewardInfo {
                reward_index: Uint128::zero(),
                pending_reward: Uint128::zero(),
            });

        user_compounded_reward_index.reward_index = current_compounded_reward_index;

        USER_COMPOUNDED_REWARD_INFO_MAP.save(
            deps.storage,
            user_pool_dist_key.to_string(),
            &user_compounded_reward_index,
        )?;
    }
    //Subscribing user for auto compounding
    USER_AUTO_COMPOUND_SUBSCRIPTION_MAP.save(deps.storage, user_pool_key.to_string(), &true)?;

    //Resetting the reward time
    USER_REWARD_STARTING_TIME_MAP.save(
        deps.storage,
        user_pool_key.to_string(),
        &env.block.time.seconds(),
    )?;
    Ok(Response::new()
        .add_attribute("action", "opt for auto compound")
        .add_attribute("response", "Successful"))
}

/// Update lock time for locking reward of auto compounding
pub fn execute_update_lock_time_frame_for_compound_reward(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    lock_time_frame_for_compound_reward: u64,
) -> StdResult<Response> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner
        && (config.second_owner.is_empty()
            || (!config.second_owner.is_empty() && info.sender != config.second_owner))
    {
        return Err(StdError::generic_err("unauthorized"));
    }
    config.lock_time_frame_for_compound_reward = lock_time_frame_for_compound_reward;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_lock_time_frame_for_auto_compund"))
}

/// Receive cw20 token for staking in the pool
pub fn receive_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let pool_contract_addr = info.sender;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Stake {}) => {
            // compare with sent item.
            if STAKEABLE_INFOS
                .may_load(deps.storage, pool_contract_addr.to_string())?
                .is_some()
            {
                execute_stake(
                    deps,
                    env,
                    Addr::unchecked(cw20_msg.sender),
                    pool_contract_addr.to_string(),
                    cw20_msg.amount,
                )
            } else {
                Err(StdError::generic_err("Incorrect Asset Provided"))
            }
        }

        Ok(Cw20HookMsg::UnstakeAndClaim {}) => {
            if LIQUIDITY_TOKEN_MAP
                .may_load(deps.storage, pool_contract_addr.to_string())?
                .is_some()
            {
                let stakeable_token_addr =
                    LIQUIDITY_TOKEN_MAP.load(deps.storage, pool_contract_addr.to_string())?;
                execute_unstake_and_claim(
                    deps,
                    env,
                    Addr::unchecked(cw20_msg.sender).to_string(),
                    stakeable_token_addr,
                    cw20_msg.amount,
                    pool_contract_addr.to_string(),
                    true,
                )
            } else {
                Err(StdError::generic_err("Incorrect Asset Provided"))
            }
        }
        Ok(Cw20HookMsg::UnstakeWithoutClaim {}) => {
            if LIQUIDITY_TOKEN_MAP
                .may_load(deps.storage, pool_contract_addr.to_string())?
                .is_some()
            {
                let stakeable_token_addr =
                    LIQUIDITY_TOKEN_MAP.load(deps.storage, pool_contract_addr.to_string())?;
                execute_unstake_and_claim(
                    deps,
                    env,
                    Addr::unchecked(cw20_msg.sender).to_string(),
                    stakeable_token_addr,
                    cw20_msg.amount,
                    pool_contract_addr.to_string(),
                    false,
                )
            } else {
                Err(StdError::generic_err("Incorrect Asset Provided"))
            }
        }
        Err(_err) => Err(StdError::generic_err("Unsuccessful")),
    }
}

// Allow admin to add tokens so that users can stake that token.
pub fn execute_add_stakeable_token(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_address: String,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;
    // Permission check
    if info.sender != config.owner
        && (config.second_owner.is_empty()
            || (!config.second_owner.is_empty() && info.sender != config.second_owner))
    {
        return Err(StdError::generic_err("unauthorized"));
    }

    if STAKEABLE_INFOS
        .may_load(deps.storage, pool_address.to_string())?
        .is_some()
    {
        return Err(StdError::generic_err("Token already exists in list"));
    }
    let stakeable_token = StakeableToken {
        liquidity_token: "".to_string(),
        token: pool_address.to_string(),
        distribution: vec![],
    };

    STAKEABLE_INFOS.save(deps.storage, pool_address.to_string(), &stakeable_token)?;
    CURRENT_POOL_ADDRESS.save(deps.storage, &pool_address)?;
    // Setting current time as a default time for pool when the pool is created
    POOL_LAST_DISTRIBUTION_TIME_IN_SECONDS.save(
        deps.storage,
        pool_address,
        &env.block.time.seconds(),
    )?;
    let response = Response::new()
        .add_submessage(SubMsg {
            // Create FLP token
            msg: WasmMsg::Instantiate {
                admin: Some(info.sender.to_string()),
                code_id: config.token_code_id,
                msg: to_binary(&TokenInstantiateMsg {
                    name: "LoopFarm".to_string(),
                    symbol: "uLF".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: env.contract.address.to_string(),
                        cap: None,
                    }),
                })?,
                funds: vec![],
                label: "FLP Token".to_string(),
            }
            .into(),
            gas_limit: None,
            id: INSTANTIATE_REPLY_ID,
            reply_on: ReplyOn::Success,
        })
        .add_attribute("action", "Pool added");

    Ok(response)
    //Ok(Response::new().add_attribute("action", "Pool added"))
}

/// Only owner can execute it. To update the owner address
pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner_provided: Option<String>,
) -> StdResult<Response> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // Permission check
    if info.sender != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }
    if let Some(owner) = owner_provided {
        // Validate address format
        let _ = deps.api.addr_validate(&owner)?;

        config.owner = owner;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

/// Only owner can execute it. To set second owner
pub fn execute_add_second_owner(
    deps: DepsMut,
    info: MessageInfo,
    second_owner_address: String,
) -> StdResult<Response> {
    let mut config: Config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }
    deps.api.addr_validate(&second_owner_address)?;
    config.second_owner = second_owner_address;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "second admin added"))
}

///  It is to stop users to Unstake.
pub fn execute_update_freeze_flag(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    freeze_flag: String,
) -> StdResult<Response> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner
        && (config.second_owner.is_empty()
            || (!config.second_owner.is_empty() && info.sender != config.second_owner))
    {
        return Err(StdError::generic_err("unauthorized"));
    }

    if freeze_flag.eq_ignore_ascii_case("Y") {
        config.freeze = true;
    } else {
        config.freeze = false;
    }
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_freeze_flag"))
}

// Only owner can execute it. It is to set the lock on the reward for certain time period.
pub fn execute_update_lock_time_frame(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    lock_time_frame: u64,
) -> StdResult<Response> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner
        && (config.second_owner.is_empty()
            || (!config.second_owner.is_empty() && info.sender != config.second_owner))
    {
        return Err(StdError::generic_err("unauthorized"));
    }
    config.lock_time_frame = lock_time_frame;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_lock_time_frame"))
}

/// waiting time for distribution of the pool
pub fn execute_update_wait_time_for_distributions(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    wait_time_for_distribution_in_seconds: u64,
) -> StdResult<Response> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.owner
        && (config.second_owner.is_empty()
            || (!config.second_owner.is_empty() && info.sender != config.second_owner))
    {
        return Err(StdError::generic_err("unauthoriized"));
    }
    if wait_time_for_distribution_in_seconds < 1u64 {
        return Err(StdError::generic_err(
            "wait_time_for_distribution_in_seconds cannot be set to 0",
        ));
    }
    config.wait_time_for_distribution_in_seconds = wait_time_for_distribution_in_seconds;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new().add_attribute("action", "update_lock_time_frame"))
}

/// Allow users to stake the tokens.
pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    info: Addr,
    pool_address: String,
    amount: Uint128,
) -> StdResult<Response> {
    let mut messages: Vec<CosmosMsg> = vec![];
    let stakeable_token = if let Some(stakeable_token) =
        STAKEABLE_INFOS.may_load(deps.storage, pool_address.to_string())?
    {
        stakeable_token
    } else {
        return Err(StdError::generic_err("Provided asset info not correct"));
    };

    let asset_infos = stakeable_token.token.clone();
    //  update staked amount in TOTAL_STAKED map
    let total_staked =
        get_total_staked_amount_in_pool_from_map_storage(deps.storage, pool_address.to_string());

    TOTAL_STAKED.save(deps.storage, pool_address, &(total_staked + amount))?;
    let mut user_pool_key = info.to_string();
    user_pool_key.push_str(&asset_infos);

    let mut user_staked =
        get_user_staked_amount_in_pool_from_map_storage(deps.storage, user_pool_key.to_string());

    let share = amount;

    let user_opt_for_auto_compound = USER_AUTO_COMPOUND_SUBSCRIPTION_MAP
        .may_load(deps.storage, user_pool_key.to_string())?
        .unwrap_or(false);

    if total_staked == Uint128::zero() {
        // Initial share = collateral amount
        for dist_itr in stakeable_token.distribution.iter() {
            let mut pool_dist_key: String = asset_infos.to_string();
            pool_dist_key.push_str(&dist_itr.token.to_string());
            let mut user_pool_dist_key = info.to_string();

            //geting pool reward index
            let current_reward_index =
                get_reward_index_map_from_map_storage(deps.storage, pool_dist_key.to_string());

            //assigning reward index to the user
            let mut reward_index_to_be_assigned = Uint128::zero();
            if current_reward_index > Uint128::zero() {
                reward_index_to_be_assigned = current_reward_index; //if it is not the 1st time to stake in the pool
            }
            user_pool_dist_key.push_str(&pool_dist_key.to_string());

            //getting user reward index
            let user_reward_info = get_user_reward_info_from_map_storage(
                deps.storage,
                user_pool_dist_key.to_string(),
                reward_index_to_be_assigned,
            );

            USER_REWARD_INFO_MAP.save(
                deps.storage,
                user_pool_dist_key.to_string(),
                &user_reward_info,
            )?;
        }
    } else {
        for dist_itr in stakeable_token.distribution.iter() {
            let mut pool_dist_key: String = asset_infos.to_string();
            pool_dist_key.push_str(&dist_itr.token.to_string());
            let mut user_pool_dist_key = info.to_string();
            user_pool_dist_key.push_str(&pool_dist_key.to_string());

            //getting pool reward index
            let current_reward_index =
                get_reward_index_map_from_map_storage(deps.storage, pool_dist_key.to_string());

            //getting user reward index
            let mut user_reward_info = get_user_reward_info_from_map_storage(
                deps.storage,
                user_pool_dist_key.to_string(),
                current_reward_index,
            );

            //calculating reward to be distributed
            let diff_priv_and_curr_reward_index =
                current_reward_index - user_reward_info.reward_index;

            user_reward_info.pending_reward +=
                diff_priv_and_curr_reward_index.multiply_ratio(user_staked, Uint128::from(1u128));
            user_reward_info.reward_index = current_reward_index;

            USER_REWARD_INFO_MAP.save(
                deps.storage,
                user_pool_dist_key.to_string(),
                &user_reward_info,
            )?;

            // calculating reward index for auto compounding
            if user_opt_for_auto_compound {
                let current_compounded_reward_index = POOL_COMPOUNDED_INDEX_MAP
                    .may_load(deps.storage, pool_dist_key.to_string())?
                    .unwrap_or_else(Uint128::zero);

                let mut user_compounded_reward_index = USER_COMPOUNDED_REWARD_INFO_MAP
                    .may_load(deps.storage, user_pool_dist_key.to_string())?
                    .unwrap_or_else(|| RewardInfo {
                        reward_index: Uint128::zero(),
                        pending_reward: Uint128::zero(),
                    });

                let diff_priv_and_curr_compound_reward_index =
                    current_compounded_reward_index - user_compounded_reward_index.reward_index;

                user_compounded_reward_index.pending_reward +=
                    diff_priv_and_curr_compound_reward_index
                        .multiply_ratio(user_staked, Uint128::from(1u128));

                user_compounded_reward_index.reward_index = current_compounded_reward_index;

                USER_COMPOUNDED_REWARD_INFO_MAP.save(
                    deps.storage,
                    user_pool_dist_key.to_string(),
                    &user_compounded_reward_index,
                )?;
            }
        }
    };
    // adding amount share to the user staked
    user_staked += share;

    // mint FLP token to user
    let receiver = info.to_string();
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: stakeable_token.liquidity_token,
        msg: to_binary(&Cw20ExecuteMsg::Mint {
            recipient: receiver,
            amount: share,
        })?,
        funds: vec![],
    }));

    // resetting time for non-compounding user and if user has subscribed for auto compounded then updating the pool total compounded amount
    if user_opt_for_auto_compound {
        let mut total_compounded = POOL_TOTAL_COMPOUNDED_AMOUNT
            .may_load(deps.storage, asset_infos.to_string())?
            .unwrap_or_else(Uint128::zero);
        total_compounded += share;
        POOL_TOTAL_COMPOUNDED_AMOUNT.save(deps.storage, asset_infos, &total_compounded)?;
    } else {
        USER_REWARD_STARTING_TIME_MAP.save(
            deps.storage,
            user_pool_key.to_string(),
            &env.block.time.seconds(),
        )?;
    }

    USER_STAKED_AMOUNT.save(deps.storage, user_pool_key, &user_staked)?;

    Ok(Response::new()
        .add_attributes(vec![
            ("action", "staked"),
            ("sender", info.as_str()),
            ("receiver", info.as_str()),
        ])
        .add_messages(messages))
}

//Allow users to unstake tokens from farming contract.
pub fn execute_unstake_and_claim(
    deps: DepsMut,
    env: Env,
    sender: String,
    pool_address: String,
    amount: Uint128,
    liquidity_token_addr: String,
    is_reward_claimed: bool,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;

    //freezing the unstaking mechanism
    if config.freeze {
        return Err(StdError::generic_err(
            "Sorry for inconvenience, system is under maintenance. Kindly check again later",
        ));
    }

    let mut stakeable_token = if let Some(stakeable_token) =
        STAKEABLE_INFOS.may_load(deps.storage, pool_address.to_string())?
    {
        stakeable_token
    } else {
        return Err(StdError::generic_err("Incorrect pool address Provided"));
    };

    let mut message: String = String::from("");
    let mut messages: Vec<CosmosMsg> = Vec::new();

    let mut user_pool_key: String = sender.to_string();
    user_pool_key.push_str(&stakeable_token.token.to_string());

    let user_staked =
        get_user_staked_amount_in_pool_from_map_storage(deps.storage, user_pool_key.to_string());

    if liquidity_token_addr != stakeable_token.liquidity_token {
        return Err(StdError::generic_err(
            "Invalid token provided, Kindly provide only FLP tokens",
        ));
    }

    if amount != user_staked {
        return Err(StdError::generic_err(
            "FLP token not provided, Kindly provide all flp tokens",
        ));
    }

    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        //burning FLP token
        contract_addr: stakeable_token.liquidity_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
        funds: vec![],
    }));

    // sending user staked amount back to the user
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: stakeable_token.token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: sender.to_string(),
            amount: user_staked,
        })?,
        funds: vec![],
    }));
    // updating user record
    let mut total_staked = get_total_staked_amount_in_pool_from_map_storage(
        deps.storage,
        stakeable_token.token.to_string(),
    );
    total_staked -= user_staked;
    TOTAL_STAKED.save(
        deps.storage,
        stakeable_token.token.to_string(),
        &total_staked,
    )?;
    USER_STAKED_AMOUNT.save(deps.storage, user_pool_key.clone(), &Uint128::zero())?;

    let mut user_opt_for_auto_compound = false;
    if let Some(true) = USER_AUTO_COMPOUND_SUBSCRIPTION_MAP
        .may_load(deps.storage, user_pool_key.to_string())
        .unwrap()
    {
        user_opt_for_auto_compound = true;
    }
    let user_staked_time =
        USER_REWARD_STARTING_TIME_MAP.load(deps.storage, user_pool_key.to_string())?;
    let mut total_compounded = POOL_TOTAL_COMPOUNDED_AMOUNT
        .may_load(deps.storage, pool_address.to_string())?
        .unwrap_or_else(Uint128::zero);

    // subtracting user staked amount from total compounded amount in the pool
    if user_opt_for_auto_compound {
        total_compounded -= user_staked;
        POOL_TOTAL_COMPOUNDED_AMOUNT.save(
            deps.storage,
            pool_address.to_string(),
            &total_compounded,
        )?;
        USER_AUTO_COMPOUND_SUBSCRIPTION_MAP
            .save(deps.storage, user_pool_key.to_string(), &false)
            .unwrap();
    }

    //getting reward amounts from the linked distributed tokens of the stakeable token
    for dist_tkn in stakeable_token.distribution.iter_mut() {
        let mut pool_dist_key: String = stakeable_token.token.to_string();
        pool_dist_key.push_str(&dist_tkn.token.to_string());
        let mut user_pool_dist_key = sender.to_string();
        user_pool_dist_key.push_str(&pool_dist_key.to_string());

        //calculate user's reward
        let reward_to_be_dist = calculate_reward(
            deps.storage,
            user_pool_dist_key.to_string(),
            //&mut stakeable_token,
            pool_dist_key.to_string(),
            &user_opt_for_auto_compound,
            &user_staked,
            dist_tkn,
        );

        if is_reward_claimed && // check if reward is claimed or not 
                        (( user_opt_for_auto_compound && env.block.time.seconds() - user_staked_time >= config.lock_time_frame_for_compound_reward) //if the user has subscribed for auto compound then check the time with lock time frame for compound reward
                        || (!user_opt_for_auto_compound
                        && env.block.time.seconds() - user_staked_time >= config.lock_time_frame)) //if the user has subscribed for non-compound then check if lock time frame is passed for user logged staked time
                        && reward_to_be_dist != Uint128::zero()
        {
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                //sending reward to user
                contract_addr: dist_tkn.token.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: sender.to_string(),
                    amount: reward_to_be_dist,
                })?,
                funds: vec![],
            }));
        } else {
            // if user does not claim reward
            dist_tkn.reserve_amount += reward_to_be_dist;

            if total_compounded != Uint128::zero() {
                // if total compounded is not zero than adding reward index in compounded reward index
                let mut compounded_index = POOL_COMPOUNDED_INDEX_MAP
                    .may_load(deps.storage, pool_dist_key.to_string())?
                    .unwrap_or_else(Uint128::zero);
                compounded_index += reward_to_be_dist
                    .multiply_ratio(REWARD_CALCULATION_DECIMAL_PRECISION, total_compounded);

                POOL_COMPOUNDED_INDEX_MAP.save(
                    deps.storage,
                    pool_dist_key.to_string(),
                    &compounded_index,
                )?;
            } else if dist_tkn.reserve_amount != Uint128::zero() {
                // if distribution token reserve amount is not zero than transfering amount to reserve address
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    //sending reward to user
                    contract_addr: dist_tkn.token.to_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: config.reserve_addr.to_string(),
                        amount: dist_tkn.reserve_amount,
                    })?,
                    funds: vec![],
                }));

                dist_tkn.reserve_amount = Uint128::zero();
            }
            // if user is not availing the reward
        }
        USER_REWARD_INFO_MAP.remove(deps.storage, user_pool_dist_key.to_string());
    }
    STAKEABLE_INFOS.save(deps.storage, pool_address, &stakeable_token)?;
    message.push_str("Unstake");
    if is_reward_claimed {
        message.push_str(" and claim")
    }

    Ok(Response::new()
        .add_messages(messages)
        .add_attribute("action", message))
}

//To claim rewards only
pub fn execute_claim_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool_address: String,
) -> StdResult<Response> {
    let mut messages: Vec<CosmosMsg> = Vec::new();
    let config = CONFIG.load(deps.storage)?;
    if config.freeze {
        return Err(StdError::generic_err(
            "Sorry for inconvenience, system is under maintenance. Kindly check again later",
        ));
    }
    let user = info.sender.into_string();
    let mut user_pool_key = user.to_string();
    user_pool_key.push_str(&pool_address);
    if let Some(mut stakeable_token) =
        STAKEABLE_INFOS.may_load(deps.storage, pool_address.to_string())?
    {
        let user_staked_time =
            USER_REWARD_STARTING_TIME_MAP.load(deps.storage, user_pool_key.to_string())?;
        let user_staked = get_user_staked_amount_in_pool_from_map_storage(
            deps.storage,
            user_pool_key.to_string(),
        );
        let mut user_opt_for_auto_compound = false;
        if let Some(true) = USER_AUTO_COMPOUND_SUBSCRIPTION_MAP
            .may_load(deps.storage, user_pool_key.to_string())
            .unwrap()
        {
            user_opt_for_auto_compound = true;
        }
        if (user_opt_for_auto_compound
            && env.block.time.seconds() - user_staked_time
                >= config.lock_time_frame_for_compound_reward) //if the user has subscribed for auto compound then check the time with lock time frame for compound reward
            || (!user_opt_for_auto_compound
                && env.block.time.seconds() - user_staked_time >= config.lock_time_frame)
        //if the user has subscribed for non-compound then check if lock time frame is passed for user logged staked time
        {
            //getting reward amounts from the linked distributed tokens of the stakeable token
            for dist_tkn in stakeable_token.distribution.iter_mut() {
                let mut pool_dist_key: String = pool_address.to_string();
                pool_dist_key.push_str(&dist_tkn.token.to_string());
                let mut user_pool_dist_key = user.to_string();
                user_pool_dist_key.push_str(&pool_dist_key.to_string());
                let reward_to_be_dist = calculate_reward(
                    deps.storage,
                    user_pool_dist_key.to_string(),
                    pool_dist_key,
                    &user_opt_for_auto_compound,
                    &user_staked,
                    dist_tkn,
                );

                if reward_to_be_dist != Uint128::zero() {
                    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                        //sending reward to user
                        contract_addr: dist_tkn.token.to_string(),
                        msg: to_binary(&Cw20ExecuteMsg::Transfer {
                            recipient: user.to_string(),
                            amount: reward_to_be_dist,
                        })?,
                        funds: vec![],
                    }));
                }
            }
        } else {
            return Err(StdError::generic_err(
                "User claim reward time not reached yet",
            ));
        }
        STAKEABLE_INFOS.save(deps.storage, pool_address.to_string(), &stakeable_token)?;
    } else {
        return Err(StdError::generic_err("Pool address not valid"));
    }
    USER_AUTO_COMPOUND_SUBSCRIPTION_MAP
        .save(deps.storage, user_pool_key.to_string(), &false)
        .unwrap();
    Ok(Response::new().add_messages(messages))
}

pub fn calculate_reward(
    store: &mut dyn Storage,
    user_pool_dist_key: String,
    //stakeable_token: &mut StakeableToken,
    pool_dist_key: String,
    user_opt_for_auto_compound: &bool,
    user_staked: &Uint128,
    dist_tkn: &mut StakeablePairedDistributionTokenInfo,
) -> Uint128 {
    //getting total amount of rewards in the pool
    let mut total_rewards_in_pool =
        get_total_reward_in_pool_from_map_storage(store, pool_dist_key.to_string());

    //getting unclaimed distributed amount of contract
    let mut total_rewards_of_ditributed_tokens_in_contract =
        get_unclaimed_distirbuted_token_amount_from_map_storage(store, dist_tkn.token.to_string());

    //getting user reward index
    let mut user_reward_info = get_user_reward_info_from_map_storage(
        store,
        user_pool_dist_key.to_string(),
        Uint128::zero(),
    );

    //calculate reward amount
    let current_reward_index =
        get_reward_index_map_from_map_storage(store, pool_dist_key.to_string());

    // getting user reward difference from it's last stake to current pool index
    let diff_priv_and_curr_reward_index = current_reward_index - user_reward_info.reward_index;

    let mut reward_to_be_dist =
        diff_priv_and_curr_reward_index.multiply_ratio(*user_staked, Uint128::new(1u128));

    reward_to_be_dist += user_reward_info.pending_reward;

    total_rewards_in_pool -= reward_to_be_dist.multiply_ratio(
        Uint128::new(1u128),
        Uint128::new(REWARD_CALCULATION_DECIMAL_PRECISION),
    );
    total_rewards_of_ditributed_tokens_in_contract -= reward_to_be_dist.multiply_ratio(
        Uint128::new(1u128),
        Uint128::new(REWARD_CALCULATION_DECIMAL_PRECISION),
    );
    if *user_opt_for_auto_compound {
        // calculating reward for auto compounding amount
        let mut user_compounded_reward_index = USER_COMPOUNDED_REWARD_INFO_MAP
            .may_load(store, user_pool_dist_key.to_string())
            .unwrap()
            .unwrap_or_else(|| RewardInfo {
                reward_index: Uint128::zero(),
                pending_reward: Uint128::zero(),
            });

        let pool_compound_reward_index = POOL_COMPOUNDED_INDEX_MAP
            .may_load(store, pool_dist_key.to_string())
            .unwrap()
            .unwrap_or_else(Uint128::zero);

        let diff_priv_and_curr_compound_reward_index =
            pool_compound_reward_index - user_compounded_reward_index.reward_index;
        let mut compound_reward = diff_priv_and_curr_compound_reward_index
            .multiply_ratio(*user_staked, Uint128::new(1u128));
        compound_reward += user_compounded_reward_index.pending_reward;

        reward_to_be_dist += compound_reward;

        dist_tkn.reserve_amount -= compound_reward.multiply_ratio(
            Uint128::new(1u128),
            Uint128::new(REWARD_CALCULATION_DECIMAL_PRECISION),
        );

        user_compounded_reward_index.pending_reward = Uint128::zero();
        user_compounded_reward_index.reward_index = pool_compound_reward_index;
        USER_COMPOUNDED_REWARD_INFO_MAP
            .save(
                store,
                user_pool_dist_key.to_string(),
                &user_compounded_reward_index,
            )
            .unwrap();
    }

    reward_to_be_dist = reward_to_be_dist.multiply_ratio(
        Uint128::new(1u128),
        Uint128::new(REWARD_CALCULATION_DECIMAL_PRECISION),
    );
    //updating the reward values

    user_reward_info.reward_index = current_reward_index;
    user_reward_info.pending_reward = Uint128::zero();

    //adjusting values of calculating reward in pool and contract

    TOTAL_REWARDS_IN_POOL
        .save(store, pool_dist_key, &total_rewards_in_pool.clone())
        .unwrap();

    UNCLAIMED_DISTRIBUTED_TOKEN_AMOUNT_MAP
        .save(
            store,
            dist_tkn.token.to_string(),
            &total_rewards_of_ditributed_tokens_in_contract,
        )
        .unwrap();

    USER_REWARD_INFO_MAP
        .save(store, user_pool_dist_key, &user_reward_info)
        .unwrap();
    reward_to_be_dist
}

/*
   paginated distribution and whenever it calls, it will distribute
   the reward with difference from last distrbution time upto current time
*/
pub fn execute_distribute_by_limit(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;
    let mut bound = None;
    if let Some(start_after) = start_after  {
        if STAKEABLE_INFOS
            .may_load(deps.storage, start_after.to_string())?
            .is_some()
        {
            bound = Some(Bound::exclusive(start_after));
        } else {
            return Err(StdError::generic_err("not a valid address passed"));
        }
    }

    let limit = limit.unwrap_or(config.default_limit).min(config.max_limit) as usize;
    // get all the values of STAKEABLE_TOKENS in form of collection vectors
    let stakeable_tokens_collection_result: StdResult<Vec<_>> = STAKEABLE_INFOS
        .range(deps.storage, bound, None, Order::Ascending)
        .take(limit)
        .collect();

    let mut last_distributed_pool = String::from("");
    if let Ok(stakeable_tokens_collection_list) = stakeable_tokens_collection_result {
        for staleable_token_vec_obj in stakeable_tokens_collection_list.iter() {
            /*
            Checks for time last distirbution was called,
            adding 1 sec for 1 se gap in execution of distribution.
            Save the reward in the storage.
            */
            let last_distributed = if let Some(last_distribute) =
                POOL_LAST_DISTRIBUTION_TIME_IN_SECONDS
                    .may_load(deps.storage, staleable_token_vec_obj.1.token.to_string())?
            {
                last_distribute + 1
            } else {
                env.block.time.seconds()
            };
            // checking current time with last distributed time to check there must be a 1 sec gap
            if last_distributed < env.block.time.seconds() {
                last_distributed_pool = staleable_token_vec_obj.1.token.to_string();
                for distributed_token in staleable_token_vec_obj.1.distribution.iter() {
                    if distributed_token.amount == Uint128::zero() {
                        continue;
                    }
                    let mut pool_dist_key: String = staleable_token_vec_obj.1.token.to_string();
                    pool_dist_key.push_str(&distributed_token.token.to_string());

                    let mut total_rewards_in_pool = get_total_reward_in_pool_from_map_storage(
                        deps.storage,
                        pool_dist_key.clone(),
                    );

                    let mut total_rewards_of_ditributed_tokens_in_contract =
                        get_unclaimed_distirbuted_token_amount_from_map_storage(
                            deps.storage,
                            distributed_token.token.to_string(),
                        );

                    let stakeable_token = staleable_token_vec_obj.1.clone();
                    let total_staked = get_total_staked_amount_in_pool_from_map_storage(
                        deps.storage,
                        stakeable_token.token.to_string(),
                    );
                    let mut total_accumulative_distributed_amount =
                        get_total_accumulated_distributed_amount_in_pool_from_map_storage(
                            deps.storage,
                            pool_dist_key.to_string(),
                        );
                    let reward_time_precission =
                        (env.block.time.seconds() - last_distributed) as u128;
                    let total_reward_to_be_dist_ratio: Uint128 =
                        Uint128::new(reward_time_precission).multiply_ratio(
                            distributed_token.amount,
                            config.wait_time_for_distribution_in_seconds,
                        );
                    //checking if contract has sufficient funds to allow admin to distribute
                    let balance = query_token_balance(
                        &deps.querier,
                        deps.api
                            .addr_validate(&distributed_token.token.to_string())?,
                        env.contract.address.clone(),
                    )?;
                    //checking total rewards should be less than the rewards assigned to the pool - distributed
                    // let balance = Uint128::from(100u128);
                    if total_reward_to_be_dist_ratio
                        <= (balance
                            - total_rewards_of_ditributed_tokens_in_contract
                            - distributed_token.reserve_amount)
                    {
                        let mut reward_index = get_reward_index_map_from_map_storage(
                            deps.storage,
                            pool_dist_key.to_string(),
                        );

                        if total_staked.is_zero() {
                            reward_index = Uint128::zero();
                        } else {
                            total_rewards_in_pool += total_reward_to_be_dist_ratio;
                            total_accumulative_distributed_amount += total_reward_to_be_dist_ratio;
                            total_rewards_of_ditributed_tokens_in_contract +=
                                total_reward_to_be_dist_ratio;

                            reward_index += Uint128::new(REWARD_CALCULATION_DECIMAL_PRECISION)
                                .multiply_ratio(total_reward_to_be_dist_ratio, total_staked);
                        }

                        POOL_REWARD_INDEX_MAP.save(
                            deps.storage,
                            pool_dist_key.to_string(),
                            &reward_index,
                        )?;
                        TOTAL_REWARDS_IN_POOL.save(
                            deps.storage,
                            pool_dist_key.to_string(),
                            &total_rewards_in_pool,
                        )?;
                        TOTAL_ACCUMULATED_DISTRIBUTED_AMOUNT_IN_POOL_MAP.save(
                            deps.storage,
                            pool_dist_key.to_string(),
                            &total_accumulative_distributed_amount,
                        )?;
                        UNCLAIMED_DISTRIBUTED_TOKEN_AMOUNT_MAP.save(
                            deps.storage,
                            distributed_token.token.to_string(),
                            &total_rewards_of_ditributed_tokens_in_contract,
                        )?;
                    } else {
                        return Err(StdError::generic_err("insufficient funds  "));
                    }
                    POOL_LAST_DISTRIBUTION_TIME_IN_SECONDS.save(
                        deps.storage,
                        staleable_token_vec_obj.1.token.to_string(),
                        &(env.block.time.seconds()),
                    )?;
                }
            } else {
                POOL_LAST_DISTRIBUTION_TIME_IN_SECONDS.save(
                    deps.storage,
                    staleable_token_vec_obj.1.token.to_string(),
                    &env.block.time.seconds(),
                )?;
            }
        }
    } else {
        return Err(StdError::generic_err(
            "please wait patiently for the specified time",
        ));
    }
    //config.last_distributed = last_distributed;
    CONFIG.save(deps.storage, &config)?;
    Ok(
        Response::new().add_attribute(" ", last_distributed_pool), // ("stats", stats),
    )
}

//Allow admin to update and set the daily reward for each pool. If admin do not set any reward for pool it
// will continue the same tokens.
pub fn execute_update_reward(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    pool: String,
    rewards: Vec<(String, Uint128)>,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner
        && (config.second_owner.is_empty()
            || (!config.second_owner.is_empty() && info.sender != config.second_owner))
    {
        return Err(StdError::generic_err("unauthorized"));
    }

    //get user stakeable token from STAKEABLE_TOKENS map

    let mut stakeable_token =
        if let Some(stakeable_token) = STAKEABLE_INFOS.may_load(deps.storage, pool.to_string())? {
            stakeable_token
        } else {
            return Err(StdError::generic_err("correct info not provided"));
        };

    for reward_tuple in rewards.iter() {
        //let mut is_provided_token_distributeable: bool = false;
        let mut is_provided_token_already_exists: bool = false;
        //to update the reward of the distributed tokens contained by a stakeable token
        deps.api.addr_validate(&reward_tuple.0.to_string())?;

        for distributed_token in stakeable_token.distribution.iter_mut() {
            //is_stakeable_token_contains_distibuteable_tokens = true;
            if reward_tuple.0 == distributed_token.token {
                is_provided_token_already_exists = true;
                distributed_token.amount = reward_tuple.1;
                break;
            }
        }

        if !is_provided_token_already_exists {
            stakeable_token
                .distribution
                .push(StakeablePairedDistributionTokenInfo {
                    token: reward_tuple.0.to_string(),
                    amount: reward_tuple.1,
                    reserve_amount: Uint128::zero(),
                });
        }
    }
    // update in the stakeable token map
    STAKEABLE_INFOS.save(deps.storage, pool, &stakeable_token)?;

    Ok(Response::new().add_attribute("action", "calculate reward"))
}

///update stakeable token address
pub fn execute_update_stakeable_token_address(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    old_address: String,
    new_address: String,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner
        && (config.second_owner.is_empty()
            || (!config.second_owner.is_empty() && info.sender != config.second_owner))
    {
        return Err(StdError::generic_err("unauthorized"));
    }

    let last_distributed = if let Some(last_distribute) =
        POOL_LAST_DISTRIBUTION_TIME_IN_SECONDS.may_load(deps.storage, old_address.to_string())?
    {
        last_distribute
    } else {
        env.block.time.seconds()
    };

    if let Some(mut stakeable_token) =
        STAKEABLE_INFOS.may_load(deps.storage, old_address.to_string())?
    {
        STAKEABLE_INFOS.remove(deps.storage, old_address);
        stakeable_token.token = new_address.to_string();
        STAKEABLE_INFOS.save(deps.storage, new_address.to_string(), &stakeable_token)?;

        POOL_LAST_DISTRIBUTION_TIME_IN_SECONDS.save(
            deps.storage,
            new_address,
            &last_distributed,
        )?;
    } else {
        return Err(StdError::generic_err("no pool exists with given address"));
    }

    Ok(Response::new().add_attribute("action", "Pool address updated"))
}

/// delete stakeable token
pub fn execute_delete_stakeable_token(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    address: String,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner
        && (config.second_owner.is_empty()
            || (!config.second_owner.is_empty() && info.sender != config.second_owner))
    {
        return Err(StdError::generic_err("unauthorized"));
    }

    if STAKEABLE_INFOS
        .may_load(deps.storage, address.to_string())?
        .is_some()
    {
        STAKEABLE_INFOS.remove(deps.storage, address);
    } else {
        return Err(StdError::generic_err("pool does not exist in the contract"));
    }

    Ok(Response::new().add_attribute("action", "Pool address deleted"))
}

///delete distributeable token
pub fn execute_delete_distributeable_token(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    pool_address: String,
    token_address: String,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner
        && (config.second_owner.is_empty()
            || (!config.second_owner.is_empty() && info.sender != config.second_owner))
    {
        return Err(StdError::generic_err("unauthorized"));
    }

    let mut index = 0;
    let mut found = false;
    if let Some(mut stakeable_token) =
        STAKEABLE_INFOS.may_load(deps.storage, pool_address.to_string())?
    {
        for distributed_token in stakeable_token.distribution.iter_mut() {
            //is_stakeable_token_contains_distibuteable_tokens = true;/

            if token_address == distributed_token.token {
                found = true;
                break;
            }
            index += 1;
        }
        match found {
            true => {
                stakeable_token.distribution.swap_remove(index);
                STAKEABLE_INFOS.save(deps.storage, pool_address, &stakeable_token)?;
            }
            false => {
                return Err(StdError::generic_err(
                    "distributeable_token does not exist in the pool",
                ));
            }
        }
    }

    Ok(Response::new().add_attribute("action", "distributeable_token address deleted"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryRewardInPool {
            pool,
            distribution_token,
        } => to_binary(&query_reward_in_pool(deps, env, pool, distribution_token)?),
        QueryMsg::QueryStakedByUser {
            wallet,
            staked_token,
        } => to_binary(&query_staked_by_user(deps, env, wallet, staked_token)?),
        QueryMsg::QueryTotalStaked { staked_token } => {
            to_binary(&query_staked(deps, env, staked_token)?)
        }
        QueryMsg::QueryListOfStakeableTokens { start_after, limit } => to_binary(
            &query_list_of_stakeable_tokens(deps, env, start_after, limit)?,
        ),
        QueryMsg::QueryListOfDistributableTokensByPool { pool } => {
            to_binary(&query_pool_rewards(deps, env, pool)?)
        }
        QueryMsg::QueryStakeableInfo { start_after, limit } => {
            to_binary(&query_stakeable_info(deps, start_after, limit)?)
        }
        QueryMsg::QueryUserRewardInPool { wallet, pool } => {
            to_binary(&query_user_reward_in_pool(deps, env, wallet, pool)?)
        }
        QueryMsg::QueryUserStakedTime { wallet, pool } => {
            to_binary(&query_user_staked_time(deps, wallet, pool)?)
        }
        QueryMsg::QueryDistributionWaitTime {} => to_binary(&query_distribution_wait_time(deps)?),
        QueryMsg::QueryLockTimeFrame {} => to_binary(&query_lock_time_frame(deps)?),
        QueryMsg::QueryLockTimeFrameForAutoCompound {} => {
            to_binary(&query_lock_time_frame_for_auto_compound(deps)?)
        }
        QueryMsg::QueryLastDistributionTime { pool_address } => {
            to_binary(&query_last_distribution_time(deps, pool_address)?)
        }
        QueryMsg::QueryTotalDistributedAmountInPool {
            pool,
            dist_token_addr,
        } => to_binary(&query_total_ditributed_amount_in_pool(
            deps,
            pool,
            dist_token_addr,
        )?),
        QueryMsg::QuerySecondAdminAddress {} => to_binary(&query_get_second_admin_address(deps)?),
        QueryMsg::QueryGetDistributeableTokenBalance { dist_token_addr } => to_binary(
            &query_get_distibuteable_token_balance(deps, env, dist_token_addr)?,
        ),
        QueryMsg::QueryGetUserAutoCompoundSubription {
            user_address,
            pool_address,
        } => to_binary(&query_get_user_auto_compound_subscription(
            deps,
            env,
            user_address,
            pool_address,
        )?),

        QueryMsg::QueryGetTotalCompounded { pool_addr } => {
            to_binary(&query_get_total_compounded(deps, pool_addr)?)
        }
        QueryMsg::QueryFlpTokenFromPoolAddress { pool_address } => {
            to_binary(&query_flp_token_address(deps, pool_address)?)
        }
    }
}

pub fn query_get_total_compounded(deps: Deps, pool_address: String) -> StdResult<Uint128> {
    Ok(POOL_TOTAL_COMPOUNDED_AMOUNT
        .may_load(deps.storage, pool_address)?
        .unwrap_or_else(Uint128::zero))
}

pub fn query_get_user_auto_compound_subscription(
    deps: Deps,
    _env: Env,
    user_address: String,
    pool_address: String,
) -> StdResult<bool> {
    let mut user_pool_address = user_address;
    user_pool_address.push_str(&pool_address);
    let user_opt_for_auto_compound = if let Some(user_opt_for_auto_compound) =
        USER_AUTO_COMPOUND_SUBSCRIPTION_MAP.may_load(deps.storage, user_pool_address)?
    {
        user_opt_for_auto_compound
    } else {
        false
    };
    Ok(user_opt_for_auto_compound)
}

//for testing only. gives us amount of particular reward in a pool
pub fn query_reward_in_pool(
    deps: Deps,
    _env: Env,
    pool: String,
    distribution_token: String,
) -> StdResult<Uint128> {
    let mut resp = Uint128::zero();
    let mut key: String = pool;
    key.push_str(&distribution_token);
    let result = TOTAL_REWARDS_IN_POOL.may_load(deps.storage, key.clone())?;

    if let Some(result) = result {
        resp = result;
    }

    Ok(resp)
}

// Tells us about the staked value of pool by user.
pub fn query_staked_by_user(
    deps: Deps,
    _env: Env,
    wallet: String,
    staked_token: String,
) -> StdResult<Uint128> {
    let mut resp = Uint128::zero();
    let mut key = wallet;
    key.push_str(&staked_token);
    let result = USER_STAKED_AMOUNT.may_load(deps.storage, key.clone())?;
    if let Some(result) = result {
        resp = result;
    }

    Ok(resp)
}

//Informs us about all staked value in a pool
pub fn query_staked(deps: Deps, _env: Env, staked_token: String) -> StdResult<Uint128> {
    let mut resp = Uint128::zero();
    let key = staked_token;
    let result = TOTAL_STAKED.may_load(deps.storage, key)?;

    if let Some(result) = result {
        resp = result;
    }

    Ok(resp)
}

// paginated list of stakeable_tokens
pub fn query_list_of_stakeable_tokens(
    deps: Deps,
    _env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<StakeableToken>> {
    let mut bound = None;
    if let Some(start_after) = start_after {
        if STAKEABLE_INFOS
            .may_load(deps.storage, start_after.to_string())?
            .is_some()
        {
            bound = Some(Bound::exclusive(start_after));
        } else {
            return Err(StdError::generic_err("not a valid address passed"));
        }
    }
    let config = CONFIG.load(deps.storage).unwrap();
    let limit = limit.unwrap_or(config.default_limit).min(config.max_limit) as usize;
    let stakeable_infos_tokens_result: StdResult<Vec<_>> = STAKEABLE_INFOS
        .range(deps.storage, bound, None, Order::Ascending)
        .take(limit)
        .collect();
    let mut st: Vec<StakeableToken> = vec![];
    if let Ok(stakeable_infos_token) = stakeable_infos_tokens_result {
        for i in stakeable_infos_token {
            st.push(i.1);
        }
    }

    Ok(st)
}

//Will pass the pool info. Tell about all tokens to be distributed to that pool...
pub fn query_pool_rewards(
    deps: Deps,
    _env: Env,
    pool: String,
) -> StdResult<Vec<QueryRewardResponse>> {
    let mut resp: Vec<QueryRewardResponse> = vec![];
    if let Some(stakeable_token) = STAKEABLE_INFOS.may_load(deps.storage, pool)? {
        for stakeable_token_distribution in stakeable_token.distribution.iter() {
            let temp = QueryRewardResponse {
                info: stakeable_token_distribution.token.clone(),
                daily_reward: stakeable_token_distribution.amount,
            };
            resp.push(temp);
        }
    } else {
        return Err(StdError::generic_err("correct into not provided"));
    }

    Ok(resp)
}
// testing only. gives us the paginated LP token generated against the stakeable token
pub fn query_stakeable_info(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<StakeableToken>> {
    let mut bound = None;
    if let Some(start_after) = start_after {
        if STAKEABLE_INFOS
            .may_load(deps.storage, start_after.to_string())?
            .is_some()
        {
            bound = Some(Bound::exclusive(start_after));
        } else {
            return Err(StdError::generic_err("not a valid address passed"));
        }
    }
    let config = CONFIG.load(deps.storage).unwrap();
    let limit = limit.unwrap_or(config.default_limit).min(config.max_limit) as usize;
    let stakeable_tokens_result: StdResult<Vec<_>> = STAKEABLE_INFOS
        .range(deps.storage, bound, None, Order::Ascending)
        .take(limit)
        .collect();
    let mut st: Vec<StakeableToken> = vec![];
    if let Ok(stakeable_tokens) = stakeable_tokens_result {
        for i in stakeable_tokens {
            st.push(i.1);
        }
    }
    Ok(st)
}

// Tell reward of users of the requested pools.
pub fn query_user_reward_in_pool(
    deps: Deps,
    _env: Env,
    wallet: String,
    pool_address: String,
) -> StdResult<Vec<QueryUserRewardInPoolResponse>> {
    let mut resp: Vec<QueryUserRewardInPoolResponse> = vec![];
    if let Some(stakeable_token) =
        STAKEABLE_INFOS.may_load(deps.storage, pool_address.to_string())?
    {
        let mut resp2 = QueryUserRewardInPoolResponse {
            pool: pool_address.to_string(),
            rewards_info: vec![],
        };
        let mut user_pool_key: String = wallet.to_string();
        user_pool_key.push_str(&pool_address);
        let mut user_opt_for_auto_compound = false;
        if let Some(true) =
            USER_AUTO_COMPOUND_SUBSCRIPTION_MAP.may_load(deps.storage, user_pool_key.to_string())?
        {
            user_opt_for_auto_compound = true;
        }
        let user_staked = if let Some(user_staked) =
            USER_STAKED_AMOUNT.may_load(deps.storage, user_pool_key.clone())?
        {
            user_staked
        } else {
            Uint128::zero()
        };

        for distt in stakeable_token.distribution.iter() {
            let mut user_pool_dist_key = wallet.to_string();
            let mut pool_dist_key: String = pool_address.to_string();
            pool_dist_key.push_str(&distt.token.to_string());
            user_pool_dist_key.push_str(&pool_dist_key.to_string());

            //getting user reward index
            let user_reward_info = if let Some(user_reward_info) =
                USER_REWARD_INFO_MAP.may_load(deps.storage, user_pool_dist_key.to_string())?
            {
                user_reward_info
            } else {
                RewardInfo {
                    reward_index: Uint128::zero(),
                    pending_reward: Uint128::zero(),
                }
            };

            //calculate reward amount
            let current_reward_index = if let Some(current_reward_index) =
                POOL_REWARD_INDEX_MAP.may_load(deps.storage, pool_dist_key.to_string())?
            {
                current_reward_index
            } else {
                Uint128::zero()
            };

            // getting user reward difference from it's last stake to current pool index
            let diff_priv_and_curr_reward_index =
                current_reward_index - user_reward_info.reward_index;

            //calculating reward to be distributed
            let mut reward_to_be_dist =
                diff_priv_and_curr_reward_index.multiply_ratio(user_staked, Uint128::from(1u128));
            reward_to_be_dist += user_reward_info.pending_reward;
            if user_opt_for_auto_compound {
                let user_compounded_reward_index = USER_COMPOUNDED_REWARD_INFO_MAP
                    .may_load(deps.storage, user_pool_dist_key.to_string())
                    .unwrap()
                    .unwrap_or_else(|| RewardInfo {
                        reward_index: Uint128::zero(),
                        pending_reward: Uint128::zero(),
                    });

                let pool_compound_reward_index = POOL_COMPOUNDED_INDEX_MAP
                    .may_load(deps.storage, pool_dist_key.to_string())
                    .unwrap()
                    .unwrap_or_else(Uint128::zero);

                let diff_priv_and_curr_compound_reward_index =
                    pool_compound_reward_index - user_compounded_reward_index.reward_index;
                let mut compound_reward = diff_priv_and_curr_compound_reward_index
                    .multiply_ratio(user_staked, Uint128::new(1u128));
                compound_reward += user_compounded_reward_index.pending_reward;
                reward_to_be_dist += compound_reward;
            }
            reward_to_be_dist = reward_to_be_dist.multiply_ratio(
                Uint128::new(1),
                Uint128::new(REWARD_CALCULATION_DECIMAL_PRECISION),
            );

            if reward_to_be_dist != Uint128::zero() {
                resp2
                    .rewards_info
                    .push((distt.token.clone(), reward_to_be_dist))
            }
        }

        resp.push(resp2);
    }
    Ok(resp)
}

pub fn get_unclaimed_distirbuted_token_amount_from_map_storage(
    store: &dyn Storage,
    distributed_token_address: String,
) -> Uint128 {
    let unclaimed_amount_of_distributed_token_in_contract: Uint128 =
        if let Some(unclaimed_amount_of_distributed_token_in_contract) =
            UNCLAIMED_DISTRIBUTED_TOKEN_AMOUNT_MAP
                .may_load(store, distributed_token_address)
                .unwrap()
        {
            unclaimed_amount_of_distributed_token_in_contract
        } else {
            Uint128::zero()
        };
    unclaimed_amount_of_distributed_token_in_contract
}

pub fn get_user_reward_info_from_map_storage(
    store: &dyn Storage,
    user_pool_dist_address: String,
    default_reward_index: Uint128,
) -> RewardInfo {
    let user_reward_info: RewardInfo = if let Some(user_reward_info) = USER_REWARD_INFO_MAP
        .may_load(store, user_pool_dist_address)
        .unwrap()
    {
        user_reward_info
    } else {
        RewardInfo {
            reward_index: default_reward_index,
            pending_reward: Uint128::zero(),
        }
    };
    user_reward_info
}

pub fn get_reward_index_map_from_map_storage(
    store: &dyn Storage,
    pool_dist_address: String,
) -> Uint128 {
    let reward_index: Uint128 = if let Some(reward_index) = POOL_REWARD_INDEX_MAP
        .may_load(store, pool_dist_address)
        .unwrap()
    {
        reward_index
    } else {
        Uint128::zero()
    };
    reward_index
}

pub fn get_total_accumulated_distributed_amount_in_pool_from_map_storage(
    store: &dyn Storage,
    pool_dist_address: String,
) -> Uint128 {
    let total_accumulate_amount_in_pool: Uint128 = if let Some(total_accumulate_amount_in_pool) =
        TOTAL_ACCUMULATED_DISTRIBUTED_AMOUNT_IN_POOL_MAP
            .may_load(store, pool_dist_address)
            .unwrap()
    {
        total_accumulate_amount_in_pool
    } else {
        Uint128::zero()
    };
    total_accumulate_amount_in_pool
}

pub fn get_total_reward_in_pool_from_map_storage(
    store: &dyn Storage,
    pool_address: String,
) -> Uint128 {
    let total_reward_in_pool: Uint128 = if let Some(total_reward_in_pool) =
        TOTAL_REWARDS_IN_POOL.may_load(store, pool_address).unwrap()
    {
        total_reward_in_pool
    } else {
        Uint128::zero()
    };
    total_reward_in_pool
}

pub fn get_total_staked_amount_in_pool_from_map_storage(
    store: &dyn Storage,
    pool_address: String,
) -> Uint128 {
    let total_staked_amount_in_pool: Uint128 = if let Some(total_staked_amount_in_pool) =
        TOTAL_STAKED.may_load(store, pool_address).unwrap()
    {
        total_staked_amount_in_pool
    } else {
        Uint128::zero()
    };
    total_staked_amount_in_pool
}

pub fn get_user_staked_amount_in_pool_from_map_storage(
    store: &dyn Storage,
    user_pool_address: String,
) -> Uint128 {
    let user_reward_issued_token_amount_in_pool: Uint128 =
        if let Some(user_reward_issued_token_amount_in_pool) = USER_STAKED_AMOUNT
            .may_load(store, user_pool_address)
            .unwrap()
        {
            user_reward_issued_token_amount_in_pool
        } else {
            Uint128::zero()
        };
    user_reward_issued_token_amount_in_pool
}

//query to get user staked time
pub fn query_user_staked_time(deps: Deps, wallet: String, pool: String) -> StdResult<String> {
    let mut user_pool_key = String::from(&wallet);
    user_pool_key.push_str(&pool);
    if let Some(user_staked_time) =
        USER_REWARD_STARTING_TIME_MAP.may_load(deps.storage, user_pool_key)?
    {
        Ok(user_staked_time.to_string())
    } else {
        Ok("".to_string())
    }
}

//query lock time frame
pub fn query_lock_time_frame(deps: Deps) -> StdResult<u64> {
    Ok(CONFIG.load(deps.storage)?.lock_time_frame)
}

//query lock time frame
pub fn query_lock_time_frame_for_auto_compound(deps: Deps) -> StdResult<u64> {
    Ok(CONFIG
        .load(deps.storage)?
        .lock_time_frame_for_compound_reward)
}

//query distribution wait time frame
pub fn query_distribution_wait_time(deps: Deps) -> StdResult<u64> {
    Ok(CONFIG
        .load(deps.storage)?
        .wait_time_for_distribution_in_seconds)
}

pub fn query_total_ditributed_amount_in_pool(
    deps: Deps,
    pool_addr: String,
    dist_token_addr: String,
) -> StdResult<Uint128> {
    let mut pool_dist_addr = pool_addr;
    pool_dist_addr.push_str(dist_token_addr.as_str());
    Ok(TOTAL_ACCUMULATED_DISTRIBUTED_AMOUNT_IN_POOL_MAP
        .load(deps.storage, pool_dist_addr)
        .unwrap_or_else(|_| Uint128::zero()))
}

pub fn query_get_second_admin_address(deps: Deps) -> StdResult<String> {
    let config: Config = CONFIG.load(deps.storage).unwrap();
    Ok(config.second_owner)
}

pub fn query_get_distibuteable_token_balance(
    deps: Deps,
    env: Env,
    dist_token_addr: String,
) -> StdResult<String> {
    let balance = get_unclaimed_distirbuted_token_amount_from_map_storage(
        deps.storage,
        dist_token_addr.to_string(),
    );

    Ok((query_token_balance(
        &deps.querier,
        deps.api.addr_validate(&dist_token_addr)?,
        env.contract.address,
    )? - balance)
        .to_string())
}

pub fn query_last_distribution_time(deps: Deps, pool_address: String) -> StdResult<u64> {
    Ok(POOL_LAST_DISTRIBUTION_TIME_IN_SECONDS
        .may_load(deps.storage, pool_address)?
        .unwrap_or(0u64))
}

pub fn query_flp_token_address(deps: Deps, pool_address: String) -> StdResult<String> {
    let stakeable_token =
        if let Some(stakeable_token) = STAKEABLE_INFOS.may_load(deps.storage, pool_address)? {
            stakeable_token
        } else {
            return Err(StdError::generic_err("Pool not found"));
        };

    Ok(stakeable_token.liquidity_token)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
    match msg.id {
        INSTANTIATE_REPLY_ID => handle_instantiate_reply(deps, msg),
        id => Err(StdError::generic_err(format!("Unknown reply id: {}", id))),
    }
}

fn handle_instantiate_reply(deps: DepsMut, msg: Reply) -> StdResult<Response> {
    // Handle the msg data and save the contract address
    // See: https://github.com/CosmWasm/cw-plus/blob/main/packages/utils/src/parse_reply.rs
    let res = parse_reply_instantiate_data(msg);

    if res.is_err() {
        return Err(StdError::generic_err(
            "no successful response get from parsing reply data",
        ));
    }

    let pool_address = CURRENT_POOL_ADDRESS.load(deps.storage)?;
    let mut stakeable_info = STAKEABLE_INFOS.load(deps.storage, pool_address.to_string())?;
    stakeable_info.liquidity_token = res.unwrap().contract_address;

    //saving against pool address
    STAKEABLE_INFOS.save(deps.storage, pool_address.to_string(), &stakeable_info)?;

    //saving against liquidity address
    //println!("{}" , input.liquidity_token.to_string());

    LIQUIDITY_TOKEN_MAP.save(
        deps.storage,
        stakeable_info.liquidity_token.to_string(),
        &pool_address,
    )?;

    Ok(Response::new().add_attribute(
        "liquidity_token_addr",
        stakeable_info.liquidity_token,
    ))
}

// #[cfg_attr(not(feature = "library"), entry_point)]
// pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> StdResult<Response> {
//     let data = msg.result.unwrap().data.unwrap();
//     let res: MsgInstantiateContractResponse =
//         Message::parse_from_bytes(data.as_slice()).map_err(|_| {
//             StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
//         })?;
//     let liquidity_token = res.get_contract_address();

//     let pool_address = CURRENT_POOL_ADDRESS.load(deps.storage)?;
//     let mut stakeable_info = STAKEABLE_INFOS.load(deps.storage, pool_address.to_string())?;
//     stakeable_info.liquidity_token = liquidity_token.to_string();

//     //saving against pool address
//     STAKEABLE_INFOS.save(deps.storage, pool_address.to_string(), &stakeable_info)?;

//     //saving against liquidity address
//     //println!("{}" , input.liquidity_token.to_string());

//     LIQUIDITY_TOKEN_MAP.save(deps.storage, liquidity_token.to_string(), &pool_address)?;

//     Ok(Response::new().add_attribute("liquidity_token_addr", liquidity_token))
// }
