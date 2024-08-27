#![allow(non_snake_case)]

mod proxy;

use multiversx_price_aggregator_sc::__wasm__endpoints__::latest_round_data;
use multiversx_sc_snippets::imports::*;
use multiversx_sc_snippets::multiversx_sc_scenario::api::VMHooksApi;
use multiversx_sc_snippets::sdk;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path, u8,
};


const GATEWAY: &str = sdk::gateway::DEVNET_GATEWAY;
const STATE_FILE: &str = "state.toml";
const STAKING_TOKEN: &str = "BSK-476470";
const STAKING_AMOUNT: u128 = 10;
const SLASH_AMOUNT: u128 = 1;
const SLASH_QUORUM: u32 = 3;
const SUBMISSION_COUNT: u32 = 3;
const FIRST_ORACLE_ADDRESS: &str = "erd1qyu5wthldzr8wx5c9ucg8kjagg0jfs53s8nr3zpz3hypefsdd8ssycr6th";
const SECOND_ORACLE_ADDRESS: &str = "erd1spyavw0956vq68xj8y4tenjpq2wd5a9p2c6j8gsz7ztyrnpxrruqzu66jx";
const THIRD_ORACLE_ADDRESS: &str = "erd13x29rvmp4qlgn4emgztd8jgvyzdj0p6vn37tqxas3v9mfhq4dy7shalqrx";
const FOURTH_ORACLE_ADDRESS: &str = "erd1k2s324ww2g0yj38qn2ch2jwctdy8mnfxep94q9arncc6xecg3xaq6mjse8";
const FIFTH_ORACLE_ADDRESS: &str = "erd1kyaqzaprcdnv4luvanah0gfxzzsnpaygsy6pytrexll2urtd05ts9vegu7";
const SIXTH_ORACLE_ADDRESS: &str = "erd18tudnj2z8vjh0339yu3vrkgzz2jpz8mjq0uhgnmklnap6z33qqeszq2yn4";

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut args = std::env::args();
    let _ = args.next();
    let cmd = args.next().expect("at least one argument required");
    let mut interact = ContractInteract::new().await;
    match cmd.as_str() {
        "deploy" => interact.deploy().await,
        "changeAmounts" => interact.change_amounts().await,
        "addOracles" => interact.add_oracles().await,
        "removeOracles" => interact.remove_oracles().await,
        "submit" => interact.submit().await,
        "submitBatch" => interact.submit_batch().await,
        "latestRoundData" => interact.latest_round_data().await,
        "latestPriceFeed" => interact.latest_price_feed().await,
        "latestPriceFeedOptional" => interact.latest_price_feed_optional().await,
        "setSubmissionCount" => interact.set_submission_count().await,
        "getOracles" => interact.get_oracles().await,
        "setPairDecimals" => interact.set_pair_decimals().await,
        "getPairDecimals" => interact.get_pair_decimals().await,
        "submission_count" => interact.submission_count().await,
        "pause" => interact.pause_endpoint().await,
        "unpause" => interact.unpause_endpoint().await,
        "isPaused" => interact.paused_status().await,
        "stake" => interact.stake().await,
        "unstake" => interact.unstake().await,
        "voteSlashMember" => interact.vote_slash_member().await,
        "cancelVoteSlashMember" => interact.cancel_vote_slash_member().await,
        "slashMember" => interact.slash_member().await,
        _ => panic!("unknown command: {}", &cmd),
    }
}


#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    contract_address: Option<Bech32Address>
}

impl State {
        // Deserializes state from file
        pub fn load_state() -> Self {
            if Path::new(STATE_FILE).exists() {
                let mut file = std::fs::File::open(STATE_FILE).unwrap();
                let mut content = String::new();
                file.read_to_string(&mut content).unwrap();
                toml::from_str(&content).unwrap()
            } else {
                Self::default()
            }
        }
    
        /// Sets the contract address
        pub fn set_address(&mut self, address: Bech32Address) {
            self.contract_address = Some(address);
        }
    
