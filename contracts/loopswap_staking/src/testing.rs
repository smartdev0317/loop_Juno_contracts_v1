use crate::contract::{execute, instantiate, query};
use crate::msg::{
    Cw20HookMsg, Cw20QueryMsg, ExecuteMsg, InstantiateMsg, QueryMsg, TokenInstantiateMsg,
};
use crate::state::UserRewardResponse;
use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{from_binary, Env, Timestamp, Uint128};
use cosmwasm_std::{to_binary, CosmosMsg, StdError, SubMsg, WasmMsg};
use cw20::{BalanceResponse, Cw20Coin, Cw20ReceiveMsg, MinterResponse};
use loopswap::mock_querier::mock_dependencies;

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
        let amount = Uint128::new(11223344);
        let minter = String::from("asmodat");
        let limit = Uint128::new(511223344);
        let init_msg = InstantiateMsg {
            token: "loop_token".to_string(),
            freeze_lock_time: 86400u64,
            lock_time_frame: 7776000u64,
            vault_address: "vault_address".to_string(),
            restake_reset_flag: false,
            token_instantiate_msg: TokenInstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![Cw20Coin {
                    address: "addr0000".into(),
                    amount,
                }],
                mint: Some(MinterResponse {
                    minter: minter.clone(),
                    cap: Some(limit),
                }),
                marketing: None,
            },
        };
        let info = mock_info("loop_staker1", &[]);
        let _res = instantiate(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        println!("{:?}", "Initializing contract ok")
    }

    #[test]
    fn test_update_config() {
        let mut deps = mock_dependencies(&[]);
        let amount = Uint128::new(11223344);
        let minter = String::from("asmodat");
        let limit = Uint128::new(511223344);
        let init_msg = InstantiateMsg {
            token: "loop_token".to_string(),
            freeze_lock_time: 86400u64,
            lock_time_frame: 7776000u64,
            vault_address: "vault_address".to_string(),
            restake_reset_flag: false,
            token_instantiate_msg: TokenInstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![Cw20Coin {
                    address: "addr0000".into(),
                    amount,
                }],
                mint: Some(MinterResponse {
                    minter: minter.clone(),
                    cap: Some(limit),
                }),
                marketing: None,
            },
        };
        let info = mock_info("loop_staker1", &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let update_config_msg = ExecuteMsg::UpdateConfig {
            owner: "another_owner".to_string(),
        };

        let res = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("loop_staker2", &[]),
            update_config_msg.clone(),
        );
        match res {
            Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "unauthorized"),
            _ => panic!("Invalid error"),
        }

        execute(deps.as_mut(), mock_env(), info, update_config_msg).unwrap();
    }

    #[test]
    fn test_stake() {
        let mut deps = mock_dependencies(&[]);

        deps.querier
            .with_token_balances(&[(&"loop_token".to_string(), &[])]);
        let amount = Uint128::new(11223344);
        let minter = String::from("asmodat");
        let limit = Uint128::new(511223344);

        let init_msg = InstantiateMsg {
            token: "loop_token".to_string(),
            freeze_lock_time: 86400u64,
            lock_time_frame: 7776000u64,
            vault_address: "vault_address".to_string(),
            restake_reset_flag: false,
            token_instantiate_msg: TokenInstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![Cw20Coin {
                    address: "addr0000".into(),
                    amount,
                }],
                mint: Some(MinterResponse {
                    minter: minter.clone(),
                    cap: Some(limit),
                }),
                marketing: None,
            },
        };
        let info = mock_info("loop_staker1", &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        //now testing staking
        let stake_msg1 = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            msg: to_binary(&Cw20HookMsg::Stake { duration: 1u64 }).unwrap(),
            amount: Uint128::from(100u128),
        });

        let stake_msg2 = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker2".to_string(),
            msg: to_binary(&Cw20HookMsg::Stake { duration: 1u64 }).unwrap(),
            amount: Uint128::from(100u128),
        });

        let _ = execute(
            deps.as_mut(),
            mock_env(),
            mock_info("loop_token", &[]),
            stake_msg1,
        )
        .unwrap();

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("loop_token", &[]),
            stake_msg2,
        )
        .unwrap();

        let user1_staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                mock_env(),
                QueryMsg::QueryStakedByUser {
                    wallet: info.sender.to_string(),
                    duration: 1u64,
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
                QueryMsg::QueryTotalStakedByDuration { duration: 1 },
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(total_staked, Uint128::from(200u128));
    }

    #[test]
    fn test_update_reward_and_distribution() {
        let mut deps = mock_dependencies(&[]);
        let amount = Uint128::new(11223344);
        let minter = String::from("asmodat");
        let limit = Uint128::new(511223344);
        let env = mock_env();
        let init_msg = InstantiateMsg {
            token: "loop_token".to_string(),
            freeze_lock_time: 86400u64,
            lock_time_frame: 2592000u64,
            vault_address: "vault_address".to_string(),
            restake_reset_flag: false,
            token_instantiate_msg: TokenInstantiateMsg {
                name: "Cash Token".to_string(),
                symbol: "CASH".to_string(),
                decimals: 9,
                initial_balances: vec![Cw20Coin {
                    address: "addr0000".into(),
                    amount,
                }],
                mint: Some(MinterResponse {
                    minter: minter.clone(),
                    cap: Some(limit),
                }),
                marketing: None,
            },
        };
        let info = mock_info("loop_staker1", &[]);
        let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();
        let duration = 1u64;
        // let update_reward_msg = ExecuteMsg::AddNewDuration { duration:duration.clone() } ;

        // execute(deps.as_mut(), mock_env(), info.clone(), update_reward_msg).unwrap();

        //now admin deposite
        let deposit_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            msg: to_binary(&Cw20HookMsg::Deposit {}).unwrap(),
            amount: Uint128::from(10000000000000000000000000u128),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("loop_token", &[]),
            deposit_msg,
        )
        .unwrap();

        //now testing staking
        let stake_msg1 = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: "loop_staker1".to_string(),
            msg: to_binary(&Cw20HookMsg::Stake {
                duration: duration.clone(),
            })
            .unwrap(),
            amount: Uint128::from(1000000u128),
        });

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info("loop_token", &[]),
            stake_msg1,
        )
        .unwrap();

        let update_reward_msg = ExecuteMsg::UpdateReward {
            amount: Uint128::from(1000000u128),
        };

        execute(deps.as_mut(), mock_env(), info.clone(), update_reward_msg).unwrap();

        let mut new_env = mock_env_time(env.block.time.plus_seconds(86401));

        //Calling Distribution
        let distribute = ExecuteMsg::Distribute {};
        execute(
            deps.as_mut(),
            new_env.clone(),
            info.clone(),
            distribute.clone(),
        )
        .unwrap();

        // let reward_user1: UserRewardResponse = from_binary(
        //     &query(
        //         deps.as_ref(),
        //         new_env.clone(),
        //         QueryMsg::QueryUserReward {
        //             wallet: "loop_staker1".to_string(),
        //         },
        //     )
        //     .unwrap(),
        // )
        // .unwrap();

        // println!("3rd day loop_staker1 {:?}", reward_user1);

        // let reward_user: UserRewardResponse = from_binary(
        //     &query(
        //         deps.as_ref(),
        //         new_env.clone(),
        //         QueryMsg::QueryUserReward {
        //             wallet: "loop_staker2".to_string(),
        //         },
        //     )
        //     .unwrap(),
        // )
        // .unwrap();

        // println!("loop_staker2 {:?}", reward_user);

        for x in 1..120 {
            // if x == 20 {
            //     let stake_msg2 = ExecuteMsg::Receive(Cw20ReceiveMsg {
            //         sender: "loop_staker3".to_string(),
            //         msg: to_binary(&Cw20HookMsg::Stake { duration: 1u64 }).unwrap(),
            //         amount: Uint128::from(1000u128),
            //     });
            //     execute(
            //         deps.as_mut(),
            //         new_env.clone(),
            //         mock_info("loop_token", &[]),
            //         stake_msg2,
            //     )
            //     .unwrap();
            // }

            let reward_user1: UserRewardResponse = from_binary(
                &query(
                    deps.as_ref(),
                    new_env.clone(),
                    QueryMsg::QueryUserReward {
                        wallet: "loop_staker1".to_string(),
                        duration: duration.clone(),
                    },
                )
                .unwrap(),
            )
            .unwrap();
            println!("{} day loop_staker1 {:?}", x, reward_user1);

            // if x > 15 {
            //     let reward_user2: UserRewardResponse = from_binary(
            //         &query(
            //             deps.as_ref(),
            //             new_env.clone(),
            //             QueryMsg::QueryUserReward {
            //                 wallet: "loop_staker2".to_string(),
            //                 duration: duration.clone(),
            //             },
            //         )
            //         .unwrap(),
            //     )
            //     .unwrap();

            //     println!("loop_staker2 {:?}", reward_user2);
            // }

            // if x == 15 {
            //     let stake_msg1 = ExecuteMsg::Receive(Cw20ReceiveMsg {
            //         sender: "loop_staker2".to_string(),
            //         msg: to_binary(&Cw20HookMsg::Stake {
            //             duration: duration.clone(),
            //         })
            //         .unwrap(),
            //         amount: Uint128::from(1000000u128),
            //     });

            //     let response = execute(
            //         deps.as_mut(),
            //         new_env.clone(),
            //         mock_info("loop_token", &[]),
            //         stake_msg1,
            //     )
            //     .unwrap();
            //     println!(
            //         "stake loop_staker2 {:?}
                
            //     ",
            //         response
            //     );
            // }
            let total_reward: Uint128 = from_binary(
                &query(
                    deps.as_ref(),
                    new_env.clone(),
                    QueryMsg::QueryTotalReward {},
                )
                .unwrap(),
            )
            .unwrap();
            println!(
                "total_reward {:?}
            
            ",
                total_reward
            );

            // if x == 20 {
            //     let update_reward_msg = ExecuteMsg::Restake {
            //         duration: duration.clone(),
            //     };

            //     let response = execute(
            //         deps.as_mut(),
            //         new_env.clone(),
            //         mock_info("loop_staker2", &[]),
            //         update_reward_msg,
            //     )
            //     .unwrap();
            //     println!(
            //         " restake stake loop_staker2 {:?}
                
            //     ",
            //         response
            //     );
            //     let total_reward: Uint128 = from_binary(
            //         &query(
            //             deps.as_ref(),
            //             new_env.clone(),
            //             QueryMsg::QueryTotalReward {},
            //         )
            //         .unwrap(),
            //     )
            //     .unwrap();
            //     println!(
            //         "total_reward {:?}
                
            //     ",
            //         total_reward
            //     );
            // }
            // if x == 5 {
            //     let update_reward_msg = ExecuteMsg::Restake {
            //         duration: duration.clone(),
            //     };

            //     let response = execute(
            //         deps.as_mut(),
            //         new_env.clone(),
            //         mock_info("loop_staker1", &[]),
            //         update_reward_msg,
            //     )
            //     .unwrap();
            //     println!(
            //         " restake stake loop_staker1 {:?}
                
            //     ",
            //         response
            //     );
            //     let total_reward: Uint128 = from_binary(
            //         &query(
            //             deps.as_ref(),
            //             new_env.clone(),
            //             QueryMsg::QueryTotalReward {},
            //         )
            //         .unwrap(),
            //     )
            //     .unwrap();
            //     println!(
            //         "total_reward {:?}
                
            //     ",
            //         total_reward
            //     );
            // }

            // if x == 15 {
            //     let update_reward_msg = ExecuteMsg::Claim { duration: 1u64 };

            //     println!(
            //         "claimed {:?}",
            //         execute(
            //             deps.as_mut(),
            //             new_env.clone(),
            //             mock_info("loop_staker1", &[]),
            //             update_reward_msg
            //         )
            //         .unwrap()
            //     );
            // }

            // if x == 25 {
            //     let stake_msg2 = ExecuteMsg::Receive(Cw20ReceiveMsg {
            //         sender: "loop_staker2".to_string(),
            //         msg: to_binary(&Cw20HookMsg::Stake { duration: 1u64 }).unwrap(),
            //         amount: Uint128::from(1000u128),
            //     });
            //     execute(
            //         deps.as_mut(),
            //         new_env.clone(),
            //         mock_info("loop_token", &[]),
            //         stake_msg2,
            //     )
            //     .unwrap();
            // }
            // if x == 20 {
            //     let stake_msg2 = ExecuteMsg::Receive(Cw20ReceiveMsg {
            //         sender: "loop_staker2".to_string(),
            //         msg: to_binary(&Cw20HookMsg::Stake { duration: 1u64 }).unwrap(),
            //         amount: Uint128::from(1000000u128),
            //     });
            //     execute(
            //         deps.as_mut(),
            //         new_env.clone(),
            //         mock_info("loop_token", &[]),
            //         stake_msg2,
            //     )
            //     .unwrap();
            // }

            // if x > 20 {
            //     let reward_user2: UserRewardResponse = from_binary(
            //         &query(
            //             deps.as_ref(),
            //             new_env.clone(),
            //             QueryMsg::QueryUserReward {
            //                 wallet: "loop_staker3".to_string(),
            //                 duration: 1u64,
            //             },
            //         )
            //         .unwrap(),
            //     )
            //     .unwrap();

            //     println!("loop_staker3 {:?}", reward_user2);

            //     let balance: Uint128 = from_binary(
            //         &query(
            //             deps.as_ref(),
            //             new_env.clone(),
            //             QueryMsg::Balance {
            //                 address: "loop_staker3".to_string(),
            //                 duration: 1,
            //             },
            //         )
            //         .unwrap(),
            //     )
            //     .unwrap();

            //     println!("loop_staker3 balance {:?}", balance);
            // }

            // if x == 46 {
            //     let claim_reward = ExecuteMsg::Restake {};
            //     let response = execute(
            //         deps.as_mut(),
            //         new_env.clone(),
            //         mock_info("loop_staker2", &[]),
            //         claim_reward.clone(),
            //     )
            //     .unwrap();
            //     println!(
            //         "loop_staker2 unstake reward{:?}

            //     ",
            //         response
            //     );
            // }

            // if x == 75 {
            //     let claim_reward = ExecuteMsg::Restake {};
            //     let response = execute(
            //         deps.as_mut(),
            //         new_env.clone(),
            //         mock_info("loop_staker2", &[]),
            //         claim_reward.clone(),
            //     )
            //     .unwrap();
            //     println!(
            //         "loop_staker2 unstake reward{:?}

            //     ",
            //         response
            //     );
            // }

            // if x == 120 {
            //     let claim_reward = ExecuteMsg::Restake {};
            //     let response = execute(
            //         deps.as_mut(),
            //         new_env.clone(),
            //         mock_info("loop_staker2", &[]),
            //         claim_reward.clone(),
            //     )
            //     .unwrap();
            //     println!(
            //         "loop_staker2 unstake reward{:?}

            //     ",
            //         response
            //     );
            // }
            new_env = mock_env_time(new_env.block.time.plus_seconds(86401));
            // if x <= 10 {
            //     let distribute = ExecuteMsg::Distribute {};
            //     execute(
            //         deps.as_mut(),
            //         new_env.clone(),
            //         info.clone(),
            //         distribute.clone(),
            //     )
            //     .unwrap();
            // }

            // if x >= 27 {
            let distribute = ExecuteMsg::Distribute {};
            execute(
                deps.as_mut(),
                new_env.clone(),
                info.clone(),
                distribute.clone(),
            )
            .unwrap();
            // }
        }

        let total_Staked: Uint128 = from_binary(
            &query(
                deps.as_ref(),
                new_env.clone(),
                QueryMsg::QueryTotalStakedByDuration {
                    duration: duration.clone(),
                },
            )
            .unwrap(),
        )
        .unwrap();

        println!(
            "total_Staked {:?}
        
        ",
            total_Staked
        );

        // for x in 25..80 {
        //     new_env = mock_env_time(new_env.block.time.plus_seconds(86401));

        //     let distribute = ExecuteMsg::Distribute {};
        //     execute(
        //         deps.as_mut(),
        //         new_env.clone(),
        //         info.clone(),
        //         distribute.clone(),
        //     )
        //     .unwrap();
        //     if x == 31 {
        //         // let stake_msg2 = ExecuteMsg::Receive(Cw20ReceiveMsg {
        //         //     sender: "loop_staker4".to_string(),
        //         //     msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
        //         //     amount: Uint128::from(10u128),
        //         // });
        //         // execute(
        //         //     deps.as_mut(),
        //         //     new_env.clone(),
        //         //     mock_info("loop_token", &[]),
        //         //     stake_msg2,
        //         // )
        //         // .unwrap();
        //     }

        //     let reward_user1: UserRewardResponse = from_binary(
        //         &query(
        //             deps.as_ref(),
        //             new_env.clone(),
        //             QueryMsg::QueryUserReward {
        //                 wallet: "loop_staker1".to_string(),
        //             },
        //         )
        //         .unwrap(),
        //     )
        //     .unwrap();

        //     println!("{} day loop_staker1 {:?}", x, reward_user1);

        //     let reward_user2: UserRewardResponse = from_binary(
        //         &query(
        //             deps.as_ref(),
        //             new_env.clone(),
        //             QueryMsg::QueryUserReward {
        //                 wallet: "loop_staker2".to_string(),
        //             },
        //         )
        //         .unwrap(),
        //     )
        //     .unwrap();

        //     println!("loop_staker2 {:?}", reward_user2);

        //     let reward_user1: UserRewardResponse = from_binary(
        //         &query(
        //             deps.as_ref(),
        //             new_env.clone(),
        //             QueryMsg::QueryUserReward {
        //                 wallet: "loop_staker1".to_string(),
        //             },
        //         )
        //         .unwrap(),
        //     )
        //     .unwrap();

        //     println!("{} day loop_staker1 {:?}", x, reward_user1);

        //     let reward_user2: UserRewardResponse = from_binary(
        //         &query(
        //             deps.as_ref(),
        //             new_env.clone(),
        //             QueryMsg::QueryUserReward {
        //                 wallet: "loop_staker2".to_string(),
        //             },
        //         )
        //         .unwrap(),
        //     )
        //     .unwrap();

        //     println!("loop_staker2 {:?}", reward_user2);

        //     let reward_user2: UserRewardResponse = from_binary(
        //         &query(
        //             deps.as_ref(),
        //             new_env.clone(),
        //             QueryMsg::QueryUserReward {
        //                 wallet: "loop_staker3".to_string(),
        //             },
        //         )
        //         .unwrap(),
        //     )
        //     .unwrap();

        //     println!("loop_staker3 {:?}", reward_user2);
        //     if x >= 32 {
        //         // let reward_user2: UserRewardResponse = from_binary(
        //         //     &query(
        //         //         deps.as_ref(),
        //         //         new_env.clone(),
        //         //         QueryMsg::QueryUserReward {
        //         //             wallet: "loop_staker4".to_string(),
        //         //         },
        //         //     )
        //         //     .unwrap(),
        //         // )
        //         // .unwrap();

        //         // println!(
        //         //     "loop_staker4 {:?}

        //         //     ",
        //         //     reward_user2
        //         // );
        //     }
        //     if x==35 {
        //         let claim_reward = ExecuteMsg::UnstakeAndClaim {  };
        //         let response = execute(
        //             deps.as_mut(),
        //             new_env.clone(),
        //             mock_info("loop_staker2", &[]),
        //             claim_reward.clone(),
        //         )
        //         .unwrap();
        //         println!(
        //             "loop_staker2 unstake reward{:?}

        //         ",
        //             response
        //         );
        //     }
        //     if x == 75 {

        //         let reward_user2: UserRewardResponse = from_binary(
        //             &query(
        //                 deps.as_ref(),
        //                 new_env.clone(),
        //                 QueryMsg::QueryUserReward {
        //                     wallet: "loop_staker3".to_string(),
        //                 },
        //             )
        //             .unwrap(),
        //         )
        //         .unwrap();

        //         println!(
        //             "loop_staker3 unstake reward{:?}

        //         ",
        //         reward_user2
        //         );
        //     }
        // }

        // let reward_user1: UserRewardResponse = from_binary(
        //     &query(
        //         deps.as_ref(),
        //         new_env.clone(),
        //         QueryMsg::QueryUserReward {
        //             wallet: "loop_staker1".to_string(),
        //         },
        //     )
        //     .unwrap(),
        // )
        // .unwrap();

        // println!("loop_staker1 {:?}", reward_user1);

        // let reward_user2: UserRewardResponse = from_binary(
        //     &query(
        //         deps.as_ref(),
        //         new_env.clone(),
        //         QueryMsg::QueryUserReward {
        //             wallet: "loop_staker2".to_string(),
        //         },
        //     )
        //     .unwrap(),
        // )
        // .unwrap();

        // println!("loop_staker2 {:?}", reward_user2);

        // let reward_user2: UserRewardResponse = from_binary(
        //     &query(
        //         deps.as_ref(),
        //         new_env.clone(),
        //         QueryMsg::QueryUserReward {
        //             wallet: "loop_staker3".to_string(),
        //         },
        //     )
        //     .unwrap(),
        // )
        // .unwrap();

        // println!("loop_staker3 {:?}", reward_user2);

        // let reward_user2: UserRewardResponse = from_binary(
        //     &query(
        //         deps.as_ref(),
        //         new_env.clone(),
        //         QueryMsg::QueryUserReward {
        //             wallet: "loop_staker4".to_string(),
        //         },
        //     )
        //     .unwrap(),
        // )
        // .unwrap();

        // println!("loop_staker4 {:?}", reward_user2);
    }

    // #[test]
    // fn test_unstake_and_claim() {
    //     let mut deps = mock_dependencies(&[]);

    //     deps.querier.with_token_balances(&[(
    //         &"loop_token".to_string(),
    //         &[
    //             (&MOCK_CONTRACT_ADDR.to_string(), &Uint128::new(10000)),
    //             (&"loop_staker1".to_string(), &Uint128::new(10000)),
    //         ],
    //     )]);
    //     let env = mock_env();
    //     let init_msg = InstantiateMsg {
    //         token: "loop_token".to_string(),
    //         freeze_lock_time: 86400u64,
    //         lock_time_frame: 7776000u64,

    //         restake_reset_flag: false,
    //     };
    //     let info = mock_info("loop_staker1", &[]);
    //     let _result = instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

    //     let deposit_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
    //         sender: "loop_staker1".to_string(),
    //         msg: to_binary(&Cw20HookMsg::Deposit {}).unwrap(),
    //         amount: Uint128::from(10000u128),
    //     });

    //     execute(
    //         deps.as_mut(),
    //         mock_env(),
    //         mock_info("loop_token", &[]),
    //         deposit_msg,
    //     )
    //     .unwrap();

    //     //now testing staking
    //     let stake_msg1 = ExecuteMsg::Receive(Cw20ReceiveMsg {
    //         sender: "loop_staker1".to_string(),
    //         msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
    //         amount: Uint128::from(300u128),
    //     });

    //     let stake_msg2 = ExecuteMsg::Receive(Cw20ReceiveMsg {
    //         sender: "loop_staker2".to_string(),
    //         msg: to_binary(&Cw20HookMsg::Stake {}).unwrap(),
    //         amount: Uint128::from(100u128),
    //     });

    //     execute(
    //         deps.as_mut(),
    //         mock_env(),
    //         mock_info("loop_token", &[]),
    //         stake_msg1,
    //     )
    //     .unwrap();
    //     execute(
    //         deps.as_mut(),
    //         mock_env(),
    //         mock_info("loop_token", &[]),
    //         stake_msg2,
    //     )
    //     .unwrap();

    //     //now testing unstaking and claiming
    //     let unstake_msg = ExecuteMsg::UnstakeAndClaim {};

    //     let err = execute(
    //         deps.as_mut(),
    //         mock_env(),
    //         mock_info("loop_staker1", &[]),
    //         unstake_msg.clone(),
    //     );
    //     match err {
    //         Err(StdError::GenericErr { msg, .. }) => assert_eq!(
    //             msg,
    //             "The rewards are still locked. Please wait patiently for the specified time"
    //         ),
    //         _ => panic!("Invalid error"),
    //     }

    //     let update_reward_msg = ExecuteMsg::UpdateReward {
    //         amount: Uint128::from(100u128),
    //     };

    //     execute(deps.as_mut(), mock_env(), info.clone(), update_reward_msg).unwrap();

    //     let new_env = mock_env_time(env.block.time.plus_seconds(86401));
    //     //Calling Distribution
    //     let distribute_msg = ExecuteMsg::Distribute {};
    //     execute(deps.as_mut(), new_env.clone(), info.clone(), distribute_msg).unwrap();

    //     let res = execute(
    //         deps.as_mut(),
    //         new_env,
    //         mock_info("loop_staker1", &[]),
    //         unstake_msg,
    //     )
    //     .unwrap();

    //     let msg_refund_staked = res.messages.get(0).expect("no message");
    //     let msg_give_reward = res.messages.get(1).expect("no message");
    //     assert_eq!(
    //         msg_refund_staked,
    //         &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
    //             contract_addr: "loop_token".to_string(),
    //             msg: to_binary(&Cw20ExecuteMsg::Transfer {
    //                 recipient: info.sender.to_string(),
    //                 amount: Uint128::from(86400u128),
    //             })
    //             .unwrap(),
    //             funds: vec![],
    //         }))
    //     );

    //     assert_eq!(
    //         msg_give_reward,
    //         &SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
    //             contract_addr: "loop_token".to_string(),
    //             msg: to_binary(&Cw20ExecuteMsg::Transfer {
    //                 recipient: info.sender.to_string(),
    //                 amount: Uint128::from(1050u128),
    //             })
    //             .unwrap(),
    //             funds: vec![],
    //         }))
    //     );

    //     let user1_staked: Uint128 = from_binary(
    //         &query(
    //             deps.as_ref(),
    //             mock_env(),
    //             QueryMsg::QueryStakedByUser {
    //                 wallet: info.sender.to_string(),
    //             },
    //         )
    //         .unwrap(),
    //     )
    //     .unwrap();

    //     assert_eq!(user1_staked, Uint128::from(0u128));

    //     let total_staked: Uint128 =
    //         from_binary(&query(deps.as_ref(), mock_env(), QueryMsg::QueryTotalStaked {}).unwrap())
    //             .unwrap();
    //     assert_eq!(total_staked, Uint128::from(100u128));
    // }
}
