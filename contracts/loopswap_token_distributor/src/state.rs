use cosmwasm_std::Uint128;

use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub token_contract_address: String,
    pub admin: String,
}
#[derive(Default, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserInfo {
    pub reward: Uint128,
    pub assigned_time: u64,
    pub duration: u64,
    pub address: String,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const USER_REWARD_MAP: Map<String, UserInfo> = Map::new("user_reward_map");
