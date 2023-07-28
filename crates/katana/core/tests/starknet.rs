use blockifier::abi::abi_utils::{get_storage_var_address, selector_from_name};
use blockifier::transaction::account_transaction::AccountTransaction;
use blockifier::transaction::transaction_execution::Transaction;
use katana_core::backend::config::{Environment, StarknetConfig};
use katana_core::backend::Backend;
use katana_core::constants::FEE_TOKEN_ADDRESS;
use starknet::core::types::TransactionStatus;
use starknet_api::block::BlockNumber;
use starknet_api::core::Nonce;
use starknet_api::hash::StarkFelt;
use starknet_api::transaction::{
    Calldata, InvokeTransaction, InvokeTransactionV1, TransactionHash,
};
use starknet_api::{calldata, stark_felt};

async fn create_test_starknet() -> Backend {
    let test_account_path =
        [env!("CARGO_MANIFEST_DIR"), "./contracts/compiled/account_without_validation.json"]
            .iter()
            .collect();

    let starknet = Backend::new(StarknetConfig {
        seed: [0u8; 32],
        auto_mine: true,
        total_accounts: 2,
        allow_zero_max_fee: true,
        account_path: Some(test_account_path),
        env: Environment::default(),
    });

    starknet.generate_genesis_block().await;
    starknet
}

#[tokio::test]
async fn test_next_block_timestamp_in_past() {
    let starknet = create_test_starknet().await;
    starknet.generate_pending_block().await;

    let timestamp = starknet.block_context.read().block_timestamp;
    starknet.set_next_block_timestamp(timestamp.0 - 1000).await.unwrap();

    starknet.generate_pending_block().await;
    let new_timestamp = starknet.block_context.read().block_timestamp;

    assert_eq!(new_timestamp.0, timestamp.0 - 1000, "timestamp should be updated");
}

#[tokio::test]
async fn test_set_next_block_timestamp_in_future() {
    let starknet = create_test_starknet().await;
    starknet.generate_pending_block().await;

    let timestamp = starknet.block_context.read().block_timestamp;
    starknet.set_next_block_timestamp(timestamp.0 + 1000).await.unwrap();

    starknet.generate_pending_block().await;
    let new_timestamp = starknet.block_context.read().block_timestamp;

    assert_eq!(new_timestamp.0, timestamp.0 + 1000, "timestamp should be updated");
}

#[tokio::test]
async fn test_increase_next_block_timestamp() {
    let starknet = create_test_starknet().await;
    starknet.generate_pending_block().await;

    let timestamp = starknet.block_context.read().block_timestamp;
    starknet.increase_next_block_timestamp(1000).await.unwrap();

    starknet.generate_pending_block().await;
    let new_timestamp = starknet.block_context.read().block_timestamp;

    assert_eq!(new_timestamp.0, timestamp.0 + 1000, "timestamp should be updated");
}

#[tokio::test]
async fn test_creating_blocks() {
    let starknet = create_test_starknet().await;
    starknet.generate_pending_block().await;
    starknet.generate_latest_block().await;

    assert_eq!(starknet.blocks.read().await.hash_to_num.len(), 2);
    assert_eq!(starknet.blocks.read().await.num_to_block.len(), 2);
    assert_eq!(
        starknet.block_context.read().block_number,
        BlockNumber(1),
        "block context should only be updated on new pending block"
    );

    let block0 = starknet.blocks.read().await.by_number(BlockNumber(0)).unwrap();
    let block1 = starknet.blocks.read().await.by_number(BlockNumber(1)).unwrap();
    let last_block = starknet.blocks.read().await.latest().unwrap();

    assert_eq!(block0.transactions().len(), 4, "genesis block should have 4 transactions");
    assert_eq!(block0.block_number(), BlockNumber(0));
    assert_eq!(block1.block_number(), BlockNumber(1));
    assert_eq!(last_block.block_number(), BlockNumber(1));
}

