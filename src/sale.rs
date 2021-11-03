use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    ext_contract, log, AccountId, Balance, CryptoHash, PromiseError, PromiseOrValue,
    Timestamp,
};

use crate::token_receiver::*;
use crate::*;

const ONE_YOCTO: Balance = 1;
const GAS_NEAR_DEPOSIT: Gas = BASE_GAS;
const GAS_AFTER_FT_ON_TRANSFER_NEAR_DEPOSIT: Gas = Gas(40_000_000_000_000);

#[ext_contract(ext_wrap_near)]
pub trait ExtWrapNear {
    /// Deposit NEAR to mint wNEAR tokens to the predecessor account in this contract.
    fn near_deposit(&mut self);
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleMetadata {
    /// Project name that is going to be on sale.
    pub name: String,
    /// Symbol (ticker) for the token on sale.
    pub symbol: String,
    /// Project description.
    pub description: String,
    /// Link to project smart contract.
    pub smart_contract_url: String,
    /// Project logo.
    pub logo_url: String,
    /// Symbol for output token
    pub output_ticker: String,
    /// Social medias of the project
    pub project_telegram: Option<String>,
    pub project_medium: Option<String>,
    pub project_twitter: Option<String>,
    pub reward_timestamp: Option<Timestamp>,
    pub reward_description: Option<String>,
}

/// Sale information for creating new sale.
#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleInput {
    pub metadata: SaleMetadata,
    /// Set of staking contract that will be checked if user has staked with it.
    /// Empty if staking is not required for this sale.
    pub staking_contracts: Vec<AccountId>,
    /// Minimum NEAR staked in the above staking contract.
    pub min_near_deposit: U128,
    /// Token to sell for.
    pub deposit_token_id: AccountId,
    /// Minimum amount of deposit token.
    pub min_buy: U128,
    /// Maximum amount of deposit token for one account.
    pub max_buy: U128,
    /// Maximum amount that can be collected by the sale.
    pub max_amount: Option<U128>,
    /// Max amount is hard requirement or not.
    /// If true, max_amount must be provided.
    pub hard_max_amount_limit: bool,
    /// Start date of the sale.
    pub start_date: U64,
    /// End date of the sale.
    pub end_date: U64,
    /// Price per a single token in decimals of the deposit token.
    pub price: U128,
    /// Hash of the merkle tree of whitelisted accounts.
    pub whitelist_hash: Option<CryptoHash>,
    /// Limit per transaction
    pub limit_per_transaction: U128,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleOutput {
    pub sale_id: u128,
    pub metadata: SaleMetadata,
    pub staking_contracts: Vec<AccountId>,
    pub min_near_deposit: U128,
    pub deposit_token_id: AccountId,
    pub min_buy: U128,
    pub max_buy: U128,
    pub max_amount: Option<U128>,
    pub hard_max_amount_limit: bool,
    pub start_date: U64,
    pub end_date: U64,
    pub price: U128,
    pub whitelist_hash: Option<CryptoHash>,
    pub limit_per_transaction: U128,
    pub collected_amount: U128,
    pub num_account_sales: u64,
}

/// Sale information.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VSale {
    Current(Sale),
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Sale {
    pub metadata: SaleMetadata,
    pub staking_contracts: Vec<AccountId>,
    pub min_near_deposit: Balance,
    pub deposit_token_id: AccountId,
    pub min_buy: Balance,
    pub max_buy: Balance,
    pub max_amount: Option<Balance>,
    pub hard_max_amount_limit: bool,
    pub start_date: Timestamp,
    pub end_date: Timestamp,
    pub price: Balance,
    pub whitelist_hash: Option<CryptoHash>,
    pub limit_per_transaction: Balance,

    pub collected_amount: Balance,
    pub account_sales: UnorderedMap<AccountId, VSaleAccount>,
    pub sale_id: Balance,
}

impl From<VSale> for Sale {
    fn from(v_sale: VSale) -> Self {
        match v_sale {
            VSale::Current(sale) => sale,
        }
    }
}

impl From<VSale> for SaleOutput {
    fn from(v_sale: VSale) -> Self {
        match v_sale {
            VSale::Current(sale) => SaleOutput {
                sale_id: sale.sale_id,
                metadata: sale.metadata,
                staking_contracts: sale.staking_contracts,
                min_near_deposit: U128(sale.min_near_deposit),
                deposit_token_id: sale.deposit_token_id,
                min_buy: U128(sale.min_buy),
                max_buy: U128(sale.max_buy),
                max_amount: sale.max_amount.map(|amount| U128(amount)),
                hard_max_amount_limit: sale.hard_max_amount_limit,
                start_date: U64(sale.start_date),
                end_date: U64(sale.end_date),
                price: U128(sale.price),
                whitelist_hash: sale.whitelist_hash,
                limit_per_transaction: sale.limit_per_transaction.into(),
                collected_amount: U128(sale.collected_amount),
                num_account_sales: sale.account_sales.keys_as_vector().len(),
            },
        }
    }
}

impl VSale {
    pub fn new(sale_id: u64, sale_input: SaleInput) -> Self {
        Self::Current(Sale {
            sale_id: sale_id.into(),
            metadata: sale_input.metadata,
            staking_contracts: sale_input.staking_contracts,
            min_near_deposit: sale_input.min_near_deposit.0,
            deposit_token_id: sale_input.deposit_token_id,
            min_buy: sale_input.min_buy.0,
            max_buy: sale_input.max_buy.0,
            max_amount: sale_input.max_amount.map(|amount| amount.0),
            hard_max_amount_limit: sale_input.hard_max_amount_limit,
            start_date: sale_input.start_date.0,
            end_date: sale_input.end_date.0,
            price: sale_input.price.0,
            whitelist_hash: sale_input.whitelist_hash,
            limit_per_transaction: sale_input.limit_per_transaction.into(),
            collected_amount: 0,
            account_sales: UnorderedMap::new(StorageKey::AccountSales { sale_id }),
        })
    }
}

/// Account deposits for the a sale.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VSaleAccount {
    Current(SaleAccount),
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleAccount {
    pub amount: U128,
}

impl From<VSaleAccount> for SaleAccount {
    fn from(v_account_sale: VSaleAccount) -> Self {
        match v_account_sale {
            VSaleAccount::Current(account_sale) => account_sale,
        }
    }
}

impl Contract {
    /// Validates deposit and records it for the given user for give sale.
    /// Returns extra amount if sale is already over capacity.
    pub(crate) fn internal_sale_deposit(
        &mut self,
        sale_id: u64,
        token_id: &AccountId,
        sender_id: &AccountId,
        staked_amount: Balance,
        amount: Balance,
    ) -> Balance {
        let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        assert_eq!(&sale.deposit_token_id, token_id, "ERR_WRONG_TOKEN");
        assert!(amount <= sale.limit_per_transaction, "ERR_LIMIT_PER_TX");
        assert!(
            staked_amount >= sale.min_near_deposit,
            "ERR_NOT_ENOUGH_STAKED"
        );
        // TODO: add check for the whitelist hash.
        let deposit_amount = if !sale.hard_max_amount_limit {
            amount
        } else {
            std::cmp::min(
                amount,
                sale.max_amount.expect("ERR_MUST_HAVE_MAX_AMOUNT") - sale.collected_amount,
            )
        };
        let mut account_sale = sale
            .account_sales
            .get(&sender_id)
            .map(|account_sale| account_sale.into())
            .unwrap_or(SaleAccount { amount: U128(0) });
        account_sale.amount = U128(account_sale.amount.0 + deposit_amount);
        assert!(
            sale.max_buy >= account_sale.amount.0 && sale.min_buy <= account_sale.amount.0,
            "ERR_WRONG_AMOUNT"
        );
        sale.account_sales
            .insert(&sender_id, &VSaleAccount::Current(account_sale));
        sale.collected_amount += deposit_amount;
        self.sales.insert(&sale_id, &VSale::Current(sale));
        amount - deposit_amount
    }

