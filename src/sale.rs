use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_sdk::{
    AccountId, Balance, CryptoHash, ext_contract, log, PromiseError, PromiseOrValue, PromiseResult,
    Timestamp,
};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::{U128, U64};
use near_sdk::serde::{Deserialize, Serialize};

use crate::*;
use crate::token_receiver::*;

const ONE_YOCTO: Balance = 1;
const GAS_NEAR_DEPOSIT: Gas = BASE_GAS;
const GAS_AFTER_FT_ON_TRANSFER_NEAR_DEPOSIT: Gas = Gas(40_000_000_000_000);
const GAS_FOR_FT_TRANSFER: Gas = Gas(10_000_000_000_000);
const GAS_FOR_AFTER_FT_TRANSFER: Gas = Gas(10_000_000_000_000);

uint::construct_uint! {
    pub struct U256(4);
}

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
    /// Is claim available?
    pub claim_available: bool,
    /// Is refund available
    pub refund_available: bool,
    /// Token for sale
    pub distribute_token_id: Option<AccountId>,
    /// Number of decimals of token for sale, used to calculate purchase amount
    pub distribute_token_decimals: Option<u8>,
    /// Minimum amount of deposit token.
    pub min_buy: U128,
    /// Maximum amount of deposit token for one account.
    pub max_buy: U128,
    /// Maximum amount that can be collected by the sale. A.k.a. target_amount
    pub max_amount: U128,
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
    /// Sale Type
    pub sale_type: SaleType
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleOutput {
    pub sale_id: Option<u64>,
    pub metadata: SaleMetadata,
    pub staking_contracts: Vec<AccountId>,
    pub min_near_deposit: U128,
    pub deposit_token_id: AccountId,
    pub claim_available: bool,
    pub refund_available: bool,
    pub distribute_token_id: Option<AccountId>,
    pub distribute_token_decimals: Option<u8>,
    pub distribute_supply_amount: Option<U128>,
    pub min_buy: U128,
    pub max_buy: U128,
    pub max_amount: U128,
    pub hard_max_amount_limit: bool,
    pub start_date: U64,
    pub end_date: U64,
    pub price: U128,
    pub whitelist_hash: Option<CryptoHash>,
    pub limit_per_transaction: U128,
    pub collected_amount: U128,
    pub num_account_sales: u64,
    pub sale_type: SaleType,
    pub claim_begun: bool
}

