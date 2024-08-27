#![allow(non_snake_case)]

mod potlock_interactor_config;
mod proxy;

use multiversx_sc_snippets::{imports::*, multiversx_sc_scenario::api::VMHooksApi};
use proxy::Pot;
use serde::{de::Expected, Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};

const STATE_FILE: &str = "state.toml";
const TOKEN_ID: &str = "BSK-476470";
// const TOKEN_ID: &str = "INTERNS-c9325f";
const FEE_AMOUNT: u64 = 50000000000000000; // 0.5
const IVAN_ADDRESS: &str= "erd13x29rvmp4qlgn4emgztd8jgvyzdj0p6vn37tqxas3v9mfhq4dy7shalqrx";

use potlock_interactor_config::Config;

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut args = std::env::args();
    let _ = args.next();
    let cmd = args.next().expect("at least one argument required");
    let mut interact = ContractInteract::new().await;
    match cmd.as_str() {
        "deploy" => interact.deploy().await,
        "acceptPot" => interact.accept_pot().await,
        "removePot" => interact.remove_pot().await,
        "acceptApplication" => interact.accept_application().await,
        "rejectDonation" => interact.reject_donation().await,
        "distributePotToProjects" => interact.distribute_pot_to_projects().await,
        "addPot" => interact.add_pot().await,
        "applyForPot" => interact.apply_for_pot().await,
        "donateToPot" => interact.donate_to_pot().await,
        "donateToProject" => interact.donate_to_project().await,
        "changeFeeForPots" => interact.change_fee_for_pots().await,
        "getFeeTokenIdentifier" => interact.fee_token_identifier().await,
        "getFeeAmount" => interact.fee_amount().await,
        "feePotPayments" => interact.fee_pot_proposer().await,
        "feeAmountAcceptPots" => interact.fee_amount_accepted_pots().await,
        "potDonations" => interact.pot_donations().await,
        "projectDonations" => interact.project_donations().await,
        "isAdmin" => interact.is_admin().await,
        "addAdmin" => interact.add_admin().await,
        "removeAdmin" => interact.remove_admin().await,
        "getAdmins" => interact.admins().await,
        _ => panic!("unknown command: {}", &cmd),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    contract_address: Option<Bech32Address>,
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
    state: State,
    config: Config,
}

impl ContractInteract {
    async fn new() -> Self {
        let config = Config::load_config();
        let mut interactor = Interactor::new(config.gateway()).await;
        let wallet_address = interactor.register_wallet(test_wallets::alice());
        let contract_code = BytesValue::interpret_from(
            "mxsc:../output/potlock.mxsc.json",
            &InterpreterContext::default(),
        );

        ContractInteract {
            interactor,
            wallet_address,
            contract_code,
            state: State::load_state(),
            config: Config::load_config(),
        }
    }

    async fn deploy(&mut self) {
        let admin = &self.config.admin;

        let new_address = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .typed(proxy::PotlockProxy)
            .init(admin)
            .code(&self.contract_code)
            .gas(50_000_000)
            .returns(ReturnsNewAddress)
            .prepare_async()
            .run()
            .await;
        let new_address_bech32 = bech32::encode(&new_address);
        self.state.set_address(Bech32Address::from_bech32_string(
            new_address_bech32.clone(),
        ));

        println!("new address: {new_address_bech32}");
    }

