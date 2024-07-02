#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Empty, Env, MessageInfo, Reply, ReplyOn,
    Response, StdError, StdResult, SubMsg, WasmMsg, Uint128,
};
use loopswap::querier::query_pair_info_from_pair;

use crate::response::MsgInstantiateContractResponse;
use crate::state::{pair_key, read_pairs, Config, TmpPairInfo, CONFIG, PAIRS, TMP_PAIR_INFO};

use loopswap::asset::{AssetInfo, PairInfo, PairInfoRaw};
use loopswap::factory::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, PairsResponse, QueryMsg,
};
use loopswap::pair::{InstantiateMsg as PairInstantiateMsg, ExecuteMsg as PairExecuteMsg};
use protobuf::Message;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<Empty>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: deps.api.addr_canonicalize(info.sender.as_str())?,
        token_code_id: msg.token_code_id,
        pair_code_id: msg.pair_code_id,
        admin: msg.admin,
        extra_commission_contract_addr: msg.extra_commission_contract_addr.unwrap_or_default(),
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<Empty>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::UpdateConfig {
            owner,
            token_code_id,
            pair_code_id,
            admin,
        } => execute_update_config(deps, env, info, owner, token_code_id, pair_code_id, admin),
        ExecuteMsg::CreatePair {
            asset_infos,
            is_stable_pair,
        } => execute_create_pair(deps, env, info, asset_infos, is_stable_pair),
    }
}

// Only owner can execute it
pub fn execute_update_config(
    deps: DepsMut<Empty>,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
    token_code_id: Option<u64>,
    pair_code_id: Option<u64>,
    admin: Option<String>,
) -> StdResult<Response> {
    let mut config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    if let Some(owner) = owner {
        // validate address format
        let _ = deps.api.addr_validate(&owner)?;

        config.owner = deps.api.addr_canonicalize(&owner)?;
    }

    if let Some(token_code_id) = token_code_id {
        config.token_code_id = token_code_id;
    }

    if let Some(pair_code_id) = pair_code_id {
        config.pair_code_id = pair_code_id;
    }

    if let Some(admin) = admin {
        let _ = deps.api.addr_validate(&admin)?;
        config.admin = admin;
    }

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

// Anyone can execute it to create swap pair
pub fn execute_create_pair(
    deps: DepsMut<Empty>,
    env: Env,
    _info: MessageInfo,
    asset_infos: [AssetInfo; 2],
    is_stable_pair: bool,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    if asset_infos[0] == asset_infos[1] {
        return Err(StdError::generic_err("same asset"));
    }

    let asset_1_decimal =
        match asset_infos[0].query_decimals(env.contract.address.clone(), &deps.querier) {
            Ok(decimal) => decimal,
            Err(_) => return Err(StdError::generic_err("asset1 is invalid")),
        };

    let asset_2_decimal = match asset_infos[1].query_decimals(env.contract.address, &deps.querier) {
        Ok(decimal) => decimal,
        Err(_) => return Err(StdError::generic_err("asset2 is invalid")),
    };

    let raw_infos = [
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ];

    let asset_decimals = [asset_1_decimal, asset_2_decimal];

    let pair_key = pair_key(&raw_infos);
    if let Ok(Some(_)) = PAIRS.may_load(deps.storage, &pair_key) {
        return Err(StdError::generic_err("Pair already exists"));
    }

    TMP_PAIR_INFO.save(
        deps.storage,
        &TmpPairInfo {
            pair_key,
            asset_infos: raw_infos,
            is_stable_pair,
            asset_decimals,
        },
    )?;
    let extra_commission_contract_addr = if config.extra_commission_contract_addr.is_empty() {
        _info.sender.to_string()
    } else {
        config.extra_commission_contract_addr
    };

    
    Ok(Response::new()
        .add_attributes(vec![
            ("action", "create_pair"),
            ("pair", &format!("{}-{}", asset_infos[0], asset_infos[1])),
        ])
        .add_submessage(SubMsg {
            id: 1,
            gas_limit: None,
            msg: CosmosMsg::Wasm(WasmMsg::Instantiate {
                code_id: config.pair_code_id,
                funds: vec![],
                admin: Some(config.admin.to_string()),
                label: "pair".to_string(),
                msg: to_binary(&PairInstantiateMsg {
                    asset_infos,
                    token_code_id: config.token_code_id,
                    is_stable_pair,
                    asset_decimals,
                    extra_commission_contract_addr,
                    admin: config.admin,
                })?,
            }),
            reply_on: ReplyOn::Success,
        }))
}

pub fn update_extra_commission_info(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_contract_addr: Option<String>,
    new_fee_allocation: Option<Uint128>,
    pair_contract_address: String,
) -> StdResult<Response> {
    let config: Config = CONFIG.load(deps.storage)?;

    // permission check
    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    let callback: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute { 
        contract_addr: pair_contract_address,
        msg: to_binary(&PairExecuteMsg::UpdateExtraCommissionInfo { new_contract_addr, new_fee_allocation })?, 
        funds: vec![],
    });

    Ok(Response::new().add_message(callback))

}

