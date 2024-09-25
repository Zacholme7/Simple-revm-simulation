use alloy::primitives::{address, U256};
use alloy::sol;
use alloy::sol_types::{SolCall, SolValue};
use anyhow::Result;
use revm::db::{CacheDB, EmptyDB};
use revm::primitives::{AccountInfo, Bytecode, ExecutionResult, Output, TransactTo};
use revm::Evm;

// Generate our contract interface with the sol! macro
sol!(Counter, "Counter.json");

fn main() -> Result<()> {
    // Dummy addresses we will use
    let counter_address = address!("A5C381211A406b48A073E954e6949B0D49506bc0");
    let caller = address!("0000000000000000000000000000000000000001");

    // Create the db to hold all EVM related state. This is analogous to the DB on the blockchain.
    // Any state will be stored here
    let mut db = CacheDB::new(EmptyDB::new());

    // insert our contract into the DB. In a way, this is "deploying" the contract to the chain.
    let counter_bytecode = Bytecode::new_raw(Counter::DEPLOYED_BYTECODE.clone());
    let counter_bytecode_hash = counter_bytecode.hash_slow();
    let counter_account = AccountInfo {
        balance: U256::ZERO,
        nonce: 0_u64,
        code: Some(counter_bytecode),
        code_hash: counter_bytecode_hash,
    };
    db.insert_account_info(counter_address, counter_account);

    // Encode our increment call
    let increment_calldata = Counter::incrementCall {}.abi_encode();

    // Construct the evm instance, this is what we use to execute the transaction
    let mut evm = Evm::builder()
        .with_db(db)
        .modify_tx_env(|tx| {
            tx.caller = caller;
            tx.transact_to = TransactTo::Call(counter_address);
            tx.data = increment_calldata.into();
            tx.value = U256::ZERO;
        })
        .build();

    // transact and commit this transaction to the database!
    if let Err(e) = evm.transact_commit() {
        panic!("Increment call failed: {:?}", e);
    }

    // Following the same procedure above, we can now call the getCount function and we should see
    // an incremented count. All we have to do it modify the calldata as we just want to call a
    // different function on the contract
    let getcount_calldata = Counter::getCountCall {}.abi_encode();
    evm.tx_mut().data = getcount_calldata.into();

    let ref_tx = evm.transact().unwrap();
    let result = ref_tx.result;

    match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => {
            let count = <U256>::abi_decode(&value, false).unwrap();
            println!("Count is {}!", count);
        }
        _result => panic!("Get count call failed!"),
    };

    Ok(())
}
