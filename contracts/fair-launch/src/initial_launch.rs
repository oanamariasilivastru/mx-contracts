use crate::{common::Percentage, exchange_actions::EndpointInfo};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

mod pair_proxy {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait PairProxy {
        #[payable("*")]
        #[endpoint(swapTokensFixedInput)]
        fn swap_tokens_fixed_input(
            &self,
            token_out: TokenIdentifier,
            amount_out_min: BigUint,
        ) -> EsdtTokenPayment;
    }
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct InitialLaunchBlocks {
    pub start: u64,
    pub end: u64,
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct InitialLaunchInfo<M: ManagedTypeApi> {
    pub account_buy_limit: BigUint<M>,
    pub tx_buy_limit: BigUint<M>,
    pub buy_fee_percentage_start: Percentage,
    pub buy_fee_percentage_end: Percentage,
    pub sell_fee_percentage_start: Percentage,
    pub sell_fee_percentage_end: Percentage,
}

#[multiversx_sc::module]
pub trait InitialLaunchModule:
    crate::common::CommonModule
    + crate::token_info::TokenInfoModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[payable("*")]
    #[endpoint(buyToken)]
    fn buy_token(&self, pair_adddress: ManagedAddress, amount_out_min: BigUint) {
        self.require_initial_launch();
        require!(
            !self.known_contracts(&pair_adddress).is_empty(),
            "Unknown pair"
        );

        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();
        let launch_info = self.initial_launch_info().get();
        let fee_percentage = self.get_fee_percentage(
            launch_info.buy_fee_percentage_start,
            launch_info.buy_fee_percentage_end,
        );
        let take_fee_result = self.take_fees(
            caller,
            ManagedVec::from_single_item(payment),
            ManagedVec::from_single_item(fee_percentage),
        );
        require!(
            !take_fee_result.transfers.is_empty(),
            "Payment amount too small to cover fees"
        );

        let out_token_id = self.get_token_id();
        let received_tokens: EsdtTokenPayment = self
            .pair_proxy(pair_adddress)
            .swap_tokens_fixed_input(out_token_id, amount_out_min)
            .with_esdt_transfer(take_fee_result.transfers.get(0))
            .execute_on_dest_context();

        require!(
            received_tokens.amount <= launch_info.tx_buy_limit,
            "Exceeded tx limit"
        );
        self.total_bought(&take_fee_result.original_caller)
            .update(|total_bought| {
                *total_bought += &received_tokens.amount;

                require!(
                    *total_bought <= launch_info.account_buy_limit,
                    "Total buy amount exceeded"
                );
            });

        self.send().direct_esdt(
            &take_fee_result.original_caller,
            &received_tokens.token_identifier,
            received_tokens.token_nonce,
            &received_tokens.amount,
        );

        // Fee remains in SC or sent to owner?
    }

    #[payable("*")]
    #[endpoint(sellToken)]
    fn sell_token(
        &self,
        pair_adddress: ManagedAddress,
        out_token_id: TokenIdentifier,
        amount_out_min: BigUint,
    ) {
        self.require_initial_launch();
        require!(
            !self.known_contracts(&pair_adddress).is_empty(),
            "Unknown pair"
        );

        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();
        let launch_info = self.initial_launch_info().get();
        let fee_percentage = self.get_fee_percentage(
            launch_info.sell_fee_percentage_start,
            launch_info.sell_fee_percentage_end,
        );
        let take_fee_result = self.take_fees(
            caller,
            ManagedVec::from_single_item(payment),
            ManagedVec::from_single_item(fee_percentage),
        );
        require!(
            !take_fee_result.transfers.is_empty(),
            "Payment amount too small to cover fees"
        );

        let received_tokens: EsdtTokenPayment = self
            .pair_proxy(pair_adddress)
            .swap_tokens_fixed_input(out_token_id, amount_out_min)
            .with_esdt_transfer(take_fee_result.transfers.get(0))
            .execute_on_dest_context();

        self.send().direct_esdt(
            &take_fee_result.original_caller,
            &received_tokens.token_identifier,
            received_tokens.token_nonce,
            &received_tokens.amount,
        );

        // Fee remains in SC or sent to owner?
    }

    fn get_fee_percentage(
        &self,
        fee_percentage_start: Percentage,
        fee_percentage_end: Percentage,
    ) -> Percentage {
        let initial_launch_blocks = self.initial_launch_blocks().get();
        let current_block = self.blockchain().get_block_nonce();
        require!(
            current_block <= initial_launch_blocks.end,
            "Invalid buy/sell block"
        );

        let blocks_passed_in_penalty_phase = current_block - initial_launch_blocks.start;
        let blocks_diff = initial_launch_blocks.end - initial_launch_blocks.start;
        let percentage_diff = fee_percentage_start - fee_percentage_end;

        let penalty_percentage_decrease =
            percentage_diff as u64 * blocks_passed_in_penalty_phase / (blocks_diff - 1);

        fee_percentage_start - penalty_percentage_decrease as u32
    }

    fn require_not_initial_launch(&self) {
        let current_block = self.blockchain().get_block_nonce();
        let initial_launch_blocks = self.initial_launch_blocks().get();
        require!(
            current_block > initial_launch_blocks.end,
            "Cannot call this endpoint during initial launch"
        );
    }

    fn require_initial_launch(&self) {
        let current_block = self.blockchain().get_block_nonce();
        let initial_launch_blocks = self.initial_launch_blocks().get();
        require!(
            current_block <= initial_launch_blocks.end,
            "Cannot call this endpoint, initial launch period passed"
        );
    }

    #[storage_mapper("initialLaunchBlocks")]
    fn initial_launch_blocks(&self) -> SingleValueMapper<InitialLaunchBlocks>;

    #[storage_mapper("initialLaunchInfo")]
    fn initial_launch_info(&self) -> SingleValueMapper<InitialLaunchInfo<Self::Api>>;

    #[storage_mapper("totalBought")]
    fn total_bought(&self, user_addr: &ManagedAddress) -> SingleValueMapper<BigUint>;

    #[storage_mapper("knownContracts")]
    fn known_contracts(
        &self,
        sc_addr: &ManagedAddress,
    ) -> SingleValueMapper<ManagedVec<EndpointInfo<Self::Api>>>;

    #[proxy]
    fn pair_proxy(&self, to: ManagedAddress) -> pair_proxy::Proxy<Self::Api>;
}