#[tokio::test]
async fn test_add_transaction() {
    let starknet = create_test_starknet().await;
    starknet.generate_pending_block().await;

    let a = starknet.predeployed_accounts.accounts[0].clone();
    let b = starknet.predeployed_accounts.accounts[1].clone();

    // CREATE `transfer` INVOKE TRANSACTION
    //

    let entry_point_selector = selector_from_name("transfer");
    let execute_calldata = calldata![
        *FEE_TOKEN_ADDRESS,         // Contract address.
        entry_point_selector.0,     // EP selector.
        stark_felt!(3_u8),          // Calldata length.
        *b.account_address.0.key(), // Calldata: num.
        stark_felt!("0x99"),        // Calldata: num.
        stark_felt!(0_u8)           // Calldata: num.
    ];

    starknet
        .handle_transaction(Transaction::AccountTransaction(AccountTransaction::Invoke(
            InvokeTransaction::V1(InvokeTransactionV1 {
                sender_address: a.account_address,
                calldata: execute_calldata,
                transaction_hash: TransactionHash(stark_felt!("0x6969")),
                nonce: Nonce(1u8.into()),
                ..Default::default()
            }),
        )))
        .await;

    // SEND INVOKE TRANSACTION
    //

    let transactions = starknet.transactions.read().await;
    let tx = transactions.transactions.get(&TransactionHash(stark_felt!("0x6969")));

    let block = starknet.blocks.read().await.by_number(BlockNumber(1)).unwrap();

    assert!(tx.is_some(), "transaction must be stored");
    assert_eq!(tx.unwrap().block_number, Some(BlockNumber(1)));
    assert!(block.transaction_by_index(0).is_some(), "transaction must be included in the block");
    assert_eq!(
        block.transaction_by_index(0).unwrap().transaction_hash(),
        TransactionHash(stark_felt!("0x6969"))
    );
    assert_eq!(tx.unwrap().status, TransactionStatus::AcceptedOnL2);

    // CHECK THAT THE BALANCE IS UPDATED
    //

    println!("FEE Address : {}", *FEE_TOKEN_ADDRESS);
    println!(
        "STORAGE ADDR : {}",
        get_storage_var_address("ERC20_balances", &[*a.account_address.0.key()]).unwrap().0.key()
    );

    // println!(
    //     "After {:?}",
    //     starknet.state.state.storage_view.get(&(
    //         ContractAddress(patricia_key!(FEE_ERC20_CONTRACT_ADDRESS)),
    //         get_storage_var_address("ERC20_balances", &[*a.account_address.0.key()]).unwrap()
    //     ))
    // );
}

#[tokio::test]
async fn test_add_reverted_transaction() {
    let starknet = create_test_starknet().await;
    starknet.generate_pending_block().await;

    let transaction_hash = TransactionHash(stark_felt!("0x1234"));
    let transaction = Transaction::AccountTransaction(AccountTransaction::Invoke(
        InvokeTransaction::V1(InvokeTransactionV1 { transaction_hash, ..Default::default() }),
    ));

    starknet.handle_transaction(transaction).await;

    let transactions = starknet.transactions.read().await;
    let tx = transactions.transactions.get(&transaction_hash);

    assert_eq!(
        starknet.transactions.read().await.transactions.len(),
        5,
        "transaction must be stored even if execution fail"
    );
    assert_eq!(tx.unwrap().block_hash, None);
    assert_eq!(tx.unwrap().block_number, None);
    assert_eq!(tx.unwrap().status, TransactionStatus::Rejected);
    assert_eq!(
        starknet.blocks.read().await.num_to_block.len(),
        1,
        "no new block should be created if tx failed"
    );
}

// #[test]
// fn test_function_call() {
//     let starknet = create_test_starknet();
//     let account = &starknet.predeployed_accounts.accounts[0]
//         .account_address
//         .0
//         .key();

//     let call = ExternalFunctionCall {
//         calldata: Calldata(Arc::new(vec![**account])),
//         contract_address: ContractAddress(patricia_key!(FEE_ERC20_CONTRACT_ADDRESS)),
//         entry_point_selector: EntryPointSelector(StarkFelt::from(
//             get_selector_from_name("balanceOf").unwrap(),
//         )),
//     };

//     let res = starknet.call(call);

//     assert!(res.is_ok(), "call must succeed");
//     assert_eq!(
//         res.unwrap().execution.retdata.0[0],
//         stark_felt!(DEFAULT_PREFUNDED_ACCOUNT_BALANCE),
//     );
// }