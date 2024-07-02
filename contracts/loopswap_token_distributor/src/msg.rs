use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    pub token_contract_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    UserAssignedReward {
        recipient: String,
    },
    UserReward {
        recipient: String,
    },
    UsersReward {
        start_after: Option<String>,
        limit: Option<u64>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AssignReward {
        recipient: String,
        reward: Uint128,
        duration: u64,
    },
    Claim {},
    UpdateRewardDuration {
        recipient: String,
        reward: Option<Uint128>,
        duration: Option<u64>,
    },
    UpdateConfig {
        token_contract_address: Option<String>,
        admin: Option<String>,
    },
}