        /// Returns the contract address
        pub fn current_address(&self) -> &Bech32Address {
            self.contract_address
                .as_ref()
                .expect("no known contract, deploy first")
        }
    }
    
    impl Drop for State {
        // Serializes state to file
        fn drop(&mut self) {
            let mut file = std::fs::File::create(STATE_FILE).unwrap();
            file.write_all(toml::to_string(self).unwrap().as_bytes())
                .unwrap();
        }
    }

struct ContractInteract {
    interactor: Interactor,
    wallet_address: Address,
    contract_code: BytesValue,
    state: State
}

impl ContractInteract {
    async fn new() -> Self {
        let mut interactor = Interactor::new(GATEWAY).await;
        let wallet_address = interactor.register_wallet(test_wallets::alice());
        
        let contract_code = BytesValue::interpret_from(
            "mxsc:../output/multiversx-price-aggregator-sc.mxsc.json",
            &InterpreterContext::default(),
        );

        ContractInteract {
            interactor,
            wallet_address,
            contract_code,
            state: State::load_state()
        }
    }

    async fn deploy(&mut self) {
        let staking_token = EgldOrEsdtTokenIdentifier::esdt(&b""[..]);
        let staking_amount = BigUint::<StaticApi>::from(0u128);
        let slash_amount = BigUint::<StaticApi>::from(0u128);
        let slash_quorum = 0u32;
        let submission_count = 0u32;
        let oracles = MultiValueVec::from(vec![bech32::decode("")]);

        let new_address = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .init(staking_token, staking_amount, slash_amount, slash_quorum, submission_count, oracles)
            .code(&self.contract_code)
            .returns(ReturnsNewAddress)
            .prepare_async()
            .run()
            .await;
        let new_address_bech32 = bech32::encode(&new_address);
        self.state
            .set_address(Bech32Address::from_bech32_string(new_address_bech32.clone()));

        println!("new address: {new_address_bech32}");
    }

    // async fn deploy_fail(&mut self, expected_result: ExpectError<'_>) {
    //     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
    //     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
    //     let slash_amount = BigUint::<StaticApi>::from(30u128);
    //     let first_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS));
    //     let second_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS));
    //     let third_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(THIRD_ORACLE_ADDRESS));
    //     let fourth_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(FOURTH_ORACLE_ADDRESS));
    //     let oracles: MultiValueVec<ManagedAddress<StaticApi>> = MultiValueVec::from(vec![first_oracle_address, second_oracle_address, third_oracle_address, fourth_oracle_address]);
    //     let slash_quorum = SLASH_QUORUM;
    //     let slash_amount = SLASH_AMOUNT;

    //     let new_address = self
    //         .interactor
    //         .tx()
    //         .from(&self.wallet_address)
    //         .gas(30_000_000u64)
    //         .typed(proxy::PriceAggregatorProxy)
    //         .init(staking_token, staking_amount, SLASH_AMOUNT, SLASH_QUORUM, submission_count, oracles)
    //         .code(&self.contract_code)
    //         .returns(expected_result)
    //         .prepare_async()
    //         .run()
    //         .await;
    // }