    async fn accept_pot(&mut self) {
        let admin = &self.config.admin;
        let potlock_id = 1u32;

        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .accept_pot(potlock_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn accept_pot_fail(&mut self, expected_result: ExpectError<'_>) {
        let admin = &self.config.admin;
        let potlock_id = 99u32;

        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .accept_pot(potlock_id)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn remove_pot(&mut self) {
        let admin = &self.config.admin;
        let potlock_id = 0u32;

        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .remove_pot(potlock_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn remove_pot_with_params(
        &mut self,
        potlock_id: u32,
    ) {
        let admin = &self.config.admin;
        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .remove_pot(potlock_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;
    
        println!("Result: {response:?}");
    }
    

    async fn remove_pot_fail(&mut self,potlock_id: u32, expected_result: ExpectError<'_>) {
        let admin = &self.config.admin;
        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .remove_pot(potlock_id)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }


    async fn accept_application(&mut self) {
        let admin: &Bech32Address = &self.config.admin;
        let project_id = 1u32;

        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .accept_application(project_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn accept_application_with_params(&mut self, project_id: u32, expected_result: ExpectError<'_>) {
        let admin: &Bech32Address = &self.config.admin;

        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .accept_application(project_id)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn reject_donation(&mut self) {
        let admin: &Bech32Address = &self.config.admin;
        let user = &self.config.pot_donor;
        let potlock_id = 1u32;

        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .reject_donation(potlock_id, user)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn reject_donation_with_params(&mut self, potlock_id: u32, expected_result: ExpectError<'_>) {
        let admin: &Bech32Address = &self.config.admin;
        let user = &self.config.pot_donor;

        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .reject_donation(potlock_id, user)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn distribute_pot_to_projects(&mut self) {
        let admin: &Bech32Address = &self.config.admin;
        let potlock_id = 1u32;
        let project_percentage = MultiValueVec::from(vec![MultiValue2::from((1u32, 10_000u64))]);

        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .distribute_pot_to_projects(potlock_id, project_percentage)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn distribute_pot_to_projects_with_params(
        &mut self,
        potlock_id: u32, 
        project_percentage: MultiValueVec<MultiValue2<u32, u64>>,
        expected_result: ExpectError<'_>
    ) {
        let admin: &Bech32Address = &self.config.admin;

        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .distribute_pot_to_projects(potlock_id, project_percentage)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }


    async fn add_pot(&mut self) {
        self.config.pot_proposer = Bech32Address::from_bech32_string(String::from("erd1qyu5wthldzr8wx5c9ucg8kjagg0jfs53s8nr3zpz3hypefsdd8ssycr6th"));
        let pot_proposer: &Bech32Address = &self.config.pot_proposer;
        let token_id = TokenIdentifier::from_esdt_bytes(TOKEN_ID);
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(FEE_AMOUNT);

        let description = ManagedBuffer::new_from_bytes(b"Pot used for testing");
        let name = ManagedBuffer::new_from_bytes(b"My Pot");

        let response = self
            .interactor
            .tx()
            .from(pot_proposer)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .add_pot(name, description)
            .payment((token_id, token_nonce, token_amount))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn add_pot_with_params(
        &mut self,
        token_id_str: String,
        token_nonce: u64,
        token_amount: u128,
        description: String,
        name: String,
    ) {
        let pot_proposer: &Bech32Address = &self.config.pot_proposer;
    
        let token_id = TokenIdentifier::from_esdt_bytes(token_id_str.as_bytes());
        let token_amount = BigUint::from(token_amount);
    
        let description = ManagedBuffer::new_from_bytes(description.as_bytes());
        let name = ManagedBuffer::new_from_bytes(name.as_bytes());
    
        let response = self
            .interactor
            .tx()
            .from(pot_proposer)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .add_pot(name, description)
            .payment((token_id, token_nonce, token_amount))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;
    
        println!("Result: {response:?}");
    }

    async fn add_pot_with_params_fail(
        &mut self,
        token_id_str: String,
        token_nonce: u64,
        token_amount: u128,
        description: String,
        name: String,
        expected_result: ExpectError<'_>

    ) {
        let pot_proposer: &Bech32Address = &self.config.pot_proposer;
    
        let token_id = TokenIdentifier::from_esdt_bytes(token_id_str.as_bytes());
        let token_amount = BigUint::from(token_amount);
    
        let description = ManagedBuffer::new_from_bytes(description.as_bytes());
        let name = ManagedBuffer::new_from_bytes(name.as_bytes());
    
        let response = self
            .interactor
            .tx()
            .from(pot_proposer)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .add_pot(name, description)
            .payment((token_id, token_nonce, token_amount))
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
    
        println!("Result: {response:?}");
    }
    async fn apply_for_pot(&mut self) {
        let project_proposer: &Bech32Address = &self.config.project_proposer;
        let potlock_id = 1u32;
        let project_name = ManagedBuffer::new_from_bytes(b"New Testing Project");
        let description = ManagedBuffer::new_from_bytes(b"Project used for testing");

        let response = self
            .interactor
            .tx()
            .from(project_proposer)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .apply_for_pot(potlock_id, project_name, description)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn apply_for_pot_with_params(&mut self, potlock_id: u32, name: String, description: String) {
        let project_proposer: &Bech32Address = &self.config.project_proposer;
        let project_name = ManagedBuffer::new_from_bytes(name.as_bytes());
        let description = ManagedBuffer::new_from_bytes(description.as_bytes());

        let response = self
            .interactor
            .tx()
            .from(project_proposer)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .apply_for_pot(potlock_id, project_name, description)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }


    async fn donate_to_pot(&mut self) {
        let pot_donor: &Bech32Address = &self.config.pot_donor;
        let token_id = TokenIdentifier::from_esdt_bytes(TOKEN_ID);
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(3 * FEE_AMOUNT);

        let potlock_id = 1u32;

        let response = self
            .interactor
            .tx()
            .from(pot_donor)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .donate_to_pot(potlock_id)
            .payment((token_id, token_nonce, token_amount))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn donate_to_pot_with_params(&mut self, potlock_id: u32, expected_result: ExpectError<'_>) {
        let pot_donor: &Bech32Address = &self.config.pot_donor;
        // pot_donor = Bech32Address::from_bech32_string(String::from("erd1qyu5wthldzr8wx5c9ucg8kjagg0jfs53s8nr3zpz3hypefsdd8ssycr6th"));
        let token_id = TokenIdentifier::from_esdt_bytes(TOKEN_ID);
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(3 * FEE_AMOUNT);

        let response = self
            .interactor
            .tx()
            .from(pot_donor)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .donate_to_pot(potlock_id)
            .payment((token_id, token_nonce, token_amount))
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }
    async fn donate_to_project(&mut self) {
        let project_donor: &Bech32Address = &self.config.project_donor;
        let token_id = TokenIdentifier::from_esdt_bytes(TOKEN_ID);
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(3 * FEE_AMOUNT);

        let project_id = 1u32;

        let response = self
            .interactor
            .tx()
            .from(project_donor)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .donate_to_project(project_id)
            .payment((token_id, token_nonce, token_amount))
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn donate_to_project_with_params(&mut self, project_id: u32, expected_result: ExpectError<'_>) {
        let project_donor: &Bech32Address = &self.config.project_donor;
        let token_id = TokenIdentifier::from_esdt_bytes(TOKEN_ID);
        let token_nonce = 0u64;
        let token_amount = BigUint::<StaticApi>::from(3 * FEE_AMOUNT);


        let response = self
            .interactor
            .tx()
            .from(project_donor)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .donate_to_project(project_id)
            .payment((token_id, token_nonce, token_amount))
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }
    async fn change_fee_for_pots(&mut self) {
        let admin: &Bech32Address = &self.config.admin;
        let token_identifier = TokenIdentifier::from_esdt_bytes(TOKEN_ID);
        let fee = BigUint::<StaticApi>::from(FEE_AMOUNT);

        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .change_fee_for_pots(token_identifier, fee)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn change_fee_for_pots_fail(&mut self, token_id: &str, fee_amount: u64, expected_result: ExpectError<'_>) {
        let admin: &Bech32Address = &self.config.admin;
        let token_identifier = TokenIdentifier::from_esdt_bytes(token_id);
        let fee = BigUint::<StaticApi>::from(fee_amount);
    
        let response = self
            .interactor
            .tx()
            .from(admin)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .change_fee_for_pots(token_identifier, fee)
            .returns(expected_result)
            .prepare_async()
            .run()
            .await;
    
        println!("Result: {response:?}");
    }
    

    async fn fee_token_identifier(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .fee_token_identifier()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn fee_amount(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .fee_amount()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn fee_pot_proposer(&mut self) {
        let potlock_id = 0u32;

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .fee_pot_proposer(potlock_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn fee_amount_accepted_pots(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .fee_amount_accepted_pots()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn pot_donations(&mut self) {
        let project_id = 0u32;

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .pot_donations(project_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn project_donations(&mut self) {
        let project_id = 0u32;

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .project_donations(project_id)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    
    async fn is_admin(&mut self) {
        let address = bech32::decode("");

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .is_admin(address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn add_admin(&mut self) {
        let address = bech32::decode(IVAN_ADDRESS);

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .add_admin(address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn remove_admin(&mut self) {
        let address = bech32::decode("");

        let response = self
            .interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .remove_admin(address)
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {response:?}");
    }

    async fn admins(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .admins()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    async fn get_potlocks(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .potlocks()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        for pot in result_value.iter() {
            println!("Result: {}", pot.name);
        }
    }
    
    
    async fn get_projects(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::PotlockProxy)
            .projects()
            .returns(ReturnsResultUnmanaged)
            .prepare_async()
            .run()
            .await;

        for project in result_value.iter() {
            println!("Result: {}", project.name);
        }
    }
    
}

#[tokio::test]
async fn test_deploy(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;

}

#[tokio::test]
async fn test_change_for_fee_pots_invalid_token(){
    let invalid_token = "INVALID-TOKEN234";
    let fee_amount = 50000000;
    let mut interact = ContractInteract::new().await;
    interact.change_fee_for_pots_fail(invalid_token, fee_amount, ExpectError(4, "Invalid token provided")).await;
}


// #[tokio::test]
// async fn test_change_for_fee_pots_negative_amount(){
//     let fee_amount = 0;
//     let mut interact = ContractInteract::new().await;
//     interact.add_admin().await;
//     interact.change_fee_for_pots_fail(TOKEN_ID, fee_amount, ExpectError(4, "Invalid token provided")).await;
// }

#[tokio::test]
async fn test_change_for_fee_pots_success(){
    let mut interact = ContractInteract::new().await;
    interact.change_fee_for_pots().await;
}
#[tokio::test]
async fn test_accept_pot_id_does_not_exist(){
    let mut interact = ContractInteract::new().await;
    interact.accept_pot_fail(ExpectError(4, "Potlock doesn't exist!")).await;
}

#[tokio::test]
async fn test_accept_pot_success(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;
    interact.change_fee_for_pots().await;
    interact.add_pot().await;
    interact.accept_pot().await;
}

#[tokio::test]
async fn test_remove_pot_id_does_not_exist(){
    let mut interact = ContractInteract::new().await;
    interact.change_fee_for_pots().await;
            let potlock_id = 0u32;
    interact.remove_pot_fail(potlock_id, ExpectError(4, "Potlock doesn't exist!")).await;
}

#[tokio::test]
async fn test_remove_pot_success(){
    let mut interact = ContractInteract::new().await;
    interact.change_fee_for_pots().await;
    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");

    interact.add_pot_with_params(token_id_str, token_nonce, token_amount, description, name).await;
    interact.remove_pot_with_params(1).await;
}

#[tokio::test]
async fn accept_invalid_project_id(){
    let mut interact = ContractInteract::new().await;
    interact.change_fee_for_pots().await;
}

#[tokio::test]
async fn apply_for_pot_success(){
    let mut interact = ContractInteract::new().await;
    interact.change_fee_for_pots().await;
    
    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params(token_id_str, token_nonce, token_amount, description, name).await;

    let potlock_id = 1u32;
    let description = String::from("Project used for testing");
    let name = String::from("My Testing");
    interact.apply_for_pot_with_params(potlock_id, name, description).await;

}

#[tokio::test]
async fn add_for_pot_success(){
    let mut interact = ContractInteract::new().await;
    interact.change_fee_for_pots().await;
    
    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params(token_id_str, token_nonce, token_amount, description, name).await;
}

#[tokio::test]
async fn add_for_pot_fail_amount(){
    let mut interact = ContractInteract::new().await;
    interact.change_fee_for_pots().await;
    
    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 500000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params_fail(token_id_str, token_nonce, token_amount, description, name, ExpectError(4, "Wrong fee amount for creating a pot")).await;
}

#[tokio::test]
async fn add_pot_fail_token(){
    let mut interact = ContractInteract::new().await;
    interact.change_fee_for_pots().await;
    
    let token_id_str = String::from("WEGLD-a28c59");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params_fail(token_id_str, token_nonce, token_amount, description, name, ExpectError(4, "Wrong token identifier for creating a pot!")).await;

}

#[tokio::test]
async fn accept_application_invalid_project_id(){
    let mut interact = ContractInteract::new().await;
    interact.change_fee_for_pots().await;
    
    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params(token_id_str, token_nonce, token_amount, description, name).await;

    let potlock_id = 1u32;
    let description = String::from("Project used for testing");
    let name = String::from("My Testing");
    interact.apply_for_pot_with_params(potlock_id, name, description).await;

    interact.accept_application_with_params(99u32, ExpectError(4, "Project doesn't exist!")).await;

    interact.accept_application().await;
}

#[tokio::test]
async fn reject_donation_invalid_id(){
    let mut interact = ContractInteract::new().await;
    interact.change_fee_for_pots().await;
    
    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params(token_id_str.clone(), token_nonce, token_amount, description.clone(), name.clone()).await;

    interact.reject_donation_with_params(0u32, ExpectError(4, "Potlock doesn't exist!")).await;

    interact.add_pot_with_params(token_id_str, token_nonce, token_amount, description, name).await;

    let potlock_id = 1u32;
    let description = String::from("Project used for testing");
    let name = String::from("My Testing");
    interact.apply_for_pot_with_params(potlock_id, name, description).await;
}

#[tokio::test]
async fn donate_to_pot_invalid(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;
    interact.change_fee_for_pots().await;
    
    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params(token_id_str, token_nonce, token_amount, description, name).await;

    // interact.apply_for_pot_with_params(potlock_id, name, description).await;
    // interact.accept_application().await;

    interact.donate_to_pot_with_params(9u32, ExpectError(4, "Potlock doesn't exist!")).await;
    interact.donate_to_pot_with_params(1u32, ExpectError(4, "Pot is not active!")).await;
}

#[tokio::test]
async fn donate_to_pot_success(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;
    interact.change_fee_for_pots().await;
    
    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params(token_id_str, token_nonce, token_amount, description, name).await;
    interact.accept_pot().await;

    // interact.apply_for_pot_with_params(potlock_id, name, description).await;
    // interact.accept_application().await;

    interact.donate_to_pot_with_params(9u32, ExpectError(4, "Potlock doesn't exist!")).await;
    interact.donate_to_pot().await;
}

#[tokio::test]
async fn donate_to_project_invalid(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;
    interact.change_fee_for_pots().await;

    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params(token_id_str, token_nonce, token_amount, description, name).await;

    let potlock_id = 1u32;
    let description = String::from("Project used for testing");
    let name = String::from("My Testing");
    interact.apply_for_pot_with_params(potlock_id, name, description).await;

    interact.donate_to_project_with_params(99u32, ExpectError(4, "Project doesn't exist!")).await;
    interact.donate_to_project_with_params(1u32, ExpectError(4, "Project is not active!")).await;
}

#[tokio::test]
async fn donate_to_project_success(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;
    interact.change_fee_for_pots().await;
    
    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params(token_id_str, token_nonce, token_amount, description, name).await;
    interact.accept_pot().await;
    let potlock_id = 1u32;
    let description = String::from("Project used for testing");
    let name = String::from("My Testing");
    interact.apply_for_pot_with_params(potlock_id, name, description).await;

    interact.donate_to_pot().await;
}

#[tokio::test]
async fn distribute_pot_to_projects_success(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;
    interact.change_fee_for_pots().await;
    
    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params(token_id_str, token_nonce, token_amount, description, name).await;
    interact.accept_pot().await;

    let potlock_id = 1u32;
    let description = String::from("Project used for testing");
    let name = String::from("My Testing");
    interact.apply_for_pot_with_params(potlock_id, name, description).await;

    interact.donate_to_pot().await;
    interact.distribute_pot_to_projects().await;
}

#[tokio::test]
async fn distribute_pot_to_projects_invalid_id(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;
    interact.change_fee_for_pots().await;

    let project_percentage = MultiValueVec::from(vec![MultiValue2::from((1u32, 10_000u64))]);
    interact.distribute_pot_to_projects_with_params(99u32, project_percentage, ExpectError(4, "Potlock doesn't exist!")).await;
}

#[tokio::test]
async fn distribute_pot_to_projects_fail(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;
    interact.change_fee_for_pots().await;

    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params(token_id_str, token_nonce, token_amount, description, name).await;
    interact.accept_pot().await;

    let potlock_id = 1u32;
    let description = String::from("Project used for testing");
    let name = String::from("My Testing");
    interact.apply_for_pot_with_params(potlock_id, name, description).await;

    interact.donate_to_pot().await;

    let procent_percentage_fail = MultiValueVec::from(vec![MultiValue2::from((1u32, 500_000_000u64))]);
    interact.distribute_pot_to_projects_with_params(1u32, procent_percentage_fail, ExpectError(4, "Total percentages more than 100%")).await;
}

#[tokio::test]
async fn test_view(){
    let mut interact = ContractInteract::new().await;
    interact.deploy().await;
    interact.change_fee_for_pots().await;
    
    let token_id_str = String::from("BSK-476470");
    let token_nonce = 0u64;
    let token_amount = 50000000000000000;
    let description = String::from("Pot used for testing");
    let name = String::from("My Pot");
    interact.add_pot_with_params(token_id_str, token_nonce, token_amount, description, name).await;
    interact.accept_pot().await;
    let potlock_id = 1u32;
    let description = String::from("Project used for testing");
    let name = String::from("My Testing");
    interact.apply_for_pot_with_params(potlock_id, name, description).await;

    interact.donate_to_pot().await;

    interact.get_projects().await;
}