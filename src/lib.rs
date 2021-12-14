use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, Balance, BorshStorageKey, Gas, PanicOnDefault,
    Promise, PromiseOrValue, PublicKey,
};

use crate::sale::VSale;

mod migration_0;
mod migration_1;
mod sale;
mod token_receiver;

const NO_DEPOSIT: Balance = 0;
const REFERRAL_FEE_DENOMINATOR: u128 = 10000;
const WRAP_NEAR_ACCOUNT: &str = "wrap.near";

#[ext_contract(ext_self)]
pub trait ExtContract {
    /// Callback from checking staked balance of the given user.
    fn on_get_account_staked_balance(
        &mut self,
        sale_id: u64,
        token_id: AccountId,
        sender_id: AccountId,
        deposit_amount: U128,
    ) -> PromiseOrValue<U128>;

    /// Callback after account creation.
    fn on_create_account(&mut self, new_account_id: AccountId) -> Promise;

    /// Callback after near deposit
    fn after_ft_on_transfer_near_deposit(
        &mut self,
        sender_id: AccountId,
        deposit_amount: U128,
    ) -> PromiseOrValue<U128>;

    /// Callback after token claim
    fn after_withdraw_purchase(
        &mut self,
        account_id: AccountId,
        amount_to_claim: U128,
        sale_id: u64,
    ) -> bool;

    /// Callback after token refund for subscription sales
    fn after_refund_purchase(
        &mut self,
        account_id: AccountId,
        amount_to_refund: U128,
        sale_id: u64,
    ) -> bool;