    async fn deploy_price_aggregator_fail(
        &mut self,
        staking_token: EgldOrEsdtTokenIdentifier<StaticApi>,
        staking_amount: BigUint<StaticApi>,
        slash_amount: BigUint<StaticApi>,
        slash_quorum: u32,
        submission_count: u32,
        oracles: MultiValueVec<ManagedAddress<StaticApi>>,
        expected_result: ExpectError<'_>,
    ) {
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(100_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .init(
                staking_token,
                staking_amount,
                slash_amount,
                slash_quorum,
                submission_count,
                oracles,
            )
            .code(&self.contract_code)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
            
            println!("Result: {response:?}");
    }

    async fn deploy_price_aggregator(
        &mut self,
        staking_token: EgldOrEsdtTokenIdentifier<StaticApi>,
        staking_amount: BigUint<StaticApi>,
        slash_amount: BigUint<StaticApi>,
        slash_quorum: u32,
        submission_count: u32,
        oracles: MultiValueVec<ManagedAddress<StaticApi>>,
    ) {
        let new_address = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .gas(100_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .init(
                staking_token,
                staking_amount,
                slash_amount,
                slash_quorum,
                submission_count,
                oracles,
            )
            .code(&self.contract_code)
            .returns(ReturnsNewAddress)
            .prepare_async()
            .run()
            .await;

        let new_address_bech32 = bech32::encode(&new_address);
        self.state
            .set_address(Bech32Address::from_bech32_string(new_address_bech32.clone()));

        println!("new address: {new_address_bech32}");
    }

    async fn change_amounts(&mut self) {
        let staking_amount = BigUint::<StaticApi>::from(0u128);
        let slash_amount = BigUint::<StaticApi>::from(0u128);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .change_amounts(staking_amount, slash_amount)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn add_oracles(&mut self) {
        let oracles = MultiValueVec::from(vec![bech32::decode("")]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .add_oracles(oracles)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn add_oracles_with_params(&mut self, oracles: MultiValueVec<ManagedAddress<StaticApi>>) {

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .add_oracles(oracles)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    
    async fn add_oracles_with_params_fail(&mut self, oracles: MultiValueVec<ManagedAddress<StaticApi>>, expected_result: ExpectError<'_>) {

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .add_oracles(oracles)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn remove_oracles(&mut self) {
        let submission_count = 0u32;
        let oracles = MultiValueVec::from(vec![bech32::decode("")]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .remove_oracles(submission_count, oracles)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn remove_oracles_with_params(&mut self, oracles: MultiValueVec<ManagedAddress<StaticApi>>, submission_count: u32) {
        // let submission_count = 0u32;
        // let oracles = MultiValueVec::from(vec![bech32::decode("")]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .remove_oracles(submission_count, oracles)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn submit(&mut self) {
        let from = ManagedBuffer::new_from_bytes(&b""[..]);
        let to = ManagedBuffer::new_from_bytes(&b""[..]);
        let submission_timestamp = 0u64;
        let price = BigUint::<StaticApi>::from(0u128);
        let decimals = 0u8;

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .submit(from, to, submission_timestamp, price, decimals)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn submit_with_params(
        &mut self,
        from: ManagedBuffer<StaticApi>,
        to: ManagedBuffer<StaticApi>,
        submission_timestamp: u64,
        price: BigUint<StaticApi>,
        decimals: u8
    ) {
        // let from = ManagedBuffer::new_from_bytes(&b""[..]);
        // let to = ManagedBuffer::new_from_bytes(&b""[..]);
        // let submission_timestamp = 0u64;
        // let price = BigUint::<StaticApi>::from(0u128);
        // let decimals = 0u8;

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .submit(from, to, submission_timestamp, price, decimals)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn submit_with_params_fail(
        &mut self,
        from: ManagedBuffer<StaticApi>,
        to: ManagedBuffer<StaticApi>,
        submission_timestamp: u64,
        price: BigUint<StaticApi>,
        decimals: u8,
        expected_result: ExpectError<'_>
    ) {


        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .submit(from, to, submission_timestamp, price, decimals)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn submit_batch(&mut self) {
        let submissions = MultiValueVec::from(vec![MultiValue5::<ManagedBuffer<StaticApi>, ManagedBuffer<StaticApi>, u64, BigUint<StaticApi>, u8>::from((ManagedBuffer::new_from_bytes(&b""[..]), ManagedBuffer::new_from_bytes(&b""[..]), 0u64, BigUint::<StaticApi>::from(0u128), 0u8))]);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .submit_batch(submissions)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn latest_round_data(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PriceAggregatorProxy)
            .latest_round_data()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        // println!("Result: {result_value:?}");
    }

    async fn latest_price_feed(&mut self) {
        let from = ManagedBuffer::new_from_bytes(&b""[..]);
        let to = ManagedBuffer::new_from_bytes(&b""[..]);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PriceAggregatorProxy)
            .latest_price_feed(from, to)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn latest_price_feed_optional(&mut self) {
        let from = ManagedBuffer::new_from_bytes(&b""[..]);
        let to = ManagedBuffer::new_from_bytes(&b""[..]);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PriceAggregatorProxy)
            .latest_price_feed_optional(from, to)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn set_submission_count(&mut self) {
        let submission_count = 0u32;

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .set_submission_count(submission_count)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn set_submission_count_with_params(&mut self, submission_count: u32) {

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .set_submission_count(submission_count)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn get_oracles(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PriceAggregatorProxy)
            .get_oracles()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn set_pair_decimals(&mut self) {
        let from = ManagedBuffer::new_from_bytes(&b""[..]);
        let to = ManagedBuffer::new_from_bytes(&b""[..]);
        let decimals = 0u8;

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .set_pair_decimals(from, to, decimals)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    
    async fn set_pair_decimals_with_params(&mut self, from: ManagedBuffer<StaticApi>, to: ManagedBuffer<StaticApi>, decimals: u8) {
        // let from = ManagedBuffer::new_from_bytes(&b""[..]);
        // let to = ManagedBuffer::new_from_bytes(&b""[..]);
        // let decimals = 0u8;

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .set_pair_decimals(from, to, decimals)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn get_pair_decimals(&mut self) {
        let from = ManagedBuffer::new_from_bytes(&b""[..]);
        let to = ManagedBuffer::new_from_bytes(&b""[..]);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PriceAggregatorProxy)
            .get_pair_decimals(from, to)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn get_pair_decimals_with_params(&mut self, from: ManagedBuffer<StaticApi>, to: ManagedBuffer<StaticApi>) {
        // let from = ManagedBuffer::new_from_bytes(&b""[..]);
        // let to = ManagedBuffer::new_from_bytes(&b""[..]);

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PriceAggregatorProxy)
            .get_pair_decimals(from, to)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }


    async fn submission_count(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PriceAggregatorProxy)
            .submission_count()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    
    async fn set_submission_count_with_params_fail(&mut self, expected_result: ExpectError<'_>) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PriceAggregatorProxy)
            .submission_count()
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn pause_endpoint(&mut self) {
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .pause_endpoint()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn unpause_endpoint(&mut self) {
        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .unpause_endpoint()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn paused_status(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PriceAggregatorProxy)
            .paused_status()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn stake(&mut self) {
        let token_id = STAKING_TOKEN;
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(1u128);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .stake()
            .payment((TokenIdentifier::from(token_id), token_nonce, token_amount))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn unstake(&mut self) {
        let unstake_amount = BigUint::<StaticApi>::from(0u128);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .unstake(unstake_amount)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn vote_slash_member(&mut self) {
        let member_to_slash = bech32::decode("");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .vote_slash_member(member_to_slash)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn cancel_vote_slash_member(&mut self) {
        let member_to_slash = bech32::decode("");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .cancel_vote_slash_member(member_to_slash)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn slash_member(&mut self) {
        let member_to_slash = bech32::decode("");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::PriceAggregatorProxy)
            .slash_member(member_to_slash)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

}

#[tokio::test]
async fn test_deploy_success(){
    let mut interact = ContractInteract::new().await;
    let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
    let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
    let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
    let first_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS));
    let second_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS));
    let third_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(THIRD_ORACLE_ADDRESS));
    let fourth_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(FOURTH_ORACLE_ADDRESS));
    let oracles: MultiValueVec<ManagedAddress<StaticApi>> = MultiValueVec::from(vec![first_oracle_address, second_oracle_address, third_oracle_address, fourth_oracle_address]);
    interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;
}

// #[tokio::test]
// async fn test_deploy_fail(){
//     let mut interact = ContractInteract::new().await;
//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(30u128);
//     let first_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS));
//     let second_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS));
//     let third_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(THIRD_ORACLE_ADDRESS));
//     let fourth_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(FOURTH_ORACLE_ADDRESS));
//     let oracles: MultiValueVec<ManagedAddress<StaticApi>> = MultiValueVec::from(vec![first_oracle_address, second_oracle_address, third_oracle_address, fourth_oracle_address]);
//     interact.deploy_price_aggregator_fail(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles, ).await;
// }

// #[tokio::test]
// async fn test_deploy_basic_fail(){
//     let mut interact = ContractInteract::new().await;
//     interact.deploy_fail(ExpectError(4, "Slash amount cannot be higher than required stake")).await;
// }
// #[tokio::test]
// async fn test_add_oracles_success(){
//     let mut interact = ContractInteract::new().await;

//     let fifth_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(FIFTH_ORACLE_ADDRESS));
//     let sixth_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(SIXTH_ORACLE_ADDRESS));

//     let oracles: MultiValueVec<ManagedAddress<StaticApi>> = MultiValueVec::from(vec![fifth_oracle_address, sixth_oracle_address]);

//     interact.add_oracles_with_params(oracles.clone()).await;
//     interact.get_oracles().await;

//     interact.add_oracles_with_params(oracles.clone()).await;
//     interact.get_oracles().await;

//     interact.remove_oracles_with_params(oracles, 3u32).await;
//     interact.get_oracles().await;
// }

// #[tokio::test]
// async fn test_submit_succes(){
//     let mut interact = ContractInteract::new().await;
    
//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let first_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS));
//     let second_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS));
//     let third_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(THIRD_ORACLE_ADDRESS));
//     let fourth_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(FOURTH_ORACLE_ADDRESS));
//     let oracles: MultiValueVec<ManagedAddress<StaticApi>> = MultiValueVec::from(vec![first_oracle_address, second_oracle_address, third_oracle_address, fourth_oracle_address]);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     let from = ManagedBuffer::new_from_bytes(FIRST_ORACLE_ADDRESS.as_bytes());
//     let to = ManagedBuffer::new_from_bytes(SECOND_ORACLE_ADDRESS.as_bytes());
//     let submission_timestamp = 1234567890u64;
//     let price = BigUint::<StaticApi>::from(1u128);
//     let decimals = 2u8;

//     interact.unpause_endpoint().await;
//     interact.stake().await;
//     interact.submit_with_params(from, to, submission_timestamp, price, decimals).await;
//     interact.get_oracles().await
    
// }


// #[tokio::test]
// async fn test_submit_fail(){
//     let mut interact = ContractInteract::new().await;
    
//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let first_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS));
//     let second_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS));
//     let third_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(THIRD_ORACLE_ADDRESS));
//     let fourth_oracle_address: ManagedAddress<StaticApi> = ManagedAddress::from(bech32::decode(FOURTH_ORACLE_ADDRESS));
//     let oracles: MultiValueVec<ManagedAddress<StaticApi>> = MultiValueVec::from(vec![first_oracle_address, second_oracle_address, third_oracle_address, fourth_oracle_address]);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     let from = ManagedBuffer::new_from_bytes(FIRST_ORACLE_ADDRESS.as_bytes());
//     let to = ManagedBuffer::new_from_bytes(SECOND_ORACLE_ADDRESS.as_bytes());
//     let submission_timestamp = 1234567890u64;
//     let price = BigUint::<StaticApi>::from(1u128);
//     let decimals = 2u8;

//     interact.unpause_endpoint().await;
//     interact.stake().await;
//     interact.submit_with_params_fail(from, to, submission_timestamp, price, decimals, ExpectError(4, "only oracles allowed")).await;
//     interact.get_oracles().await;
    
// }

// #[tokio::test]
// async fn test_multiple_submissions_success() {
//     let mut interact = ContractInteract::new().await;

//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(THIRD_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     let submissions = vec![
//         (FIRST_ORACLE_ADDRESS, SECOND_ORACLE_ADDRESS, 1234567890u64, BigUint::<StaticApi>::from(100u128), 2u8),
//         (SECOND_ORACLE_ADDRESS, THIRD_ORACLE_ADDRESS, 1234567891u64, BigUint::<StaticApi>::from(200u128), 2u8),
//         (THIRD_ORACLE_ADDRESS, FIRST_ORACLE_ADDRESS, 1234567892u64, BigUint::<StaticApi>::from(300u128), 2u8),
//     ];

//     for (from, to, timestamp, price, decimals) in submissions {
//         let from = ManagedBuffer::new_from_bytes(from.as_bytes());
//         let to = ManagedBuffer::new_from_bytes(to.as_bytes());
//         interact.unpause_endpoint().await;
//         interact.stake().await;
//         interact.submit_with_params(from, to, timestamp, price, decimals).await;
//     }

//     let round_data = interact.latest_round_data().await;
//     // assert_eq!(latest_round_data(), 1, "Should have one round data entry");
// }

// #[tokio::test]
// async fn test_submit_future_timestamp_fail() {
//     let mut interact = ContractInteract::new().await;

//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(THIRD_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(FOURTH_ORACLE_ADDRESS)),
        
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     let future_timestamp = 9999999999u64; // Timestamp in the future
//     let from = ManagedBuffer::new_from_bytes(FIRST_ORACLE_ADDRESS.as_bytes());
//     let to = ManagedBuffer::new_from_bytes(SECOND_ORACLE_ADDRESS.as_bytes());
    
//     let price = BigUint::<StaticApi>::from(100u128);
//     let decimals = 2u8;

//     interact.unpause_endpoint().await;
//     interact.stake().await;

//     interact.submit_with_params_fail(from, to, future_timestamp, price, decimals, ExpectError(4, "Timestamp is from the future")).await;
// }

// #[tokio::test]
// async fn test_set_and_check_pair_decimals() {
//     let mut interact = ContractInteract::new().await;

//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     let from = ManagedBuffer::new_from_bytes(FIRST_ORACLE_ADDRESS.as_bytes());
//     let to = ManagedBuffer::new_from_bytes(SECOND_ORACLE_ADDRESS.as_bytes());
//     let decimals = 8u8;

//     interact.set_pair_decimals_with_params(from.clone(), to.clone(), decimals).await;
//     interact.get_pair_decimals_with_params(from, to).await;

// }

// #[tokio::test]
// async fn test_invalid_submission_count() {
//     let mut interact = ContractInteract::new().await;

//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     let new_submission_count = 2; // Invalid submission count
//     interact.set_submission_count_with_params_fail(new_submission_count, ExpectError(4, "Invalid submission count")).await;
// }

// #[tokio::test]
// async fn test_add_oracles_paused_fail() {
//     let mut interact = ContractInteract::new().await;

//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     interact.pause_endpoint().await; // Pause the contract

//     let new_oracles = vec![
//         ManagedAddress::from(bech32::decode(THIRD_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(FOURTH_ORACLE_ADDRESS)),
//     ];
//     let new_oracles = MultiValueVec::from(new_oracles);

//     interact.add_oracles_with_params_fail(new_oracles, ExpectError(4, "Contract is paused")).await;
// }

// #[tokio::test]
// async fn test_oracle_priority_and_update() {
//     let mut interact = ContractInteract::new().await;

//     // Setup initial contract state
//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(THIRD_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     // Define oracle submissions
//     let submissions = vec![
//         (FIRST_ORACLE_ADDRESS, SECOND_ORACLE_ADDRESS, 1234567890u64, BigUint::<StaticApi>::from(100u128), 2u8),
//         (SECOND_ORACLE_ADDRESS, THIRD_ORACLE_ADDRESS, 1234567891u64, BigUint::<StaticApi>::from(200u128), 2u8),
//         (THIRD_ORACLE_ADDRESS, FIRST_ORACLE_ADDRESS, 1234567892u64, BigUint::<StaticApi>::from(300u128), 2u8),
//     ];

//     for (from, to, timestamp, price, decimals) in submissions {
//         let from = ManagedBuffer::new_from_bytes(from.as_bytes());
//         let to = ManagedBuffer::new_from_bytes(to.as_bytes());
//         interact.unpause_endpoint().await;
//         interact.stake().await;
//         interact.submit_with_params(from, to, timestamp, price, decimals).await;
//     }

//     // Verify that the latest round data is processed correctly
//     let round_data = interact.latest_round_data().await;
//     // Your assert here to check correct processing
// }

// #[tokio::test]
// async fn test_stake_withdrawal() {
//     let mut interact = ContractInteract::new().await;

//     // Setup initial contract state
//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     // Stake and then withdraw
//     interact.unpause_endpoint().await;
//     interact.stake().await;
//     interact.unstake().await;

// }

// #[tokio::test]
// async fn test_modify_contract_parameters() {
//     let mut interact = ContractInteract::new().await;

//     // Setup initial contract state
//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     // Modify contract parameters
//     let new_submission_count = 4;
//     interact.set_submission_count_with_params(new_submission_count).await;

//     // Assert that the submission count is updated correctly
//     // assert_eq!(updated_submission_count, new_submission_count);
// }


// #[tokio::test]
// async fn test_combined_error_conditions() {
//     let mut interact = ContractInteract::new().await;

//     // Setup initial contract state
//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     // Pause the contract
//     interact.pause_endpoint().await;

//     // Test adding oracles while paused
//     let new_oracles = vec![
//         ManagedAddress::from(bech32::decode(THIRD_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(FOURTH_ORACLE_ADDRESS)),
//     ];
//     let new_oracles = MultiValueVec::from(new_oracles);
//     interact.add_oracles_with_params_fail(new_oracles, ExpectError(4, "Contract is paused")).await;

//     // Unpause and test invalid submission
//     interact.unpause_endpoint().await;
//     interact.set_submission_count_with_params_fail(1, ExpectError(4, "Invalid submission count")).await;

//     // Test with future timestamp
//     let future_timestamp = 9999999999u64;
//     let from = ManagedBuffer::new_from_bytes(FIRST_ORACLE_ADDRESS.as_bytes());
//     let to = ManagedBuffer::new_from_bytes(SECOND_ORACLE_ADDRESS.as_bytes());
//     let price = BigUint::<StaticApi>::from(100u128);
//     let decimals = 2u8;
//     interact.submit_with_params_fail(from, to, future_timestamp, price, decimals, ExpectError(4, "Timestamp is from the future")).await;
// }

// #[tokio::test]
// async fn test_oracle_role_handling() {
//     let mut interact = ContractInteract::new().await;

//     // Setup initial contract state
//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(THIRD_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     // Stake and submit data from different oracles
//     let submissions = vec![
//         (FIRST_ORACLE_ADDRESS, SECOND_ORACLE_ADDRESS, 1234567890u64, BigUint::<StaticApi>::from(100u128), 2u8),
//         (SECOND_ORACLE_ADDRESS, THIRD_ORACLE_ADDRESS, 1234567891u64, BigUint::<StaticApi>::from(200u128), 2u8),
//     ];

//     for (from, to, timestamp, price, decimals) in submissions {
//         let from = ManagedBuffer::new_from_bytes(from.as_bytes());
//         let to = ManagedBuffer::new_from_bytes(to.as_bytes());
//         interact.unpause_endpoint().await;
//         interact.stake().await;
//         interact.submit_with_params(from, to, timestamp, price, decimals).await;
//     }

//     // Verify that oracles are correctly identified and their data is processed
//     let round_data = interact.latest_round_data().await;
//     // Your assert here to verify data accuracy based on oracle roles
// }

// #[tokio::test]
// async fn test_concurrent_oracle_submissions() {
//     let mut interact = ContractInteract::new().await;

//     // Setup initial contract state
//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     // Define concurrent submissions
//     let submissions = vec![
//         (FIRST_ORACLE_ADDRESS, SECOND_ORACLE_ADDRESS, 1234567890u64, BigUint::<StaticApi>::from(100u128), 2u8),
//         (SECOND_ORACLE_ADDRESS, FIRST_ORACLE_ADDRESS, 1234567891u64, BigUint::<StaticApi>::from(200u128), 2u8),
//     ];

//     let mut handles = vec![];
//     for (from, to, timestamp, price, decimals) in submissions {
//         let from = ManagedBuffer::new_from_bytes(from.as_bytes());
//         let to = ManagedBuffer::new_from_bytes(to.as_bytes());

//         // let handle = tokio::spawn(async move {
//         //     interact.unpause_endpoint().await;
//         //     interact.stake().await;
//         //     interact.submit_with_params(from, to, timestamp, price, decimals).await;
//         // });

//         // handles.push(handle);
//     }

//     // Await all concurrent submissions
//     for handle in handles {
//         handle.await.unwrap();
//     }

//     // Verify that the data is correctly processed
//     let round_data = interact.latest_round_data().await;
//     // Your assert here to check if the data was processed correctly
// }

// #[tokio::test]
// async fn test_state_manipulation_and_recovery() {
//     let mut interact = ContractInteract::new().await;

//     // Setup initial contract state
//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     // Perform a series of actions
//     interact.unpause_endpoint().await;
//     interact.stake().await;

//     // Perform submissions
//     let submissions = vec![
//         (FIRST_ORACLE_ADDRESS, SECOND_ORACLE_ADDRESS, 1234567890u64, BigUint::<StaticApi>::from(100u128), 2u8),
//         (SECOND_ORACLE_ADDRESS, FIRST_ORACLE_ADDRESS, 1234567891u64, BigUint::<StaticApi>::from(200u128), 2u8),
//     ];

//     for (from, to, timestamp, price, decimals) in submissions {
//         let from = ManagedBuffer::new_from_bytes(from.as_bytes());
//         let to = ManagedBuffer::new_from_bytes(to.as_bytes());
//         interact.submit_with_params(from, to, timestamp, price, decimals).await;
//     }

//     // Pause and then unpause contract to simulate recovery
//     interact.pause_endpoint().await;
//     interact.unpause_endpoint().await;

//     // Verify state consistency
//     let round_data = interact.latest_round_data().await;
//     // Your assert here to verify that the state is consistent
// }

// #[tokio::test]
// async fn test_resource_limit_and_abuse() {
//     let mut interact = ContractInteract::new().await;

//     // Setup initial contract state
//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     // Attempt to perform too many submissions quickly
//     for _ in 0..100 {
//         let from = ManagedBuffer::new_from_bytes(FIRST_ORACLE_ADDRESS.as_bytes());
//         let to = ManagedBuffer::new_from_bytes(SECOND_ORACLE_ADDRESS.as_bytes());
//         let timestamp = 1234567890u64;
//         let price = BigUint::<StaticApi>::from(100u128);
//         let decimals = 2u8;
//         interact.submit_with_params(from.clone(), to.clone(), timestamp, price, decimals).await;
//     }

//     // Check if the contract handles the limit correctly
//     assert!(error_occurred, "Expected error due to resource abuse");
// }

// #[tokio::test]
// async fn test_permissions_and_roles_complex() {
//     let mut interact = ContractInteract::new().await;

//     // Setup initial contract state
//     let staking_token = EgldOrEsdtTokenIdentifier::esdt(STAKING_TOKEN.as_bytes());
//     let staking_amount = BigUint::<StaticApi>::from(STAKING_AMOUNT);
//     let slash_amount = BigUint::<StaticApi>::from(SLASH_AMOUNT);
//     let oracles = vec![
//         ManagedAddress::from(bech32::decode(FIRST_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(SECOND_ORACLE_ADDRESS)),
//         ManagedAddress::from(bech32::decode(THIRD_ORACLE_ADDRESS)),
//     ];
//     let oracles = MultiValueVec::from(oracles);
//     interact.deploy_price_aggregator(staking_token, staking_amount, slash_amount, SLASH_QUORUM, SUBMISSION_COUNT, oracles).await;

//     // Test with valid oracle
//     let from = ManagedBuffer::new_from_bytes(FIRST_ORACLE_ADDRESS.as_bytes());
//     let to = ManagedBuffer::new_from_bytes(SECOND_ORACLE_ADDRESS.as_bytes());
//     let timestamp = 1234567890u64;
//     let price = BigUint::<StaticApi>::from(100u128);
//     let decimals = 2u8;
//     interact.unpause_endpoint().await;
//     interact.stake().await;
//     interact.submit_with_params(from.clone(), to.clone(), timestamp, price, decimals).await;

//     // Test with invalid oracle role
//     let invalid_oracle = ManagedBuffer::new_from_bytes(THIRD_ORACLE_ADDRESS.as_bytes());
//     interact.submit_with_params_fail(from, invalid_oracle, timestamp, price, decimals, ExpectError(4, "Invalid oracle role")).await;

//     // Verify state after permissions test
//     let round_data = interact.latest_round_data().await;
//     // Your assert here to check if data integrity is maintained
// }
