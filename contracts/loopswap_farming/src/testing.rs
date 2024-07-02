use crate::contract::{execute, instantiate, query, query_stakeable_info, reply};
use loopswap::mock_querier::mock_dependencies;

use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{from_binary, ContractResult, Env, Timestamp, Uint128};
use cosmwasm_std::{
    to_binary, Coin, CosmosMsg, Reply, StdError, SubMsg, SubMsgExecutionResponse, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};

use loopswap::asset::{StakeablePairedDistributionTokenInfo, StakeableToken};
use loopswap::farming::{Cw20HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};

fn mock_env_time(time: Timestamp) -> Env {
    let mut env = mock_env();
    env.block.time = time;
    env
}

mod tests {
    use super::*;

    #[test]
    fn test_initialize() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        println!("{:?}", "Initializing contract ok")
    }

    #[test]
    fn test_update_config() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);

        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let update_config_msg = ExecuteMsg::UpdateConfig {
            owner: Some("another_owner".to_string()),
        };

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"loop_staker2".to_string(), &[]),
            update_config_msg.clone(),
        );
        match res {
            Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
            _ => panic!("Invalid error"),
        }

        execute(deps.as_mut(), mock_env(), info, update_config_msg).unwrap();
    }

    #[test]
    fn test_update_lock_time_frame() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);

        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let update_config_msg = ExecuteMsg::UpdateLockTimeFrame {
            lock_time_frame: 8400,
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            update_config_msg.clone(),
        )
        .unwrap();

        let user1_staked: u64 = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::QueryLockTimeFrame {}).unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, 8400u64);
    }

    #[test]
    fn test_update_lock_time_frame_for_auto_compounding() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);

        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let update_config_msg = ExecuteMsg::UpdateLockTimeFrameForCompundReward {
            lock_time_frame_for_compound_reward: 8400,
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            update_config_msg.clone(),
        )
        .unwrap();

        let user1_staked: u64 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryLockTimeFrameForAutoCompound {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, 8400u64);
    }

    #[test]
    fn test_update_wait_time_for_distribution() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);

        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let update_config_msg = ExecuteMsg::UpdateWaitTimeForDistribution {
            wait_time_for_distribution_in_seconds: 8400,
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            update_config_msg.clone(),
        )
        .unwrap();

        let user1_staked: u64 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryDistributionWaitTime {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, 8400u64);
    }

    #[test]
    fn test_update_stakeable_token_address() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);

        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            stakeable_tokens.get(0).unwrap(),
            &StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }
        );

        let add_stakeable_token_msg = ExecuteMsg::UpdateStakeableTokenAddress {
            old_address: "asset0000".to_string(),
            new_address: "asset0001".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            stakeable_tokens.get(0).unwrap(),
            &StakeableToken {
                token: "asset0001".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }
        );
    }

    #[test]
    fn test_delete_stakeable_token() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);

        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            stakeable_tokens.get(0).unwrap(),
            &StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }
        );

        let add_stakeable_token_msg = ExecuteMsg::DeleteStakeableToken {
            address: "asset0000".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(stakeable_tokens, vec![]);
    }

    #[test]
    fn test_add_second_owner() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);

        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddSecondOwner {
            second_owner_address: "second owner".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let admin: String = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QuerySecondAdminAddress {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(admin, "second owner".to_string());
    }

    #[test]
    fn test_delete_distributeable_token() {
        let mut deps = mock_dependencies(&[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }]);

        deps.querier.with_token_balances(&[
            (
                &"liquidity0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
            ),
            (&"asset0000".to_string(), &[]),
        ]);

        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();
        let mut rewards = vec![];
        rewards.push(("rewards0000".to_string(), Uint128::from(200u128)));
        let update_reward = ExecuteMsg::UpdateReward {
            pool: "asset0000".to_string(),
            rewards: rewards.clone(),
        };

        let _res = execute(deps.as_mut(), mock_env(), info.clone(), update_reward).unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            stakeable_tokens.get(0).unwrap(),
            &StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![StakeablePairedDistributionTokenInfo {
                    token: "rewards0000".to_string(),
                    amount: Uint128::from(200u128),
                    reserve_amount: Uint128::zero(),
                }],
                liquidity_token: "".to_string(),
            }
        );

        let delete_distributeable_token = ExecuteMsg::DeleteDistributeableToken {
            pool_address: "asset0000".to_string(),
            token_address: "rewards0000".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            delete_distributeable_token,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            stakeable_tokens.get(0).unwrap(),
            &StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }
        );
    }

    //below is the test of complete staking function including transferfrom and minting new FLP tokens
    // to user as well as the "Add stakeable token" function
    #[test]
    fn test_stake_with_stakeable_token() {
        let mut deps = mock_dependencies(&[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }]);

        deps.querier.with_token_balances(&[
            (
                &"liquidity0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
            ),
            (&"asset0000".to_string(), &[]),
        ]);

        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0001".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            stakeable_tokens,
            vec![
                StakeableToken {
                    token: "asset0000".to_string(),
                    distribution: vec![],
                    liquidity_token: "".to_string(),
                },
                StakeableToken {
                    token: "asset0001".to_string(),
                    distribution: vec![],
                    liquidity_token: "".to_string(),
                },
            ]
        );
        // testing of stakeable token completed

        //now testing staking
        let stake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0000".to_string(), &[]),
            stake_msg,
        )
        .unwrap();

        let stake_msg2 = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker2".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0000".to_string(), &[]),
            stake_msg2,
        )
        .unwrap();

        let user1_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, Uint128::from(100u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(200u128));
    }

    #[test]
    #[should_panic]
    fn test_stake_with_unstakeable_token() {
        let mut deps = mock_dependencies(&[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }]);

        deps.querier.with_token_balances(&[
            (
                &"liquidity0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
            ),
            (&"asset0000".to_string(), &[]),
        ]);

        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            stakeable_tokens,
            vec![StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }]
        );
        // testing of stakeable token completed

        //now testing staking
        let stake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0001".to_string(), &[]),
            stake_msg,
        )
        .unwrap();
        let stake_msg2 = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker2".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0001".to_string(), &[]),
            stake_msg2,
        )
        .unwrap();

        let user1_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, Uint128::from(100u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(200u128));
    }

    //testing reward assignment and distribution of reward to the respective pool
    #[test]
    fn test_update_reward_and_distribution() {
        let mut deps = mock_dependencies(&[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }]);

        deps.querier.with_token_balances(&[
            (
                &"liquidity0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
            ),
            (&"asset0000".to_string(), &[]),
            (
                &"reward0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(10000u128))],
            ),
        ]);

        let env = mock_env();
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            stakeable_tokens,
            vec![StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }]
        );
        // testing of stakeable token completed

        //adding distribution token

        //now testing staking
        let stake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0000".to_string(), &[]),
            stake_msg,
        )
        .unwrap();

        let user1_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, Uint128::from(100u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(100u128));

        //Assigning Distribution reward
        let tempvec = vec![("reward0000".to_string(), Uint128::from(1000u128))];

        let update_reward_msg = ExecuteMsg::UpdateReward {
            pool: "asset0000".to_string(),
            rewards: tempvec,
        };

        execute(deps.as_mut(), mock_env(), info.clone(), update_reward_msg).unwrap();

        let new_env = mock_env_time(env.block.time.plus_seconds(86401));
        //Calling Distribution
        let distribute = ExecuteMsg::DistributeByLimit {
            start_after: None,
            limit: None,
        };
        execute(deps.as_mut(), new_env, info, distribute).unwrap();

        let reward_assigned: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryRewardInPool {
                    pool: "asset0000".to_string(),
                    distribution_token: "reward0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reward_assigned, Uint128::from(1000u128));
    }

    #[test]
    fn test_unstake_and_claim() {
        let mut deps = mock_dependencies(&[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }]);

        deps.querier.with_token_balances(&[
            (
                &"liquidity0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
            ),
            (&"asset0000".to_string(), &[]),
            (
                &"reward0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(10000u128))],
            ),
        ]);
        let env = mock_env();
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();
        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            stakeable_tokens,
            vec![StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }]
        );
        // testing of stakeable token completed
        let reply_msg = Reply {
            id: 1,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                data: Some(
                    vec![
                        10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                    ]
                    .into(),
                ),
            }),
        };

        let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
        let stakeable_info: Vec<StakeableToken> =
            query_stakeable_info(deps.as_ref(), None, None).unwrap();
        assert_eq!("liquidity0000", stakeable_info[0].liquidity_token.as_str());
        //now testing staking
        let stake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0000".to_string(), &[]),
            stake_msg,
        )
        .unwrap();

        let user1_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, Uint128::from(100u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(100u128));

        //Assigning Distribution reward
        let tempvec = vec![("reward0000".to_string(), Uint128::from(1000u128))];

        let update_reward_msg = ExecuteMsg::UpdateReward {
            pool: "asset0000".to_string(),
            rewards: tempvec,
        };

        execute(deps.as_mut(), mock_env(), info.clone(), update_reward_msg).unwrap();

        let new_env = mock_env_time(env.block.time.plus_seconds(86401));
        //Calling Distribution
        let distribute = ExecuteMsg::DistributeByLimit {
            start_after: None,
            limit: None,
        };
        execute(deps.as_mut(), new_env, info.clone(), distribute).unwrap();

        let reward_assigned: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryRewardInPool {
                    pool: "asset0000".to_string(),
                    distribution_token: "reward0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reward_assigned, Uint128::from(1000u128));

        //now testing unstaking and claiming
        // let unstake_msg = ExecuteMsg::UnstakeAndClaim {
        //     pool_address: "asset0000".to_string(),
        // };
        let unstake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::UnstakeAndClaim {}).unwrap(),
        });
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"liquidity0000".to_string(), &[]),
            unstake_msg,
        )
        .unwrap();
        let msg_burn_liquidity = res.messages.get(0).expect("no message");
        // let msg_receiving_flp = res.messages.get(1).expect("no message");
        let msg_refund_staked = res.messages.get(1).expect("no message");
        let msg_give_reward = res.messages.get(2).expect("no message");
        assert_eq!(
            msg_burn_liquidity,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "liquidity0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );
        assert_eq!(
            msg_refund_staked,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );

        assert_eq!(
            msg_give_reward,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "reward0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: Uint128::from(1000u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );

        let user_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(user_staked, Uint128::from(0u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(0u128));

        let reward_left_in_pool: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryRewardInPool {
                    pool: "asset0000".to_string(),
                    distribution_token: "reward0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reward_left_in_pool, Uint128::from(0u128));
    }

    #[test]
    fn test_unstake_without_claim() {
        let mut deps = mock_dependencies(&[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }]);

        deps.querier.with_token_balances(&[
            (
                &"liquidity0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
            ),
            (&"asset0000".to_string(), &[]),
            (
                &"reward0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(10000u128))],
            ),
        ]);
        let env = mock_env();
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();
        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            stakeable_tokens,
            vec![StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }]
        );

        let reply_msg = Reply {
            id: 1,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                data: Some(
                    vec![
                        10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                    ]
                    .into(),
                ),
            }),
        };

        let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
        let stakeable_info: Vec<StakeableToken> =
            query_stakeable_info(deps.as_ref(), None, None).unwrap();
        assert_eq!("liquidity0000", stakeable_info[0].liquidity_token.as_str());

        // testing of stakeable token completed

        //now testing staking
        let stake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0000".to_string(), &[]),
            stake_msg,
        )
        .unwrap();

        let user1_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, Uint128::from(100u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(100u128));

        //Assigning Distribution reward
        let tempvec = vec![("reward0000".to_string(), Uint128::from(1000u128))];

        let update_reward_msg = ExecuteMsg::UpdateReward {
            pool: "asset0000".to_string(),
            rewards: tempvec,
        };

        execute(deps.as_mut(), mock_env(), info.clone(), update_reward_msg).unwrap();

        let new_env = mock_env_time(env.block.time.plus_seconds(86401));
        //Calling Distribution
        let distribute = ExecuteMsg::DistributeByLimit {
            start_after: None,
            limit: None,
        };
        execute(deps.as_mut(), new_env, info.clone(), distribute).unwrap();

        let reward_assigned: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryRewardInPool {
                    pool: "asset0000".to_string(),
                    distribution_token: "reward0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reward_assigned, Uint128::from(1000u128));

        //now testing unstaking and claiming

        let unstake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::UnstakeAndClaim {}).unwrap(),
        });
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"liquidity0000".to_string(), &[]),
            unstake_msg,
        )
        .unwrap();
        let msg_burn_liquidity = res.messages.get(0).expect("no message");
        // let msg_receiving_flp = res.messages.get(1).expect("no message");
        let msg_refund_staked = res.messages.get(1).expect("no message");
        let msg_give_reward = res.messages.get(2).expect("no message");
        assert_eq!(
            msg_burn_liquidity,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "liquidity0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );
        assert_eq!(
            msg_refund_staked,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );

        assert_eq!(
            msg_give_reward,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "reward0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: Uint128::from(1000u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );
        let user_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(user_staked, Uint128::from(0u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(0u128));

        let reward_left_in_pool: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryRewardInPool {
                    pool: "asset0000".to_string(),
                    distribution_token: "reward0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reward_left_in_pool, Uint128::from(0u128));
    }

    #[test]
    fn test_freeze_flag() {
        let mut deps = mock_dependencies(&[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }]);

        deps.querier.with_token_balances(&[
            (
                &"liquidity0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
            ),
            (&"asset0000".to_string(), &[]),
            (
                &"reward0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(10000u128))],
            ),
        ]);
        let env = mock_env();
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();
        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            stakeable_tokens,
            vec![StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }]
        );

        let reply_msg = Reply {
            id: 1,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                data: Some(
                    vec![
                        10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                    ]
                    .into(),
                ),
            }),
        };

        let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
        let stakeable_info: Vec<StakeableToken> =
            query_stakeable_info(deps.as_ref(), None, None).unwrap();
        assert_eq!("liquidity0000", stakeable_info[0].liquidity_token.as_str());

        // testing of stakeable token completed

        //now testing staking
        let stake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0000".to_string(), &[]),
            stake_msg,
        )
        .unwrap();

        let stake_msg = ExecuteMsg::UpdateFreezeFlag {
            freeze_flag: "Y".to_string(),
        };

        execute(deps.as_mut(), mock_env(), info.clone(), stake_msg).unwrap();

        let user1_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, Uint128::from(100u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(100u128));

        //Assigning Distribution reward
        let tempvec = vec![("reward0000".to_string(), Uint128::from(1000u128))];

        let update_reward_msg = ExecuteMsg::UpdateReward {
            pool: "asset0000".to_string(),
            rewards: tempvec,
        };

        execute(deps.as_mut(), mock_env(), info.clone(), update_reward_msg).unwrap();

        let new_env = mock_env_time(env.block.time.plus_seconds(86401));
        //Calling Distribution
        let distribute = ExecuteMsg::DistributeByLimit {
            start_after: None,
            limit: None,
        };
        execute(deps.as_mut(), new_env, info.clone(), distribute).unwrap();

        let reward_assigned: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryRewardInPool {
                    pool: "asset0000".to_string(),
                    distribution_token: "reward0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reward_assigned, Uint128::from(1000u128));

        //now testing unstaking and claiming

        let unstake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::UnstakeAndClaim {}).unwrap(),
        });
        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"liquidity0000".to_string(), &[]),
            unstake_msg,
        );
        match res {
            Err(StdError::GenericErr { msg, .. }) => assert_eq!(
                msg,
                "Sorry for inconvenience, system is under maintenance. Kindly check again later"
            ),
            _ => panic!("Invalid error"),
        }

        let user_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(user_staked, Uint128::from(100u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(100u128));

        let reward_left_in_pool: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryRewardInPool {
                    pool: "asset0000".to_string(),
                    distribution_token: "reward0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reward_left_in_pool, Uint128::from(1000u128));
    }

    #[test]
    fn test_claim_reward_only() {
        let mut deps = mock_dependencies(&[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }]);

        deps.querier.with_token_balances(&[
            (
                &"liquidity0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
            ),
            (&"asset0000".to_string(), &[]),
            (
                &"reward0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(10000u128))],
            ),
        ]);
        let env = mock_env();
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let info_2 = mock_info(&"loop_staker2".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();
        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            stakeable_tokens,
            vec![StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }]
        );
        // testing of stakeable token completed

        //now testing staking
        let stake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0000".to_string(), &[]),
            stake_msg,
        )
        .unwrap();

        let stake_msg1 = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker2".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0000".to_string(), &[]),
            stake_msg1,
        )
        .unwrap();

        let user1_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, Uint128::from(100u128));

        let user2_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info_2.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user2_staked, Uint128::from(100u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(200u128));

        //Assigning Distribution reward
        let tempvec = vec![("reward0000".to_string(), Uint128::from(1000u128))];

        let update_reward_msg = ExecuteMsg::UpdateReward {
            pool: "asset0000".to_string(),
            rewards: tempvec,
        };

        execute(deps.as_mut(), mock_env(), info.clone(), update_reward_msg).unwrap();

        let new_env = mock_env_time(env.block.time.plus_seconds(86401));
        //Calling Distribution
        let distribute = ExecuteMsg::DistributeByLimit {
            start_after: None,
            limit: None,
        };
        execute(deps.as_mut(), new_env, info.clone(), distribute).unwrap();

        let reward_assigned: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryRewardInPool {
                    pool: "asset0000".to_string(),
                    distribution_token: "reward0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reward_assigned, Uint128::from(1000u128));

        //now testing unstaking and claiming
        let claim_reward = ExecuteMsg::ClaimReward {
            pool_address: "asset0000".to_string(),
        };

        let res = execute(deps.as_mut(), mock_env(), info.clone(), claim_reward).unwrap();

        let claim_reward2 = ExecuteMsg::ClaimReward {
            pool_address: "asset0000".to_string(),
        };

        let res2 = execute(deps.as_mut(), mock_env(), info_2.clone(), claim_reward2).unwrap();
        //let msg_refund_staked = res.messages.get(0).expect("no message");
        // let msg_receiving_flp = res.messages.get(1).expect("no message");
        // let msg_burn_liquidity = res.messages.get(1).expect("no message");
        let msg_give_reward = res.messages.get(0).expect("no message");

        assert_eq!(
            msg_give_reward,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "reward0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: Uint128::from(500u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );

        let msg_give_reward2 = res2.messages.get(0).expect("no message");
        //println!("lullu {:?}", from_binary(&msg_give_reward.msg.));

        assert_eq!(
            msg_give_reward2,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "reward0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info_2.sender.to_string(),
                    amount: Uint128::from(500u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );

        let user_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(user_staked, Uint128::from(100u128));

        let user2_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info_2.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(user2_staked, Uint128::from(100u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(200u128));

        let reward_left_in_pool: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryRewardInPool {
                    pool: "asset0000".to_string(),
                    distribution_token: "reward0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reward_left_in_pool, Uint128::from(0u128));
    }

    #[test]
    fn test_distribute_by_limit() {
        let mut deps = mock_dependencies(&[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }]);

        deps.querier.with_token_balances(&[
            (
                &"liquidity0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
            ),
            (&"asset0000".to_string(), &[]),
            (
                &"reward0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(10000u128))],
            ),
        ]);
        let env = mock_env();
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);

        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();
        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            stakeable_tokens,
            vec![StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }]
        );
        // testing of stakeable token completed

        //now testing staking
        let stake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0000".to_string(), &[]),
            stake_msg,
        )
        .unwrap();

        let user1_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, Uint128::from(100u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(100u128));

        //Assigning Distribution reward
        let tempvec = vec![("reward0000".to_string(), Uint128::from(1000u128))];

        let update_reward_msg = ExecuteMsg::UpdateReward {
            pool: "asset0000".to_string(),
            rewards: tempvec,
        };

        execute(deps.as_mut(), mock_env(), info.clone(), update_reward_msg).unwrap();

        let new_env = mock_env_time(env.block.time.plus_seconds(86401));
        //Calling Distribution
        let distribute = ExecuteMsg::DistributeByLimit {
            start_after: None,
            limit: Some(5),
        };
        execute(deps.as_mut(), new_env, info, distribute).unwrap();

        let reward_assigned: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryRewardInPool {
                    pool: "asset0000".to_string(),
                    distribution_token: "reward0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reward_assigned, Uint128::from(1000u128));
    }

    #[test]
    fn test_claim_compounding_reward() {
        let mut deps = mock_dependencies(&[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }]);

        deps.querier.with_token_balances(&[
            (
                &"liquidity0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(100u128))],
            ),
            (&"asset0000".to_string(), &[]),
            (
                &"reward0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::from(10000u128))],
            ),
        ]);
        let env = mock_env();
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let info_2 = mock_info(&"loop_staker2".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();
        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            stakeable_tokens,
            vec![StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }]
        );

        let reply_msg = Reply {
            id: 1,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                data: Some(
                    vec![
                        10, 13, 108, 105, 113, 117, 105, 100, 105, 116, 121, 48, 48, 48, 48,
                    ]
                    .into(),
                ),
            }),
        };

        let _res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
        let stakeable_info: Vec<StakeableToken> =
            query_stakeable_info(deps.as_ref(), None, None).unwrap();
        assert_eq!("liquidity0000", stakeable_info[0].liquidity_token.as_str());

        // testing of stakeable token completed

        //now testing staking
        let stake_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0000".to_string(), &[]),
            stake_msg,
        )
        .unwrap();

        let stake_msg1 = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker2".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"asset0000".to_string(), &[]),
            stake_msg1,
        )
        .unwrap();

        let user1_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, Uint128::from(100u128));

        let user2_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info_2.sender.to_string(),
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user2_staked, Uint128::from(100u128));

        let total_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryTotalStaked {
                    staked_token: "asset0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(200u128));

        //Assigning Distribution reward

        let tempvec = vec![("reward0000".to_string(), Uint128::from(1000u128))];

        let update_reward_msg = ExecuteMsg::UpdateReward {
            pool: "asset0000".to_string(),
            rewards: tempvec,
        };

        execute(deps.as_mut(), mock_env(), info.clone(), update_reward_msg).unwrap();

        let new_env = mock_env_time(env.block.time.plus_seconds(86401));
        //Calling Distribution
        let distribute = ExecuteMsg::DistributeByLimit {
            start_after: None,
            limit: None,
        };
        execute(deps.as_mut(), new_env, info.clone(), distribute).unwrap();

        let reward_assigned: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryRewardInPool {
                    pool: "asset0000".to_string(),
                    distribution_token: "reward0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reward_assigned, Uint128::from(1000u128));

        let opt_for_auto_compounding = ExecuteMsg::OptForAutoCompound {
            pool_address: "asset0000".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info_2.clone(),
            opt_for_auto_compounding,
        )
        .unwrap();

        //now testing unstaking and claiming
        let claim_reward = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::UnstakeWithoutClaim {}).unwrap(),
        });

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"liquidity0000".to_string(), &[]),
            claim_reward,
        )
        .unwrap();

        let claim_reward2 = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker2".to_string(),
            amount: Uint128::from(100u128),
            msg: to_binary(&Cw20HookMsg::UnstakeWithoutClaim {}).unwrap(),
        });

        let res2 = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(&"liquidity0000".to_string(), &[]),
            claim_reward2,
        )
        .unwrap();
        let msg_burn = res.messages.get(0).expect("no message");
        let msg_transfer = res.messages.get(1).expect("no message");
        //let msg_give_reward = res.messages.get(1).expect("no message");
        assert_eq!(
            msg_burn,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "liquidity0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );

        assert_eq!(
            msg_transfer,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );
        let msg_burn = res.messages.get(0).expect("no message");
        let msg_unstake = res.messages.get(1).expect("no message");
        let msg_give_reward2 = res2.messages.get(2).expect("no message");
        assert_eq!(
            msg_burn,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "liquidity0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Burn {
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );

        assert_eq!(
            msg_unstake,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "asset0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: info.sender.to_string(),
                    amount: Uint128::from(100u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );

        assert_eq!(
            msg_give_reward2,
            &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "reward0000".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: "reserve addr".to_string(),
                    amount: Uint128::from(1000u128),
                })
                .unwrap(),
                funds: vec![],
            }))
        );

        let reward_left_in_pool: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryRewardInPool {
                    pool: "asset0000".to_string(),
                    distribution_token: "reward0000".to_string(),
                },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(reward_left_in_pool, Uint128::from(0u128));
    }

    #[test]
    fn test_second_owner_lock_time_frame() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let info2 = mock_info(&"second owner".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddSecondOwner {
            second_owner_address: "second owner".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let admin: String = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QuerySecondAdminAddress {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(admin, "second owner".to_string());

        let update_config_msg = ExecuteMsg::UpdateLockTimeFrame {
            lock_time_frame: 8400,
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info2.clone(),
            update_config_msg.clone(),
        )
        .unwrap();

        let user1_staked: u64 = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::QueryLockTimeFrame {}).unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, 8400u64);
    }

    #[test]
    fn test_second_owner_add_stakeable_token() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let info2 = mock_info(&"second owner".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddSecondOwner {
            second_owner_address: "second owner".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let admin: String = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QuerySecondAdminAddress {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(admin, "second owner".to_string());

        let update_config_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info2.clone(),
            update_config_msg.clone(),
        )
        .unwrap();

        let user1_staked: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            user1_staked.get(0).unwrap(),
            &StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }
        );
    }

    #[test]
    fn test_update_config_with_second_owner() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let info2 = mock_info(&"second owner".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddSecondOwner {
            second_owner_address: "second owner".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let admin: String = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QuerySecondAdminAddress {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(admin, "second owner".to_string());

        let update_config_msg = ExecuteMsg::UpdateConfig {
            owner: Some("another_owner".to_string()),
        };

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info2.clone(),
            update_config_msg.clone(),
        );
        match res {
            Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
            _ => panic!("Invalid error"),
        }

        execute(deps.as_mut(), mock_env(), info, update_config_msg).unwrap();
    }

    #[test]
    fn test_update_lock_time_frame_for_auto_compounding_with_second_owner() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let info2 = mock_info(&"second owner".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddSecondOwner {
            second_owner_address: "second owner".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let admin: String = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QuerySecondAdminAddress {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(admin, "second owner".to_string());

        let update_config_msg = ExecuteMsg::UpdateLockTimeFrameForCompundReward {
            lock_time_frame_for_compound_reward: 8400,
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info2.clone(),
            update_config_msg.clone(),
        )
        .unwrap();

        let user1_staked: u64 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryLockTimeFrameForAutoCompound {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, 8400u64);
    }

    #[test]
    fn test_update_wait_time_for_distribution_with_seecond_owner() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let info2 = mock_info(&"second owner".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddSecondOwner {
            second_owner_address: "second owner".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let admin: String = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QuerySecondAdminAddress {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(admin, "second owner".to_string());

        let update_config_msg = ExecuteMsg::UpdateWaitTimeForDistribution {
            wait_time_for_distribution_in_seconds: 8400,
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info2.clone(),
            update_config_msg.clone(),
        )
        .unwrap();

        let user1_staked: u64 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryDistributionWaitTime {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(user1_staked, 8400u64);
    }

    #[test]
    fn test_update_stakeable_token_address_with_second_owner() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let info2 = mock_info(&"second owner".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddSecondOwner {
            second_owner_address: "second owner".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let admin: String = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QuerySecondAdminAddress {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(admin, "second owner".to_string());

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info2.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            stakeable_tokens.get(0).unwrap(),
            &StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }
        );

        let add_stakeable_token_msg = ExecuteMsg::UpdateStakeableTokenAddress {
            old_address: "asset0000".to_string(),
            new_address: "asset0001".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info2.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            stakeable_tokens.get(0).unwrap(),
            &StakeableToken {
                token: "asset0001".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }
        );
    }

    #[test]
    fn test_delete_stakeable_token_with_second_owner() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let info2 = mock_info(&"second owner".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddSecondOwner {
            second_owner_address: "second owner".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let admin: String = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QuerySecondAdminAddress {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(admin, "second owner".to_string());

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info2.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            stakeable_tokens.get(0).unwrap(),
            &StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }
        );

        let add_stakeable_token_msg = ExecuteMsg::DeleteStakeableToken {
            address: "asset0000".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info2.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(stakeable_tokens, vec![]);
    }

    #[test]
    fn test_delete_distributeable_token_with_second_owner() {
        let mut deps = mock_dependencies(&[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::from(200u128),
        }]);

        deps.querier.with_token_balances(&[
            (
                &"liquidity0000".to_string(),
                &[(&MOCK_CONTRACT_ADDR.to_string(), &Uint128::zero())],
            ),
            (&"asset0000".to_string(), &[]),
        ]);

        let init_msg = InstantiateMsg {
            reserve_addr: "reserve addr".to_string(),
            token_code_id: 6,
        };
        let info = mock_info(&"loop_staker1".to_string(), &[]);
        let info2 = mock_info(&"second owner".to_string(), &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let add_stakeable_token_msg = ExecuteMsg::AddSecondOwner {
            second_owner_address: "second owner".to_string(),
        };

        execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();

        let admin: String = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QuerySecondAdminAddress {},
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(admin, "second owner".to_string());

        let add_stakeable_token_msg = ExecuteMsg::AddStakeableToken {
            token: "asset0000".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info2.clone(),
            add_stakeable_token_msg,
        )
        .unwrap();
        let mut rewards = vec![];
        rewards.push(("rewards0000".to_string(), Uint128::from(200u128)));
        let update_reward = ExecuteMsg::UpdateReward {
            pool: "asset0000".to_string(),
            rewards: rewards.clone(),
        };

        let _res = execute(deps.as_mut(), mock_env(), info2.clone(), update_reward).unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            stakeable_tokens.get(0).unwrap(),
            &StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![StakeablePairedDistributionTokenInfo {
                    token: "rewards0000".to_string(),
                    amount: Uint128::from(200u128),
                    reserve_amount: Uint128::zero(),
                }],
                liquidity_token: "".to_string(),
            }
        );

        let delete_distributeable_token = ExecuteMsg::DeleteDistributeableToken {
            pool_address: "asset0000".to_string(),
            token_address: "rewards0000".to_string(),
        };

        let _res = execute(
            deps.as_mut(),
            mock_env(),
            info2.clone(),
            delete_distributeable_token,
        )
        .unwrap();

        let stakeable_tokens: Vec<StakeableToken> = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryListOfStakeableTokens {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(
            stakeable_tokens.get(0).unwrap(),
            &StakeableToken {
                token: "asset0000".to_string(),
                distribution: vec![],
                liquidity_token: "".to_string(),
            }
        );
    }
}
