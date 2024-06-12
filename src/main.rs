use alloy_network::EthereumSigner;
use alloy_node_bindings::Anvil;
use alloy_provider::Provider;
use alloy_provider::ProviderBuilder;
use alloy_signer_wallet::LocalWallet;
use alloy_sol_types::{sol, SolCall};
use anyhow::Result;
use foundry_evm::fork::{BlockchainDb, BlockchainDbMeta, SharedBackend};
use revm::{db::CacheDB, Evm};
use std::collections::BTreeSet;

use crate::Counter::*;

sol!(
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
    // setup provdier to forked anvil
    let anvil = Anvil::new()
        .fork("https://blastl2-mainnet.blastapi.io/f862eb6f-8672-4ee1-a02a-9def8b777f51")
        .try_spawn()?;

    let signer: LocalWallet = anvil.keys()[0].clone().into();

    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .network::<alloy_network::AnyNetwork>()
        .signer(EthereumSigner::from(signer))
        .on_builtin(&anvil.endpoint())
        .await?;

    // deploy the contract
    let contract = Counter::deploy(&provider).await?;
    println!("Deployed contract at {}", contract.address());

    // setup shared backend
    let block_number = provider.get_block_number().await?;

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


    let increment_call_encode = incrementCall::new(()).abi_encode();




    //let increment_call_decode = incrementCall::abi_decode_return(data, true)?;

    /*

    let db = CacheDB::new(EmptyDB::default());
    let mut evm = Evm::builder().with_db(db).build();


    let user = address!("E2b5A9c1e325511a227EF527af38c3A7B65AFA1d");
    let usdt = address!("dAC17F958D2ee523a2206206994597C13D831ec7");



    //let calldata = balanceOfCall { owner: user }.abi_encode();
    let calldata = balanceOfCall::new((user,)).abi_encode();
    // clones the current dst of tx, evm.tx() get ref to current tx setup, transact_to is the dst,
    // clone will clone it
    let original = evm.tx().transact_to.clone();

    // now we can rewrite over it
    // set that we are doing a call to the usdt address
    evm.tx_mut().transact_to = TransactTo::Call(usdt);
    evm.tx_mut().data = calldata.into();

    let result = match evm.transact() {
        Ok(result) => result,
        Err(e) => return Err(anyhow!("EVM ref call failed: {e:?}")),
    };

    let tx_result = match result.result {
        ExecutionResult::Success {
            gas_used,
            gas_refunded,
            output,
            logs,
            ..
        } => match output {
            Output::Call(o) => TxResult {
                output: o,
                logs: Some(logs),
                gas_used,
                gas_refunded,
            },
            _ => panic!("should not reach here")
        }
        _ => panic!("should not reach here")
    };


    let ret = balanceOfCall::abi_decode_returns(&tx_result.output, true)?;


    println!("{:?}", tx_result);
    */
    Ok(())
}
