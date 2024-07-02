use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use loopswap::asset::PairInfoRaw;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PAIR_INFO: Item<PairInfoRaw> = Item::new("pair_info");
pub const FACTORY_CONTRACT_ADDR: Item<Addr> = Item::new("factory_contract_addr");
pub const EXTRA_COMMISSION_INFO: Item<ExtraCommissionInfo> = Item::new("extra_commission_info");
pub const EXTRA_COMMISSION_FEE: Item<ExtraCommissionFee> = Item::new("extra_commission_fee");
pub const CONFIG: Item<Config> = Item::new("Config");
#[derive(Serialize, Deserialize, Clone, Debug,  PartialEq, Eq, JsonSchema)]
pub struct ExtraCommissionFee {
    pub amount0: Uint128,
    pub amount1: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug,  PartialEq, Eq, JsonSchema)]
pub struct ExtraCommissionInfo {
    pub contract_addr: Addr,
    pub fee_allocation: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub admin: String,
    pub commission_rate: String,
}