/// This just stores the result for future query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut<Empty>, _env: Env, msg: Reply) -> StdResult<Response> {
    let tmp_pair_info = TMP_PAIR_INFO.load(deps.storage)?;

    let res: MsgInstantiateContractResponse =
        Message::parse_from_bytes(msg.result.unwrap().data.unwrap().as_slice()).map_err(|_| {
            StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
        })?;

    let pair_contract = res.get_address();
    let pair_info = query_pair_info_from_pair(&deps.querier, Addr::unchecked(pair_contract))?;

    PAIRS.save(
        deps.storage,
        &tmp_pair_info.pair_key,
        &PairInfoRaw {
            liquidity_token: deps.api.addr_canonicalize(&pair_info.liquidity_token)?,
            contract_addr: deps.api.addr_canonicalize(pair_contract)?,
            asset_infos: tmp_pair_info.asset_infos,
            asset_decimals: tmp_pair_info.asset_decimals,
        },
    )?;

    Ok(Response::new().add_attributes(vec![
        ("pair_contract_addr", pair_contract),
        ("liquidity_token_addr", pair_info.liquidity_token.as_str()),
    ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<Empty>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Pair { asset_infos } => to_binary(&query_pair(deps, asset_infos)?),
        QueryMsg::Pairs { start_after, limit } => {
            to_binary(&query_pairs(deps, start_after, limit)?)
        }
    }
}

pub fn query_config(deps: Deps<Empty>) -> StdResult<ConfigResponse> {
    let state: Config = CONFIG.load(deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.addr_humanize(&state.owner)?.to_string(),
        token_code_id: state.token_code_id,
        pair_code_id: state.pair_code_id,
        admin: state.admin,
    };

    Ok(resp)
}

pub fn query_pair(deps: Deps<Empty>, asset_infos: [AssetInfo; 2]) -> StdResult<PairInfo> {
    let pair_key = pair_key(&[
        asset_infos[0].to_raw(deps.api)?,
        asset_infos[1].to_raw(deps.api)?,
    ]);
    let pair_info: PairInfoRaw = PAIRS.load(deps.storage, &pair_key)?;
    pair_info.to_normal(deps.api)
}

pub fn query_pairs(
    deps: Deps<Empty>,
    start_after: Option<[AssetInfo; 2]>,
    limit: Option<u32>,
) -> StdResult<PairsResponse> {
    let start_after = if let Some(start_after) = start_after {
        Some([
            start_after[0].to_raw(deps.api)?,
            start_after[1].to_raw(deps.api)?,
        ])
    } else {
        None
    };

    let pairs: Vec<PairInfo> = read_pairs(deps.storage, deps.api, start_after, limit)?;
    let resp = PairsResponse { pairs };

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
