#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128};

use crate::msg::{ExecuteMsg};

use crate::msg::{ InstantiateMsg, MigrateMsg };


// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    
    Ok(Response::default())
}


//Execute the handle messages.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateReward { pool, rewards } => {
            execute_update_reward(deps, env, info, pool, rewards)
        }
        
    }
}



pub fn execute_update_reward(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    pool: String,
    rewards: Vec<(String, Uint128)>,
) -> StdResult<Response> {
    

    Ok(Response::new().add_attribute("action", "reward assigned"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {

    Ok(Response::default())
}