    pub(crate) fn internal_finalize_near_deposit(
        &mut self,
        return_amount: Balance,
        sender_id: AccountId,
        deposit_amount: Balance,
    ) -> PromiseOrValue<U128> {
        let wrap_amount = deposit_amount - return_amount;
        if wrap_amount > 0 {
            // Assuming it will succeed
            ext_wrap_near::near_deposit(
                AccountId::new_unchecked(WRAP_NEAR_ACCOUNT.to_string()),
                wrap_amount,
                GAS_NEAR_DEPOSIT,
            );
        }
        if return_amount > 0 {
            Promise::new(sender_id).transfer(return_amount).into()
        } else {
            PromiseOrValue::Value(U128(0))
        }
    }
}

#[near_bindgen]
impl Contract {
    #[payable]
    pub fn deposit_near(&mut self, sale_deposit: SaleDeposit) -> PromiseOrValue<U128> {
        let sender_id = env::predecessor_account_id();
        let token_id = AccountId::new_unchecked(WRAP_NEAR_ACCOUNT.to_string());
        let amount = env::attached_deposit();
        match self.internal_ft_on_transfer(token_id, sender_id.clone(), amount.into(), sale_deposit)
        {
            PromiseOrValue::Promise(promise) => promise
                .then(ext_self::after_ft_on_transfer_near_deposit(
                    sender_id,
                    U128(amount),
                    env::current_account_id(),
                    NO_DEPOSIT,
                    GAS_AFTER_FT_ON_TRANSFER_NEAR_DEPOSIT,
                ))
                .into(),
            PromiseOrValue::Value(value) => {
                self.internal_finalize_near_deposit(value.0, sender_id, amount)
            }
        }
    }

