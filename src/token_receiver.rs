use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, ext_contract, serde_json};

use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;

use crate::sale::Sale;
use crate::sale::SaleType;
use crate::*;

const GAS_GET_ACCOUNT_STAKED_BALANCE: Gas = Gas(25_000_000_000_000);
const GAS_ON_GET_ACCOUNT_STAKED_BALANCE: Gas = Gas(25_000_000_000_000);
const NO_DEPOSIT: Balance = 0;

#[ext_contract(ext_staking_pool)]
pub trait ExtStakingPool {
    /// Check the staked balance of the given account.
    fn get_account_staked_balance(&self, account_id: AccountId) -> U128;
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleDeposit {
    pub sale_id: u64,
    /// Optional argument to point to the contract where this user has staked if sale requires this.
    pub staking_contract: Option<AccountId>,
}

impl Contract {
    pub fn internal_ft_on_transfer(
        &mut self,
        token_id: AccountId,
        sender_id: AccountId,
        amount: U128,
        sale_deposit: SaleDeposit,
    ) -> PromiseOrValue<U128> {
        // Check that account is registered.
        let _ = self
            .accounts
            .get(&sender_id)
            .expect("ERR_NOT_REGISTERED_ACCOUNT");
        let sale: Sale = self
            .sales
            .get(&sale_deposit.sale_id)
            .expect("ERR_NO_SALE")
            .into();
        assert_eq!(sale.deposit_token_id, token_id, "ERR_WRONG_TOKEN");
        if sale.sale_type == SaleType::ByAmount{
            assert!(
                sale.collected_amount < sale.max_amount,
                "ERR_SALE_DONE"
            );
        }
        let timestamp = env::block_timestamp();
        assert!(timestamp >= sale.start_date, "ERR_SALE_NOT_STARTED");
        assert!(
            timestamp >= sale.start_date && timestamp <= sale.end_date,
            "ERR_SALE_DONE"
        );

        // Send call to check how much is staked if staking is required.
        if sale.staking_contracts.len() > 0 {
            let staking_contract = sale_deposit
                .staking_contract
                .expect("ERR_MUST_HAVE_STAKING_CONTRACT");
            assert!(
                sale.staking_contracts.contains(&staking_contract),
                "ERR_NOT_WHITELISTED_STAKING_CONTRACT"
            );
            PromiseOrValue::Promise(
                ext_staking_pool::get_account_staked_balance(
                    sender_id.clone(),
                    staking_contract,
                    NO_DEPOSIT,
                    GAS_GET_ACCOUNT_STAKED_BALANCE,
                )
                .then(ext_self::on_get_account_staked_balance(
                    sale_deposit.sale_id,
                    token_id,
                    sender_id,
                    amount,
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_ON_GET_ACCOUNT_STAKED_BALANCE,
                )),
            )
        } else {
            PromiseOrValue::Value(U128(self.internal_sale_deposit(
                sale_deposit.sale_id,
                &token_id,
                &sender_id,
                0,
                amount.0,
            )))
        }
    }
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    /// Callback on receiving tokens by this contract.
    /// Record the AccountSale for given Sale.
    #[allow(unused_variables)]
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let sale_deposit = serde_json::from_str::<SaleDeposit>(&msg).expect("ERR_MSG_WRONG_FORMAT");
        self.internal_ft_on_transfer(
            env::predecessor_account_id(),
            sender_id,
            amount,
            sale_deposit,
        )
    }
}
