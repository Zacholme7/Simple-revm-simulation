use alloy_network::EthereumSigner;
use alloy_node_bindings::Anvil;
use alloy_primitives::{address, U256};
use alloy_provider::Provider;
use alloy_provider::ProviderBuilder;
use alloy_signer_wallet::LocalWallet;
use alloy_sol_types::{sol, SolCall};
use anyhow::Result;
use foundry_evm::fork::{BlockchainDb, BlockchainDbMeta, SharedBackend};
use revm::interpreter::primitives::{keccak256, AccountInfo, Bytecode, Bytes, TransactTo};
use revm::{db::CacheDB, Evm};
use revm_primitives::{ExecutionResult, Output};
use std::collections::BTreeSet;

sol!(
    #[derive(Debug)]
    #[sol(rpc, bytecode = "6080604052348015600e575f80fd5b506101778061001c5f395ff3fe608060405234801561000f575f80fd5b506004361061003f575f3560e01c806306661abd14610043578063a87d942c14610061578063d09de08a1461007f575b5f80fd5b61004b610089565b60405161005891906100c8565b60405180910390f35b61006961008e565b60405161007691906100c8565b60405180910390f35b610087610096565b005b5f5481565b5f8054905090565b60015f808282546100a7919061010e565b92505081905550565b5f819050919050565b6100c2816100b0565b82525050565b5f6020820190506100db5f8301846100b9565b92915050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601160045260245ffd5b5f610118826100b0565b9150610123836100b0565b925082820190508082111561013b5761013a6100e1565b5b9291505056fea2646970667358221220dc61928b75c9f79e8b82b86e93a996b4da03268809cca775e9130181f9b398eb64736f6c634300081a0033")]
    contract Counter {
        uint256 public count;

        function increment() public {
            count += 1;
        }

        function getCount() public view returns (uint256) {
            return count;
        }
    }
);

#[tokio::main]
async fn main() -> Result<()> {
    // setup provider to forked anvil
    let anvil = Anvil::new().fork("https://eth.merkle.io").try_spawn()?;

    // signer to deploy the contract
    let signer: LocalWallet = anvil.keys()[0].clone().into();

    // provider communicating with anvil
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .network::<alloy_network::AnyNetwork>()
        .signer(EthereumSigner::from(signer))
        .on_builtin(&anvil.endpoint())
        .await?;

    // deploy the contract
    let contract = Counter::deploy(&provider).await?;
    println!("Deployed contract at {}", contract.address());

    let block_number = provider.get_block_number().await?;

    // setup shared backend
    let shared_backend = SharedBackend::spawn_backend_thread(
        provider.clone(),
        BlockchainDb::new(
            BlockchainDbMeta {
                cfg_env: Default::default(),
                block_env: Default::default(),
                hosts: BTreeSet::from(["".to_string()]),
            },
            None,
        ),
        Some(block_number.into()),
    );

    let db = CacheDB::new(shared_backend);
    let mut evm = Evm::builder().with_db(db).build();

    // modify the env
    evm.cfg_mut().limit_contract_code_size = Some(0x100000);
    evm.cfg_mut().disable_block_gas_limit = true;
    evm.cfg_mut().disable_base_fee = true;
    evm.block_mut().number = U256::from(block_number + 1);

    let mut_db = evm.db_mut();

    // setup user account and insert into database
    let user = address!("18B06aaF27d44B756FCF16Ca20C1f183EB49111f");
    let ten_eth = U256::from(10)
        .checked_mul(U256::from(10).pow(U256::from(18)))
        .unwrap();
    let user_account_info =
        AccountInfo::new(ten_eth, 0, keccak256(Bytes::new()), Bytecode::default());
    mut_db.insert_account_info(user, user_account_info);

    // call increment
    let increment_call_encode = Counter::incrementCall::new(()).abi_encode();
    evm.tx_mut().caller = address!("0000000000000000000000000000000000000000");
    evm.tx_mut().transact_to = TransactTo::Call(*contract.address());
    evm.tx_mut().data = increment_call_encode.into();
    evm.transact_commit().unwrap();

    // modify transaction and call getCount
    let getcount_call_encode = Counter::getCountCall::new(()).abi_encode();
    evm.tx_mut().data = getcount_call_encode.into();
    let ref_tx = evm.transact().unwrap();
    let result = ref_tx.result;

    let value = match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            ..
        } => value,
        _result => panic!("it failed"),
    };

    let output = Counter::getCountCall::abi_decode_returns(&value, false)?;
    println!("Output after incrementing: {:?}", output);
    Ok(())
}
