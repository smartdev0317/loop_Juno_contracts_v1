// use crate::contract::{execute_mint, instantiate, query, query_balance};
use crate::contract::{instantiate, query,execute};

use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{from_binary, Env, Timestamp, Uint128};
use cosmwasm_std::{to_binary, CosmosMsg, StdError, SubMsg, WasmMsg};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, Cw20Coin,MinterResponse, BalanceResponse};
use cw20_base::msg::{QueryMsg, ExecuteMsg};
use loopswap::mock_querier::mock_dependencies;
// use crate::msg::{InstantiateMsg};
use loopswap::token::{InstantiateMsg};
fn mock_env_time(time: Timestamp) -> Env {
    let mut env = mock_env();
    env.block.time = time;
    env
}

mod tests {

    use super::*;
    // #[test]
    fn test_initialize() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            name: "loop_token".to_string(),
            symbol: "LPT".to_string(),
            decimals: 6u8,
            initial_balances: vec![Cw20Coin {
                address: "vault_address".to_string(),
                amount: Uint128::from(123456789u128),
            }],
            mint: None,
            // marketing: None,
        };
        let info = mock_info("juno1jx22pxvxdhpadzzjk0lcwcydgywwpyvhuw44jk", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        // println!("{:?}", _res)
    }

    // #[test]
    fn test_mint(){
        let mut deps = mock_dependencies(&[]);
        let recipient: String = "juno1jx22pxvxdhpadzzjk0lcwcydgywwpyvhuw44jk".to_string();
        let amount = Uint128::from(1000u128);

        let info = mock_info("juno1jx22pxvxdhpadzzjk0lcwcydgywwpyvhuw44jk", &[]);
        // let _res = execute_mint(deps.as_mut(), mock_env(), info, recipient, amount);
 
        // let _res = execute(deps.as_mut(), mock_env(), info, recipient);

    
        // println!("{:?}", _res)
    }

    #[test]
    fn test_query_balance(){

        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            name: "loop_token".to_string(),
            symbol: "LPT".to_string(),
            decimals: 6u8,
            initial_balances: vec![Cw20Coin {
                address: "juno1jx22pxvxdhpadzzjk0lcwcydgywwpyvhuw44jk".to_string(),
                amount: Uint128::from(5000u128),
            }],
            // mint: None,
            mint: Some(MinterResponse{
                minter: "juno1rlzh35nhhpjyqkuwtsrgh7drdf794qla2zjgpn".to_string(),
                cap: Some(Uint128::from(123456789u128)),
            }),
            // marketing: None,
        };
        let info = mock_info("juno1rlzh35nhhpjyqkuwtsrgh7drdf794qla2zjgpn", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

    
        // let mut deps = mock_dependencies(&[]);
        let recipient: String = "juno1jx22pxvxdhpadzzjk0lcwcydgywwpyvhuw44jk".to_string();
        let amount = Uint128::from(1000u128);

        let info = mock_info("juno1rlzh35nhhpjyqkuwtsrgh7drdf794qla2zjgpn", &[]);
        // let _res = execute(deps.as_mut(), mock_env(), info, recipient, amount);

        let distribute = ExecuteMsg::Mint {
            recipient,
            amount,
        };
        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            distribute.clone(),
        )
        .unwrap();

        println!("mint_response: {:?}", _res);


        // let mut deps = mock_dependencies(&[]);
        let address = "juno1jx22pxvxdhpadzzjk0lcwcydgywwpyvhuw44jk".to_string();
        // let balance = query(deps.as_ref(), mock_env(), address);

        let balance = QueryMsg::Balance { address };
        let balance: Uint128 =  from_binary(&query(deps.as_ref(), mock_env(), balance.clone()).unwrap()).unwrap();

        println!("Test_balance: {:?}", balance);
        // assert_eq!(BalanceResponse{balance}, BalanceResponse{balance});

    }
}