    /// Callback after affiliate_rewards claim
    fn after_withdraw_affiliate_reward(
        &mut self,
        account_id: AccountId,
        amount: U128,
        sale_id: u64,
    ) -> bool;
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct AccountOld {
    referrer: AccountId,
    links: UnorderedSet<PublicKey>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Account {
    referrer: AccountId,
    affiliates: LookupMap<u8, UnorderedSet<AccountId>>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum VAccount {
    Current(Account),
}

impl From<VAccount> for Account {
    fn from(v_account: VAccount) -> Self {
        match v_account {
            VAccount::Current(account) => account,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
struct AccountOutput {
    referrer: AccountId,
    affiliates_num: u64,
}

impl From<Account> for AccountOutput {
    fn from(account: Account) -> Self {
        let aff_0 = get_affiliates_num(&account, 0);
        let aff_1 = if aff_0 > 0 {
            get_affiliates_num(&account, 1)
        } else {
            0
        };
        let aff_2 = if aff_1 > 0 {
            get_affiliates_num(&account, 2)
        } else {
            0
        };

        Self {
            referrer: account.referrer,
            affiliates_num: aff_0 + aff_1 + aff_2,
        }
    }
}

fn get_affiliates_num(account: &Account, level: u8) -> u64 {
    if let Some(affiliates) = account.affiliates.get(&level) {
        affiliates.len()
    } else {
        0
    }
}

impl Account {
    pub fn new(account_id: &AccountId, referrer: &AccountId) -> Self {
        Self {
            referrer: referrer.clone(),
            affiliates: LookupMap::new(StorageKey::Affiliates {
                account_id: account_id.clone(),
            }),
        }
    }
}

#[allow(dead_code)]
#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
    Sales,
    AccountSales { sale_id: u64 },
    Links,
    AccountLinks,
    AccountAffiliateRewards { sale_id: u64 },
    Affiliates { account_id: AccountId },
    AffiliateLevels { account_id: AccountId, level: u8 },
    AccountsV1,
}

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
struct Contract {
    owner_id: AccountId,
    join_fee: Balance,
    /// 3 values vector, 1 => 0.01% (fee / 10000)
    referral_fees: Vec<u64>,
    accounts: UnorderedMap<AccountId, VAccount>,
    sales: LookupMap<u64, VSale>,
    num_sales: u64,
    // not used anymore
    accounts_old: UnorderedMap<AccountId, AccountOld>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId, join_fee: U128, referral_fees: Vec<u64>) -> Self {
        let mut this = Self {
            owner_id,
            join_fee: join_fee.0,
            referral_fees,
            accounts: UnorderedMap::new(StorageKey::Accounts),
            sales: LookupMap::new(StorageKey::Sales),
            num_sales: 0,
            accounts_old: UnorderedMap::new(StorageKey::AccountsV1),
        };
        this.accounts.insert(
            &this.owner_id,
            &VAccount::Current(Account::new(&this.owner_id, &this.owner_id)),
        );
        this
    }

    #[payable]
    pub fn join(&mut self, referrer_id: Option<AccountId>) {
        let account_id = env::predecessor_account_id();
        let referrer_id_unwrapped: AccountId = if let Some(referrer_id_unwrapped) = referrer_id {
            if self.accounts.get(&referrer_id_unwrapped).is_some() {
                referrer_id_unwrapped
            } else {
                self.owner_id.clone()
            }
        } else {
            self.owner_id.clone()
        };

        assert_ne!(referrer_id_unwrapped, account_id, "SELF_REFERRER");
        assert!(
            self.accounts.get(&account_id).is_none(),
            "ERR_ACCOUNT_EXISTS"
        );
        assert_eq!(env::attached_deposit(), self.join_fee, "ERR_FEE");
        self.accounts.insert(
            &account_id,
            &VAccount::Current(Account::new(&account_id, &referrer_id_unwrapped)),
        );

        // Don't save internal affiliates to save storage and gas
        if self.owner_id != referrer_id_unwrapped {
            self.insert_affiliates(referrer_id_unwrapped, account_id);
        }
    }

    fn insert_affiliate_on_level(
        &mut self,
        referrer_id: &AccountId,
        mut referrer_account: Account,
        level: u8,
        affiliate_account_id: &AccountId,
    ) {
        let mut affiliates_on_level: UnorderedSet<AccountId> =
            if let Some(affiliates_unwrapped) = referrer_account.affiliates.get(&level) {
                affiliates_unwrapped
            } else {
                UnorderedSet::new(StorageKey::AffiliateLevels {
                    account_id: referrer_id.clone(),
                    level,
                })
            };

        affiliates_on_level.insert(affiliate_account_id);
        referrer_account
            .affiliates
            .insert(&level, &affiliates_on_level);

        self.accounts
            .insert(referrer_id, &VAccount::Current(referrer_account));
    }

    fn insert_affiliates(&mut self, referrer_1_id: AccountId, account_id: AccountId) {
        if let Some(referrer_v_account_1) = self.accounts.get(&referrer_1_id) {
            let referrer_account_1: Account = referrer_v_account_1.into();
            let referrer_2_id = &referrer_account_1.referrer.clone();
            self.insert_affiliate_on_level(&referrer_1_id, referrer_account_1, 0, &account_id);

            if &self.owner_id != referrer_2_id {
                if let Some(referrer_v_account_2) = self.accounts.get(referrer_2_id) {
                    let referrer_account_2: Account = referrer_v_account_2.into();
                    let referrer_3_id = &referrer_account_2.referrer.clone();
                    self.insert_affiliate_on_level(
                        referrer_2_id,
                        referrer_account_2,
                        1,
                        &account_id,
                    );

                    if &self.owner_id != referrer_3_id {
                        if let Some(referrer_v_account_3) = self.accounts.get(referrer_3_id) {
                            let referrer_account_3: Account = referrer_v_account_3.into();
                            self.insert_affiliate_on_level(
                                referrer_3_id,
                                referrer_account_3,
                                2,
                                &account_id,
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn get_join_fee(&self) -> U128 {
        U128(self.join_fee)
    }

    pub fn get_referral_fees(&self) -> Vec<u64> {
        self.referral_fees.clone()
    }

    pub fn get_account(&self, account_id: AccountId) -> AccountOutput {
        let account: Account = self
            .accounts
            .get(&account_id)
            .expect("ERR_ACCOUNT_DOESNT_EXIST")
            .into();

        account.into()
    }

    pub fn get_num_accounts(&self) -> u64 {
        self.accounts.len()
    }

    pub fn get_accounts(&self, from_index: u64, limit: u64) -> Vec<(AccountId, AccountOutput)> {
        let keys = self.accounts.keys_as_vector();
        let values = self.accounts.values_as_vector();
        (from_index..std::cmp::min(from_index + limit, keys.len()))
            .map(|index| {
                let account: Account = values.get(index).unwrap().into();
                (keys.get(index).unwrap(), account.into())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_sdk::json_types::U64;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::test_utils::{accounts, testing_env_with_promise_results};
    use near_sdk::{serde_json, testing_env, PromiseResult};

    use crate::sale::{SaleInput, SaleMetadata, SaleType};
    use crate::token_receiver::SaleDeposit;

    use super::*;

    fn contract_with_sale_info(
        max_amount: Balance,
        start_date: u64,
        end_date: u64,
    ) -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let join_fee = U128(1_000_000);
        let referral_fees = vec![10, 20, 30];
        let mut contract = Contract::new(accounts(0), join_fee, referral_fees.clone());
        contract.create_sale(SaleInput {
            metadata: SaleMetadata {
                name: "test".to_string(),
                symbol: "TEST".to_string(),
                description: "".to_string(),
                smart_contract_url: "".to_string(),
                logo_url: "".to_string(),
                output_ticker: "".to_string(),
                project_telegram: None,
                project_medium: None,
                project_twitter: None,
                reward_timestamp: None,
                reward_description: None,
            },
            staking_contracts: vec![AccountId::new_unchecked("test.staking".to_string())],
            min_near_deposit: U128(100),
            deposit_token_id: accounts(1),
            claim_available: true,
            distribute_token_id: None,
            distribute_token_decimals: None,
            min_buy: U128(100),
            max_buy: U128(10000),
            max_amount: max_amount.map(|a| U128(a)),
            hard_max_amount_limit: max_amount.is_some(),
            start_date: U64(start_date),
            end_date: U64(end_date),
            price: U128(1000),
            whitelist_hash: None,
            limit_per_transaction: U128(100),
            sale_type: SaleType::ByAmount,
        });
        assert_eq!(contract.get_referral_fees(), referral_fees);
        assert_eq!(contract.get_join_fee(), join_fee);
        (context, contract)
    }

    fn contract_with_sale() -> (VMContextBuilder, Contract) {
        contract_with_sale_info(Some(10000), 0, 1_000_000_000)
    }

    fn register_account(
        context: &mut VMContextBuilder,
        contract: &mut Contract,
        account_id: AccountId,
    ) {
        testing_env!(context
            .predecessor_account_id(account_id)
            .attached_deposit(1000000)
            .build());
        contract.join();
    }

    fn deposit(context: &mut VMContextBuilder, contract: &mut Contract, account_id: AccountId) {
        testing_env!(context.predecessor_account_id(accounts(1)).build());
        contract.ft_on_transfer(
            account_id,
            U128(100),
            serde_json::to_string(&SaleDeposit {
                sale_id: 0,
                staking_contract: None,
            })
            .unwrap(),
        );
    }

    #[test]
    fn test_basics() {
        let (mut context, mut contract) = contract_with_sale();
        assert_eq!(contract.get_num_sales(), 1);
        assert_eq!(contract.get_sale(0).price.0, 1000);
        assert_eq!(contract.get_sales(0, 10).len(), 1);

        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(1000000)
            .build());
        contract.join();
        assert_eq!(contract.get_account(accounts(2)).referrer, accounts(0));

        testing_env!(context.predecessor_account_id(accounts(1)).build());
        contract.ft_on_transfer(
            accounts(2),
            U128(100),
            serde_json::to_string(&SaleDeposit {
                sale_id: 0,
                staking_contract: Some(AccountId::new_unchecked("test.staking".to_string())),
            })
            .unwrap(),
        );

        testing_env_with_promise_results(
            context
                .current_account_id(accounts(0))
                .predecessor_account_id(accounts(0))
                .build(),
            PromiseResult::Successful(vec![]),
        );
        contract.on_get_account_staked_balance(U128(1000), 0, accounts(1), accounts(2), U128(100));

        assert_eq!(contract.get_sale(0).num_account_sales, 1);
        assert_eq!(contract.get_sale(0).collected_amount.0, 100);

        assert_eq!(contract.get_num_accounts(), 2);
        assert_eq!(contract.get_accounts(0, 10).len(), 2);
        assert_eq!(contract.get_account(accounts(2)).referrer, accounts(0));
    }

    #[test]
    #[should_panic(expected = "ERR_NOT_REGISTERED_ACCOUNT")]
    fn test_not_registered() {
        let (mut context, mut contract) = contract_with_sale();
        testing_env!(context.predecessor_account_id(accounts(1)).build());
        contract.ft_on_transfer(
            accounts(2),
            U128(100),
            serde_json::to_string(&SaleDeposit {
                sale_id: 0,
                staking_contract: None,
            })
            .unwrap(),
        );
    }

    #[test]
    #[should_panic(expected = "ERR_NO_SALE")]
    fn test_no_sale() {
        let (mut context, mut contract) = contract_with_sale();
        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(1000000)
            .build());
        contract.join();
        testing_env!(context.predecessor_account_id(accounts(1)).build());
        contract.ft_on_transfer(
            accounts(2),
            U128(100),
            serde_json::to_string(&SaleDeposit {
                sale_id: 1,
                staking_contract: None,
            })
            .unwrap(),
        );
    }

    #[test]
    fn test_create_remove_link() {
        let (mut context, mut contract) = contract_with_sale();
        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(1000000)
            .build());
        contract.join();
        testing_env!(context
            .predecessor_account_id(accounts(2))
            .attached_deposit(CREATE_LINK_AMOUNT)
            .build());
        let pk = PublicKey::from_str("qSq3LoufLvTCTNGC3LJePMDGrok8dHMQ5A1YD9psbiz").unwrap();
        contract.create_link(pk.clone());
        contract.remove_link(pk);
    }

    #[test]
    #[should_panic = "ERR_SALE_NOT_STARTED"]
    fn test_sale_too_early() {
        let (mut context, mut contract) = contract_with_sale_info(None, 1_000, 1_000_000);
        register_account(&mut context, &mut contract, accounts(2));
        deposit(&mut context, &mut contract, accounts(2));
    }

    #[test]
    #[should_panic = "ERR_SALE_DONE"]
    fn test_sale_too_late() {
        let (mut context, mut contract) = contract_with_sale_info(None, 1_000, 1_000_000);
        register_account(&mut context, &mut contract, accounts(2));
        testing_env!(context
            .block_timestamp(1_000_001)
            .predecessor_account_id(accounts(1))
            .build());
        contract.ft_on_transfer(
            accounts(2),
            U128(100),
            serde_json::to_string(&SaleDeposit {
                sale_id: 0,
                staking_contract: None,
            })
            .unwrap(),
        );
    }
}