    pub fn create_sale(&mut self, sale: SaleInput) -> u64 {
        assert_eq!(
            self.owner_id,
            env::predecessor_account_id(),
            "ERR_MUST_BE_OWNER"
        );
        assert!(
            !sale.hard_max_amount_limit
                || (sale.hard_max_amount_limit && sale.max_amount.is_some()),
            "ERR_MUST_HAVE_MAX_AMOUNT"
        );
        self.sales
            .insert(&self.num_sales, &VSale::new(self.num_sales, sale));
        let sale_id = self.num_sales;
        self.num_sales += 1;
        sale_id
    }

    pub fn get_num_sales(&self) -> u64 {
        self.num_sales
    }

    pub fn get_sale(&self, sale_id: u64) -> SaleOutput {
        self.sales.get(&sale_id).expect("ERR_NO_SALE").into()
    }

    pub fn get_sales(&self, from_index: u64, limit: u64) -> Vec<SaleOutput> {
        (from_index..std::cmp::min(from_index + limit, self.num_sales))
            .filter_map(|sale_id| self.sales.get(&sale_id).map(|sale| sale.into()))
            .collect()
    }

    pub fn get_sale_accounts(
        &self,
        sale_id: u64,
        from_index: u64,
        limit: u64,
    ) -> Vec<(AccountId, SaleAccount)> {
        let sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        let keys = sale.account_sales.keys_as_vector();
        let values = sale.account_sales.values_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| (keys.get(index).unwrap(), values.get(index).unwrap().into()))
            .collect()
    }

    pub fn get_sale_account(&self, sale_id: u64, account_id: AccountId) -> SaleAccount {
        let sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        sale.account_sales
            .get(&account_id)
            .expect("ERR_NO_ACCOUNT_SALE")
            .into()
    }

    pub fn on_get_account_staked_balance(
        &mut self,
        #[callback] staked_amount: U128,
        sale_id: u64,
        token_id: AccountId,
        sender_id: AccountId,
        deposit_amount: U128,
    ) -> PromiseOrValue<U128> {
        assert_eq!(
            env::predecessor_account_id(),
            env::current_account_id(),
            "ERR_NOT_OWNER"
        );
        log!("{} stake: {}", sender_id, staked_amount.0);
        PromiseOrValue::Value(U128(self.internal_sale_deposit(
            sale_id,
            &token_id,
            &sender_id,
            staked_amount.0,
            deposit_amount.0,
        )))
    }

    #[private]
    pub fn after_ft_on_transfer_near_deposit(
        &mut self,
        #[callback_result] return_amount: Result<U128, PromiseError>,
        sender_id: AccountId,
        deposit_amount: U128,
    ) -> PromiseOrValue<U128> {
        self.internal_finalize_near_deposit(
            return_amount.map(|v| v.0).unwrap_or(0),
            sender_id,
            deposit_amount.0,
        )
    }
}