/// Sale information.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VSale {
    First(SaleOld),
    Current(Sale),
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq)]
#[serde(crate = "near_sdk::serde")]
pub enum SaleType {
    /// First buyers receive tokens. Sale stops when collected_amount reached
    ByAmount,
    /// Unlimited purchase, proportional distribution. Sale stops when end_date reached
    BySubscription,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SaleOld {
    pub metadata: SaleMetadata,
    pub staking_contracts: Vec<AccountId>,
    pub min_near_deposit: Balance,
    pub deposit_token_id: AccountId,
    pub min_buy: Balance,
    pub max_buy: Balance,
    pub max_amount: Balance,
    pub hard_max_amount_limit: bool,
    pub start_date: Timestamp,
    pub end_date: Timestamp,
    pub price: Balance,
    pub whitelist_hash: Option<CryptoHash>,
    pub limit_per_transaction: Balance,

    pub collected_amount: Balance,
    pub account_sales: UnorderedMap<AccountId, VSaleAccount>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Sale {
    pub metadata: SaleMetadata,
    pub staking_contracts: Vec<AccountId>,
    pub min_near_deposit: Balance,
    pub deposit_token_id: AccountId,
    pub claim_available: bool,
    pub refund_available: bool,
    pub distribute_token_id: Option<AccountId>,
    pub distribute_token_decimals: Option<u8>,
    /// Total amount of supply (in tokens for distribution). Only for sale_type: Subscription
    pub distribute_supply_amount: Option<Balance>,
    pub min_buy: Balance,
    pub max_buy: Balance,
    /// target_amount
    pub max_amount: Balance,
    pub hard_max_amount_limit: bool,
    pub start_date: Timestamp,
    pub end_date: Timestamp,
    pub price: Balance,
    pub whitelist_hash: Option<CryptoHash>,
    pub limit_per_transaction: Balance,

    pub collected_amount: Balance,
    pub account_sales: UnorderedMap<AccountId, VSaleAccount>,
    pub account_affiliate_rewards: UnorderedMap<AccountId, VAffiliateRewardAccount>,
    pub sale_type: SaleType,
    pub claim_begun: bool
}

impl From<VSale> for Sale {
    fn from(v_sale: VSale) -> Self {
        match v_sale {
            VSale::First(sale) => Sale {
                metadata: sale.metadata,
                staking_contracts: sale.staking_contracts,
                min_near_deposit: sale.min_near_deposit,
                deposit_token_id: sale.deposit_token_id,
                claim_available: false,
                refund_available: false,
                distribute_token_id: None,
                distribute_token_decimals: None,
                distribute_supply_amount: None,
                min_buy: sale.min_buy,
                max_buy: sale.max_buy,
                max_amount: sale.max_amount,
                hard_max_amount_limit: sale.hard_max_amount_limit,
                start_date: sale.start_date,
                end_date: sale.end_date,
                price: sale.price,
                whitelist_hash: sale.whitelist_hash,
                limit_per_transaction: sale.limit_per_transaction,
                collected_amount: sale.collected_amount,
                account_sales: sale.account_sales,
                account_affiliate_rewards: UnorderedMap::new(StorageKey::AccountAffiliateRewards { sale_id: 0 }),
                sale_type: SaleType::ByAmount,
                claim_begun: false,
            },
            VSale::Current(sale) => sale,
        }
    }
}

impl From<VSale> for SaleOutput {
    fn from(v_sale: VSale) -> Self {
        match v_sale {
            VSale::First(sale) => SaleOutput {
                sale_id: None,
                metadata: sale.metadata,
                staking_contracts: sale.staking_contracts,
                min_near_deposit: U128(sale.min_near_deposit),
                deposit_token_id: sale.deposit_token_id,
                claim_available: false,
                refund_available: false,
                distribute_token_id: None,
                distribute_token_decimals: None,
                distribute_supply_amount: None,
                min_buy: U128(sale.min_buy),
                max_buy: U128(sale.max_buy),
                max_amount: U128(sale.max_amount),
                hard_max_amount_limit: sale.hard_max_amount_limit,
                start_date: U64(sale.start_date),
                end_date: U64(sale.end_date),
                price: U128(sale.price),
                whitelist_hash: sale.whitelist_hash,
                limit_per_transaction: sale.limit_per_transaction.into(),
                collected_amount: U128(sale.collected_amount),
                num_account_sales: sale.account_sales.keys_as_vector().len(),
                sale_type: SaleType::ByAmount,
                claim_begun: false
            },
            VSale::Current(sale) => SaleOutput {
                sale_id: None,
                metadata: sale.metadata,
                staking_contracts: sale.staking_contracts,
                min_near_deposit: U128(sale.min_near_deposit),
                deposit_token_id: sale.deposit_token_id,
                claim_available: sale.claim_available,
                refund_available: sale.refund_available,
                distribute_token_id: sale.distribute_token_id,
                distribute_token_decimals: sale.distribute_token_decimals,
                distribute_supply_amount: Some(U128(sale.distribute_supply_amount.unwrap_or(0))),
                min_buy: U128(sale.min_buy),
                max_buy: U128(sale.max_buy),
                max_amount: U128(sale.max_amount),
                hard_max_amount_limit: sale.hard_max_amount_limit,
                start_date: U64(sale.start_date),
                end_date: U64(sale.end_date),
                price: U128(sale.price),
                whitelist_hash: sale.whitelist_hash,
                limit_per_transaction: sale.limit_per_transaction.into(),
                collected_amount: U128(sale.collected_amount),
                num_account_sales: sale.account_sales.keys_as_vector().len(),
                sale_type: sale.sale_type,
                claim_begun: sale.claim_begun
            },
        }
    }
}

impl VSale {
    pub fn new(sale_id: u64, sale_input: SaleInput) -> Self {
        let distribute_supply_amount: Option<Balance> = if sale_input.sale_type == SaleType::BySubscription {
            Some((
                U256::from(sale_input.price.0)
                    * U256::from(sale_input.max_amount.0)
                    / U256::from(u128::pow(10, sale_input.distribute_token_decimals.unwrap() as u32))
            ).as_u128())
        } else {
            None
        };
        Self::Current(Sale {
            metadata: sale_input.metadata,
            staking_contracts: sale_input.staking_contracts,
            min_near_deposit: sale_input.min_near_deposit.0,
            deposit_token_id: sale_input.deposit_token_id,
            claim_available: sale_input.claim_available,
            refund_available: sale_input.refund_available,
            distribute_token_id: sale_input.distribute_token_id,
            distribute_token_decimals: sale_input.distribute_token_decimals,
            distribute_supply_amount,
            min_buy: sale_input.min_buy.0,
            max_buy: sale_input.max_buy.0,
            max_amount: sale_input.max_amount.0,
            hard_max_amount_limit: sale_input.hard_max_amount_limit,
            start_date: sale_input.start_date.0,
            end_date: sale_input.end_date.0,
            price: sale_input.price.0,
            whitelist_hash: sale_input.whitelist_hash,
            limit_per_transaction: sale_input.limit_per_transaction.into(),
            collected_amount: 0,
            account_sales: UnorderedMap::new(StorageKey::AccountSales { sale_id }),
            account_affiliate_rewards: UnorderedMap::new(StorageKey::AccountAffiliateRewards { sale_id }),
            sale_type: sale_input.sale_type,
            claim_begun: false
        })
    }
}

/// Account deposits for the a sale.
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VSaleAccount {
    First(SaleAccountOld),
    Current(SaleAccount),
}

/// Affiliate rewards for the a sale
#[derive(BorshSerialize, BorshDeserialize)]
pub enum VAffiliateRewardAccount {
    Current(AffiliateRewardAccount),
}


#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleAccountOld {
    pub amount: U128,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct SaleAccount {
    pub amount: U128,
    pub amount_to_claim: U128,
    pub claimed: U128,
    pub refund: U128,
    pub refunded: U128,
}

impl From<VSaleAccount> for SaleAccount {
    fn from(v_account_sale: VSaleAccount) -> Self {
        match v_account_sale {
            VSaleAccount::Current(account_sale) => account_sale,
            VSaleAccount::First(account_sale) => SaleAccount {
                amount: account_sale.amount,
                amount_to_claim: U128(0),
                claimed: U128(0),
                refund: U128(0),
                refunded: U128(0),
            },
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct AffiliateRewardAccount {
    pub amount: U128,
    pub claimed: U128,
}

impl From<VAffiliateRewardAccount> for AffiliateRewardAccount {
    fn from(v_account_affiliate_reward: VAffiliateRewardAccount) -> Self {
        match v_account_affiliate_reward {
            VAffiliateRewardAccount::Current(account_affiliate_reward) => account_affiliate_reward
        }
    }
}

impl Contract {
    fn get_sale_output(sale: VSale, sale_id: u64) -> SaleOutput {
        let mut output: SaleOutput = sale.into();
        output.sale_id = Some(sale_id);
        output
    }

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
                sale.max_amount - sale.collected_amount,
            )
        };
        let mut account_sale = sale
            .account_sales
            .get(sender_id)
            .map(|account_sale| account_sale.into())
            .unwrap_or(SaleAccount {
                amount: U128(0),
                amount_to_claim: U128(0),
                claimed: U128(0),
                refund: U128(0),
                refunded: U128(0),
            });
        account_sale.amount = U128(account_sale.amount.0 + deposit_amount);
        assert!(
            sale.max_buy >= account_sale.amount.0 && sale.min_buy <= account_sale.amount.0,
            "ERR_WRONG_AMOUNT"
        );

        let fees = self.referral_fees.clone();
        if let Some(referrer_v_account_1) = self.accounts.get(sender_id) {
            let referrer_account_1: Account = referrer_v_account_1.into();
            let reward_1 = deposit_amount * fees[0] as u128 / REFERRAL_FEE_DENOMINATOR;
            self.internal_insert_affiliate(&mut sale, &referrer_account_1.referrer, reward_1);
            if let Some(referrer_v_account_2) = self.accounts.get(&referrer_account_1.referrer) {
                let referrer_account_2: Account = referrer_v_account_2.into();
                let reward_2 = deposit_amount * fees[1] as u128 / REFERRAL_FEE_DENOMINATOR;
                self.internal_insert_affiliate(&mut sale, &referrer_account_2.referrer, reward_2);
                if let Some(referrer_v_account_3) = self.accounts.get(&referrer_account_2.referrer) {
                    let referrer_account_3: Account = referrer_v_account_3.into();
                    let reward_3 = deposit_amount * fees[2] as u128 / REFERRAL_FEE_DENOMINATOR;
                    self.internal_insert_affiliate(&mut sale, &referrer_account_3.referrer, reward_3);
                }
            }
        }

        sale.account_sales.insert(sender_id, &VSaleAccount::Current(account_sale));
        sale.collected_amount += deposit_amount;
        self.sales.insert(&sale_id, &VSale::Current(sale));
        amount - deposit_amount
    }

    pub(crate) fn internal_insert_affiliate(&mut self, sale: &mut Sale, account_id: &AccountId, amount: u128) {
        let account_affiliate_reward =
            if let Some(v_account_affiliate_reward) = sale.account_affiliate_rewards.get(account_id) {
                let mut account_affiliate_reward: AffiliateRewardAccount = v_account_affiliate_reward.into();
                account_affiliate_reward.amount = U128::from(account_affiliate_reward.amount.0 + amount);
                account_affiliate_reward
            } else {
                AffiliateRewardAccount {
                    amount: U128::from(amount),
                    claimed: U128::from(0),
                }
            };

        sale.account_affiliate_rewards.insert(account_id, &VAffiliateRewardAccount::Current(account_affiliate_reward));
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
    pub fn get_referrals(&self, account_id: AccountId) -> Vec<AccountId> {
        let mut referrals = [].to_vec();

        if let Some(referrer_v_account_1) = self.accounts.get(&account_id) {
            let referrer_account_1: Account = referrer_v_account_1.into();
            referrals.push(referrer_account_1.referrer.clone());
            if let Some(referrer_v_account_2) = self.accounts.get(&referrer_account_1.referrer) {
                let referrer_account_2: Account = referrer_v_account_2.into();
                referrals.push(referrer_account_2.referrer.clone());
                if let Some(referrer_v_account_3) = self.accounts.get(&referrer_account_2.referrer) {
                    let referrer_account_3: Account = referrer_v_account_3.into();
                    referrals.push(referrer_account_3.referrer);
                }
            }
        }

        referrals
    }

    pub fn get_affiliates(&self, account_id: AccountId) -> (Vec<AccountId>, Vec<AccountId>, Vec<AccountId>) {
        let account: Account = self.accounts.get(&account_id).expect("ERR_NO_ACCOUNT").into();
        (
            internal_get_affiliates_vector(&account.affiliates, 0),
            internal_get_affiliates_vector(&account.affiliates, 1),
            internal_get_affiliates_vector(&account.affiliates, 2),
        )
    }

    pub fn get_sale_amount(&self, sale_id: u64, account_id: AccountId) -> U128 {
        let sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        if let Some(sale_account) = sale.account_sales.get(&account_id) {
            let sale_account: SaleAccount = sale_account.into();
            match sale.sale_type {
                SaleType::ByAmount => sale_account.amount,
                SaleType::BySubscription => {
                    U128::from(
                        get_amount_by_subscription(sale_account.amount.0, sale.collected_amount, sale.distribute_supply_amount.expect("ERR_MUST_HAVE_SUPPLY_AMOUNT"))
                    )
                }
            }
        } else {
            U128::from(0)
        }
    }

    fn internal_calculate_purchase(&mut self, sale_id: u64) {
        let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();

        let account_id = env::predecessor_account_id();
        let distribute_token_decimals = sale.distribute_token_decimals.expect("ERR_NO_TOKEN_DECIMALS");

        if let Some(v_sale_account) = sale.account_sales.get(&account_id) {
            let mut account_sale: SaleAccount = v_sale_account.into();

            assert_ne!(account_sale.amount.0, 0, "ERR_NO_ALLOCATION");

            let deposit_amount = account_sale.amount.0;

            let amount_to_claim: u128 =
                if account_sale.claimed.0 == 0 {
                    let total_amount_to_claim: u128 = (
                        U256::from(u128::pow(10, distribute_token_decimals as u32))
                            * U256::from(deposit_amount)
                            / U256::from(sale.price)
                    ).as_u128();

                    let total_filled_amount: u128 = (
                        U256::from(u128::pow(10, distribute_token_decimals as u32))
                            * U256::from(sale.collected_amount)
                            / U256::from(sale.price)
                    ).as_u128();

                    match sale.sale_type {
                        SaleType::ByAmount => total_amount_to_claim,
                        SaleType::BySubscription => {
                            if sale.max_amount >= sale.collected_amount {
                                total_amount_to_claim
                            } else {
                                get_amount_by_subscription(total_amount_to_claim, total_filled_amount, sale.distribute_supply_amount.expect("ERR_MUST_HAVE_SUPPLY_AMOUNT"))
                            }
                        }
                    }
                } else {
                    account_sale.claimed.0
                };

            if account_sale.amount_to_claim.0 == 0 && amount_to_claim > 0 {
                account_sale.amount_to_claim = U128(amount_to_claim);
            }

            if sale.sale_type == SaleType::BySubscription {
                let client_purchase_amount: u128 = (
                    U256::from(amount_to_claim)
                        * U256::from(sale.price)
                        / U256::from(u128::pow(10, distribute_token_decimals as u32))
                ).as_u128();

                if account_sale.refund.0 == 0 && deposit_amount > client_purchase_amount {
                    account_sale.refund = U128(deposit_amount - client_purchase_amount);
                }
            }

            sale.account_sales.insert(&account_id, &VSaleAccount::Current(account_sale));
            self.sales.insert(&sale_id, &VSale::Current(sale));
        } else {
            panic!("ERR_NO_DATA");
        }
    }

    pub fn claim_purchase(&mut self, sale_id: u64) -> Promise {
        let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        assert!(sale.claim_available, "ERR_CLAIM_NOT_AVAILABLE");
        assert_ne!(sale.price, 0, "ERR_NO_SALE_PRICE");
        assert!(env::block_timestamp() > sale.end_date, "ERR_SALE_IN_PROGRESS");


        let distribute_token_id = sale.distribute_token_id.clone().expect("ERR_NO_TOKEN_ID");

        let account_id = env::predecessor_account_id();

        self.internal_calculate_purchase(sale_id);

        if let Some(v_sale_account) = sale.account_sales.get(&account_id) {
            let mut account_sale: SaleAccount = v_sale_account.into();

            assert_eq!(account_sale.claimed.0, 0, "ERR_ALREADY_CLAIMED");

            let amount_to_claim = account_sale.amount_to_claim;
            assert_ne!(amount_to_claim.0, 0, "ERR_NOTHING_TO_CLAIM");
            account_sale.claimed = amount_to_claim;

            if !sale.claim_begun {
                sale.claim_begun = true;
            }

            log!("Amount to claim: {}", amount_to_claim.0);

            sale.account_sales.insert(&account_id, &VSaleAccount::Current(account_sale));
            self.sales.insert(&sale_id, &VSale::Current(sale));

            self.withdraw_purchase(account_id,
                                   amount_to_claim.0,
                                   distribute_token_id,
                                   sale_id)
        }
        else {
            panic!("ERR_NO_DATA")
        }
    }

    pub fn claim_refund(&mut self, sale_id: u64) -> Promise {
        let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        assert!(sale.sale_type == SaleType::BySubscription, "ERR_REFUND_NOT_ALLOWED");
        assert!(sale.refund_available, "ERR_REFUND_NOT_AVAILABLE");
        assert!(env::block_timestamp() > sale.end_date, "ERR_SALE_IN_PROGRESS");

        let account_id = env::predecessor_account_id();

        self.internal_calculate_purchase(sale_id);

        if let Some(v_sale_account) = sale.account_sales.get(&account_id) {
            let mut account_sale: SaleAccount = v_sale_account.into();

            assert_ne!(account_sale.refund.0, 0, "ERR_NOTHING_TO_REFUND");
            assert_eq!(account_sale.refunded.0, 0, "ERR_ALREADY_REFUNDED");
            assert_ne!(account_sale.claimed.0, 0, "ERR_MUST_CLAIM_BEFORE_REFUND");

            let amount_to_refund: U128 = account_sale.refund;
            account_sale.refunded = amount_to_refund;

            log!("Amount to refund: {}", amount_to_refund.0);
            let token_account_id = sale.deposit_token_id.clone();

            sale.account_sales
                .insert(&account_id, &VSaleAccount::Current(account_sale));
            self.sales.insert(&sale_id, &VSale::Current(sale));

            self.refund_purchase(account_id,
                                 amount_to_refund.0,
                                 token_account_id,
                                 sale_id)
        } else {
            panic!("ERR_NO_DATA");
        }
    }

    pub fn claim_affiliate_reward(&mut self, sale_id: u64) -> Promise {
        let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        let distribute_token_decimals = sale.distribute_token_decimals.expect("ERR_NO_TOKEN_DECIMALS");
        let account_id = env::predecessor_account_id();

        assert!(sale.refund_available, "ERR_NOT_AVAILABLE");
        assert!(sale.sale_type == SaleType::BySubscription && sale.max_amount < sale.collected_amount, "SALE_BY_SUBSCRIPTION_FAILED");
        assert!(env::block_timestamp() > sale.end_date, "ERR_SALE_IN_PROGRESS");

        if let Some(v_sale_account) = sale.account_affiliate_rewards.get(&account_id) {
            let mut account_affiliate_reward: AffiliateRewardAccount = v_sale_account.into();

            assert_ne!(account_affiliate_reward.amount.0, 0, "ERR_NOTHING_TO_CLAIM");
            assert_eq!(account_affiliate_reward.claimed.0, 0, "ERR_ALREADY_CLAIMED");

            let deposit_amount = account_affiliate_reward.amount.0;

            let total_amount_to_claim: u128 = (
                U256::from(u128::pow(10, distribute_token_decimals as u32))
                    * U256::from(deposit_amount)
                    / U256::from(sale.price)
            ).as_u128();

            let total_filled_amount: u128 = (
                U256::from(u128::pow(10, distribute_token_decimals as u32))
                    * U256::from(sale.collected_amount)
                    / U256::from(sale.price)
            ).as_u128();

            let amount_to_claim: u128 = match sale.sale_type {
                SaleType::ByAmount => total_amount_to_claim,
                SaleType::BySubscription => {
                    get_amount_by_subscription(total_amount_to_claim, total_filled_amount, sale.distribute_supply_amount.expect("ERR_MUST_HAVE_SUPPLY_AMOUNT"))
                }
            };

            assert_ne!(amount_to_claim, 0, "ERR_NOTHING_TO_CLAIM");
            log!("Amount to claim: {}", amount_to_claim);

            account_affiliate_reward.claimed = U128(amount_to_claim);

            let deposit_token_id = sale.deposit_token_id.clone();

            sale.account_affiliate_rewards.insert(&account_id, &VAffiliateRewardAccount::Current(account_affiliate_reward));
            self.sales.insert(&sale_id, &VSale::Current(sale));

            self.withdraw_affiliate_reward(account_id, amount_to_claim, deposit_token_id, sale_id)
        } else {
            panic!("ERR_NO_DATA");
        }
    }

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
                || (sale.hard_max_amount_limit && sale.max_amount.0 > 0),
            "ERR_MUST_HAVE_MAX_AMOUNT"
        );

        assert!(sale.distribute_token_decimals.expect("ERR_NO_TOKEN_DECIMALS") > 0, "WRONG_DECIMALS");

        self.sales
            .insert(&self.num_sales, &VSale::new(self.num_sales, sale));
        let sale_id = self.num_sales;
        self.num_sales += 1;
        sale_id
    }

    #[private]
    pub fn remove_sale(&mut self, sale_id: u64) {
        let sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        assert_eq!(sale.collected_amount, 0, "SALE_NOT_EMPTY");
        self.sales.remove(&sale_id);
    }

    #[private]
    pub fn update_referral_fees(&mut self, referral_fees: Vec<u64>) {
        assert_eq!(referral_fees.len(), 3, "WRONG_LENGTH");
        self.referral_fees = referral_fees;
    }

    #[private]
    pub fn update_sale_dates(&mut self, sale_id: u64, start_date: U64, end_date: U64) {
        let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        assert!(
            sale.collected_amount < sale.max_amount,
            "ERR_SALE_DONE"
        );
        sale.start_date = start_date.into();
        sale.end_date = end_date.into();
        self.sales.insert(&sale_id, &VSale::Current(sale));
    }

    #[private]
    pub fn update_sale_distribute_token_id(&mut self, sale_id: u64, distribute_token_id: AccountId) {
        let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        assert!(sale.distribute_token_id.is_none(), "ERR_ALREADY_SET");
        sale.distribute_token_id = Some(distribute_token_id);
        self.sales.insert(&sale_id, &VSale::Current(sale));
    }

    #[private]
    pub fn update_sale_price(&mut self, sale_id: u64, price: U128) {
        let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        assert!(!sale.claim_begun, "ERR_CLAIM_ALREADY_BEGUN");
        let timestamp = env::block_timestamp();
        assert!(
            timestamp < sale.start_date || timestamp > sale.end_date,
            "ERR_SALE_IS_ACTIVE"
        );
        sale.price = price.0;
        self.sales.insert(&sale_id, &VSale::Current(sale));
    }

    #[private]
    pub fn update_sale_distribute_token_decimals(&mut self, sale_id: u64, distribute_token_decimals: u8) {
        let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        assert!(sale.distribute_token_decimals.is_none(), "ERR_ALREADY_SET");
        sale.distribute_token_decimals = Some(distribute_token_decimals);
        self.sales.insert(&sale_id, &VSale::Current(sale));
    }

    #[private]
    pub fn update_sale_claim_available(&mut self, sale_id: u64, claim_available: bool) {
        let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        assert!(sale.distribute_token_id.is_some(), "ERR_NOT_ENOUGH_DATA");
        assert!(sale.distribute_token_decimals.is_some(), "ERR_NOT_ENOUGH_DATA");
        sale.claim_available = claim_available;
        self.sales.insert(&sale_id, &VSale::Current(sale));
    }

    #[private]
    pub fn update_sale_refund_available(&mut self, sale_id: u64, refund_available: bool) {
        let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        assert!(sale.distribute_token_id.is_some(), "ERR_NOT_ENOUGH_DATA");
        assert!(sale.distribute_token_decimals.is_some(), "ERR_NOT_ENOUGH_DATA");
        sale.refund_available = refund_available;
        self.sales.insert(&sale_id, &VSale::Current(sale));
    }


    pub fn get_num_sales(&self) -> u64 {
        self.num_sales
    }

    pub fn get_sale(&self, sale_id: u64) -> SaleOutput {
        Contract::get_sale_output(self.sales.get(&sale_id).expect("ERR_NO_SALE"), sale_id)
    }

    pub fn get_sales(&self, from_index: u64, limit: u64) -> Vec<SaleOutput> {
        (from_index..std::cmp::min(from_index + limit, self.num_sales))
            .filter_map(|sale_id| self.sales.get(&sale_id).map(|sale| Contract::get_sale_output(sale, sale_id)))
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
        if let Some(sale_account) = sale.account_sales.get(&account_id) {
            sale_account.into()
        } else {
            SaleAccount {
                amount: U128(0),
                amount_to_claim: U128(0),
                claimed: U128(0),
                refund: U128(0),
                refunded: U128(0),
            }
        }
    }

    pub fn get_affiliate_account(&self, sale_id: u64, account_id: AccountId) -> AffiliateRewardAccount {
        let sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();
        if let Some(sale_account) = sale.account_affiliate_rewards.get(&account_id) {
            sale_account.into()
        } else {
            AffiliateRewardAccount {
                amount: U128::from(0),
                claimed: U128::from(0),
            }
        }
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

    pub(crate) fn withdraw_purchase(&mut self,
                                    recipient_account_id: AccountId,
                                    amount_to_claim: Balance,
                                    claim_token_account_id: AccountId,
                                    sale_id: u64) -> Promise {
        ext_fungible_token::ft_transfer(
            recipient_account_id.clone(),
            amount_to_claim.into(),
            Some(format!("Claim {} of {}. Sale #{}", amount_to_claim, claim_token_account_id, sale_id)),
            claim_token_account_id,
            ONE_YOCTO,
            GAS_FOR_FT_TRANSFER,
        )
            .then(ext_self::after_withdraw_purchase(
                recipient_account_id,
                amount_to_claim.into(),
                sale_id,
                env::current_account_id(),
                NO_DEPOSIT,
                GAS_FOR_AFTER_FT_TRANSFER,
            ))
    }

    #[private]
    pub fn after_withdraw_purchase(
        &mut self,
        account_id: AccountId,
        amount_to_claim: U128,
        sale_id: u64,
    ) -> bool {
        let promise_success = is_promise_success();
        if !promise_success {
            let mut sale: Sale = self
                .sales
                .get(&sale_id)
                .expect("ERR_NO_SALE")
                .into();

            if let Some(v_sale_account) = sale.account_sales.get(&account_id) {
                let mut account_sale: SaleAccount = v_sale_account.into();
                account_sale.claimed = U128::from(account_sale.claimed.0 - amount_to_claim.0);
                sale.account_sales.insert(&account_id, &VSaleAccount::Current(account_sale));
                self.sales.insert(&sale_id, &VSale::Current(sale));
                log!("Purchase withdraw for {} failed. Tokens to recharge: {}",account_id, amount_to_claim.0);
            }
        }
        promise_success
    }

    pub(crate) fn refund_purchase(&mut self,
                                  recipient_account_id: AccountId,
                                  amount_to_refund: Balance,
                                  token_account_id: AccountId,
                                  sale_id: u64) -> Promise {
        ext_fungible_token::ft_transfer(
            recipient_account_id.clone(),
            amount_to_refund.into(),
            Some(format!("Refund {} of {}. Sale #{}", amount_to_refund, token_account_id, sale_id)),
            token_account_id,
            ONE_YOCTO,
            GAS_FOR_FT_TRANSFER,
        )
            .then(ext_self::after_refund_purchase(
                recipient_account_id,
                amount_to_refund.into(),
                sale_id,
                env::current_account_id(),
                NO_DEPOSIT,
                GAS_FOR_AFTER_FT_TRANSFER,
            ))
    }

    #[private]
    pub fn after_refund_purchase(
        &mut self,
        account_id: AccountId,
        amount_to_refund: U128,
        sale_id: u64,
    ) -> bool {
        let promise_success = is_promise_success();
        if !promise_success {
            let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();

            if let Some(v_sale_account) = sale.account_sales.get(&account_id) {
                let mut account_sale: SaleAccount = v_sale_account.into();
                account_sale.refunded = U128::from(account_sale.refunded.0 - amount_to_refund.0);
                sale.account_sales.insert(&account_id, &VSaleAccount::Current(account_sale));
                self.sales.insert(&sale_id, &VSale::Current(sale));
                log!("Purchase refund for {} failed. Tokens to recharge: {}", account_id, amount_to_refund.0);
            }
        }

        promise_success
    }

    pub(crate) fn withdraw_affiliate_reward(&mut self,
                                            recipient_account_id: AccountId,
                                            amount: Balance,
                                            token_account_id: AccountId,
                                            sale_id: u64) -> Promise {
        ext_fungible_token::ft_transfer(
            recipient_account_id.clone(),
            amount.into(),
            Some(format!("Claim affiliate rewards {} of {}. Sale #{}", amount, token_account_id, sale_id)),
            token_account_id,
            ONE_YOCTO,
            GAS_FOR_FT_TRANSFER,
        )
            .then(ext_self::after_withdraw_affiliate_reward(
                recipient_account_id,
                amount.into(),
                sale_id,
                env::current_account_id(),
                NO_DEPOSIT,
                GAS_FOR_AFTER_FT_TRANSFER,
            ))
    }

    #[private]
    pub fn after_withdraw_affiliate_reward(
        &mut self,
        account_id: AccountId,
        amount: U128,
        sale_id: u64,
    ) -> bool {
        let promise_success = is_promise_success();
        if !promise_success {
            let mut sale: Sale = self.sales.get(&sale_id).expect("ERR_NO_SALE").into();

            if let Some(v_sale_account) = sale.account_affiliate_rewards.get(&account_id) {
                let mut account_affiliate_reward: AffiliateRewardAccount = v_sale_account.into();
                account_affiliate_reward.claimed = U128::from(account_affiliate_reward.claimed.0 - amount.0);
                sale.account_affiliate_rewards.insert(&account_id, &VAffiliateRewardAccount::Current(account_affiliate_reward));
                self.sales.insert(&sale_id, &VSale::Current(sale));
                log!("Affiliate rewards withdraw for {} failed. Tokens to recharge: {}",account_id, amount.0);
            }
        }
        promise_success
    }
}


fn is_promise_success() -> bool {
    assert_eq!(
        env::promise_results_count(),
        1,
        "Contract expected a result on the callback"
    );
    match env::promise_result(0) {
        PromiseResult::Successful(_) => true,
        _ => false,
    }
}

fn get_amount_by_subscription(amount_to_claim: Balance, collected_amount: Balance, supply_amount: Balance) -> u128 {
    (
        U256::from(amount_to_claim)
            * U256::from(supply_amount)
            / U256::from(collected_amount)
    ).as_u128()
}

fn internal_get_affiliates_vector(affiliates: &LookupMap<u8, UnorderedSet<AccountId>>, level: u8) -> Vec<AccountId> {
    if let Some(affiliates_unwrapped) =  affiliates.get(&level){
        affiliates_unwrapped.to_vec()
    }
    else{
        [].to_vec()
    }
}
