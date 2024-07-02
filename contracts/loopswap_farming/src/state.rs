use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Uint128;
use cw_storage_plus::{Item, Map};
use loopswap::asset::StakeableToken;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub owner: String,
    pub freeze: bool,
    pub lock_time_frame: u64,
    pub wait_time_for_distribution_in_seconds: u64,
    pub second_owner: String,
    pub default_limit: u32,
    pub max_limit: u32,
    pub lock_time_frame_for_compound_reward: u64,
    pub reserve_addr: String,
    pub token_code_id: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct RewardInfo {
    pub reward_index: Uint128,
    pub pending_reward: Uint128,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const STAKEABLE_INFOS: Map<String, StakeableToken> = Map::new("StakeableInfos");
pub const UNCLAIMED_DISTRIBUTED_TOKEN_AMOUNT_MAP: Map<String, Uint128> =
    Map::new("unclaimedDistributedTokenAmountMap");
pub const USER_STAKED_AMOUNT: Map<String, Uint128> = Map::new("rewardTokenIssued");
pub const TOTAL_STAKED: Map<String, Uint128> = Map::new("totalStaked");
pub const TOTAL_REWARDS_IN_POOL: Map<String, Uint128> = Map::new("totalRewardsInPool");
pub const TOTAL_ACCUMULATED_DISTRIBUTED_AMOUNT_IN_POOL_MAP: Map<String, Uint128> =
    Map::new("totalAccumulatedDistributedTokenAmountMapInPools");
pub const POOL_REWARD_INDEX_MAP: Map<String, Uint128> = Map::new("rewardIndexMap");
pub const USER_REWARD_INFO_MAP: Map<String, RewardInfo> = Map::new("userRewardInfoxMap");
pub const USER_REWARD_STARTING_TIME_MAP: Map<String, u64> = Map::new("userRewardStartingTimeMap");
pub const POOL_LAST_DISTRIBUTION_TIME_IN_SECONDS: Map<String, u64> =
    Map::new("liquidityAndDevTokenMap");
pub const USER_AUTO_COMPOUND_SUBSCRIPTION_MAP: Map<String, bool> =
    Map::new("UserAutoCompoundSubscriptionMap");
pub const POOL_TOTAL_COMPOUNDED_AMOUNT: Map<String, Uint128> = Map::new("totalCompoundedStaked");
pub const POOL_COMPOUNDED_INDEX_MAP: Map<String, Uint128> = Map::new("CompoundedIndexMap");
pub const USER_COMPOUNDED_REWARD_INFO_MAP: Map<String, RewardInfo> =
    Map::new("userCompoundedInfoxMap");
pub const CURRENT_POOL_ADDRESS: Item<String> = Item::new("CurrentPoolAddress");
pub const LIQUIDITY_TOKEN_MAP: Map<String, String> = Map::new("LiquidityTokenMap");
pub const LAST_CLAIMED_REWARD_TIME: Map<String, u64> = Map::new("LastClaimedRewardTime");