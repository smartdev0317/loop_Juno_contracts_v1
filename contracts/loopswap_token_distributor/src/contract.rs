use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, UserInfo, CONFIG, USER_REWARD_MAP};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order, Response, StdError,
    StdResult, Uint128, WasmMsg,
};
use loopswap::factory::MigrateMsg;
use cw20_base::msg::ExecuteMsg as Cw20ExecuteMsg;
use cw20_base::ContractError;
use cw_storage_plus::Bound;
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        token_contract_address: msg.token_contract_address,
        admin: info.sender.to_string(),
    };
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::AssignReward {
            recipient,
            reward,
            duration,
        } => assign_reward(deps, env, info, recipient, reward, duration),
        ExecuteMsg::Claim {} => claim(deps, env, info),
        ExecuteMsg::UpdateRewardDuration {
            recipient,
            reward,
            duration,
        } => update_reward_duration(deps, info, recipient, reward, duration),
        ExecuteMsg::UpdateConfig {
            token_contract_address,
            admin,
        } => update_config(deps, info, token_contract_address, admin),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    _token_contract_address: Option<String>,
    _admin: Option<String>,
) -> StdResult<Response> {
    let mut config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(StdError::generic_err("unauthorized"));
    }
    if let Some(admin) = _admin {
        // Validate address format
        let _ = deps.api.addr_validate(&admin)?;

        config.admin = admin;
    }
    if let Some(token_contract_address) = _token_contract_address {
        // Validate address format
        let _ = deps.api.addr_validate(&token_contract_address)?;

        config.token_contract_address = token_contract_address;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

pub fn assign_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    recipient: String,
    reward: Uint128,
    duration: u64,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(StdError::generic_err("unauthorized"));
    }

    let user_info = USER_REWARD_MAP.may_load(deps.storage, recipient.to_string())?;
    if user_info.is_some() {
        return Err(StdError::generic_err("User already exist"));
    }
    let user_info = UserInfo {
        reward,
        assigned_time : env.block.time.seconds(),
        duration,
        address: recipient.to_string(),
    };
    USER_REWARD_MAP.save(deps.storage, recipient.to_string(), &user_info)?;

    Ok(Response::new().add_attribute("action", "assign reward"))
}

pub fn update_reward_duration(
    deps: DepsMut,
    info: MessageInfo,
    recipient: String,
    reward: Option<Uint128>,
    duration: Option<u64>,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;
    if info.sender != config.admin {
        return Err(StdError::generic_err("unauthorized"));
    }

    let user_info = USER_REWARD_MAP.may_load(deps.storage, recipient.to_string())?;
    if user_info.is_none() {
        return Err(StdError::generic_err("User not found"));
    }
    let mut user_info = user_info.unwrap();
    if reward.is_some() {
        user_info.reward = reward.unwrap();
    }

    if duration.is_some() {
        user_info.duration = duration.unwrap();
    }

    USER_REWARD_MAP.save(deps.storage, recipient.to_string(), &user_info)?;

    Ok(Response::new().add_attribute("action", "assign reward"))
}

pub fn claim(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;
    let user_info = USER_REWARD_MAP.may_load(deps.storage, info.sender.to_string())?;
    if user_info.is_none() {
        return Err(StdError::generic_err("No User Found"));
    }
    let mut user_info = user_info.unwrap();
    let reward_time_precission = (env.block.time.seconds() - user_info.assigned_time) as u128;
    let total_reward_to_be_claimed: Uint128 =
        Uint128::new(reward_time_precission).multiply_ratio(user_info.reward, user_info.duration);

    let message: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: config.token_contract_address,
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.to_string(),
            amount: total_reward_to_be_claimed.clone(),
        })?,
        funds: vec![],
    });

    user_info.assigned_time = env.block.time.seconds();
    USER_REWARD_MAP.save(deps.storage, info.sender.to_string(), &user_info)?;

    Ok(Response::new()
        .add_attribute("action", "claim")
        .add_attribute("amount", total_reward_to_be_claimed.to_string())
        .add_message(message))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::UserAssignedReward { recipient } => {
            to_binary(&query_user_assigned_reward(deps, recipient)?)
        }

        QueryMsg::UserReward { recipient } => to_binary(&query_user_reward(deps, env, recipient)?),

        QueryMsg::UsersReward { start_after, limit } => {
            to_binary(&query_users_reward(deps, start_after, limit)?)
        }
    }
}
pub fn query_users_reward(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u64>,
) -> StdResult<Vec<UserInfo>> {
    let mut bound = None;
    if start_after != None {
        if USER_REWARD_MAP
            .may_load(deps.storage, start_after.as_ref().unwrap().to_string())?
            .is_some()
        {
            bound = Some(Bound::exclusive(start_after.unwrap()));
        } else {
            return Err(StdError::generic_err("not a valid address passed"));
        }
    }
    let limit = limit.unwrap_or(10).min(30) as usize;
    let stakeable_tokens_result: StdResult<Vec<_>> = USER_REWARD_MAP
        .range(deps.storage, bound, None, Order::Ascending)
        .take(limit)
        .collect();
    let mut st: Vec<UserInfo> = vec![];
    if let Ok(stakeable_tokens) = stakeable_tokens_result {
        for i in stakeable_tokens {
            st.push(i.1);
        }
    }
    Ok(st)
}
pub fn query_user_assigned_reward(deps: Deps, recipient: String) -> StdResult<UserInfo> {
    USER_REWARD_MAP.load(deps.storage, recipient)
}

pub fn query_user_reward(deps: Deps, env: Env, recipient: String) -> StdResult<Uint128> {
    let user_info = USER_REWARD_MAP.may_load(deps.storage, recipient)?;
    if user_info.is_none() {
        return Err(StdError::generic_err("No User Found"));
    }
    let user_info = user_info.unwrap();
    let reward_time_precission = (env.block.time.seconds() - user_info.assigned_time) as u128;
    let total_reward_to_be_claimed: Uint128 =
        Uint128::new(reward_time_precission).multiply_ratio(user_info.reward, user_info.duration);

    Ok(total_reward_to_be_claimed)
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    // set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}
