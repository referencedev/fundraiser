use near_sdk::log;

use crate::sale::*;
use crate::*;

#[near_bindgen]
impl Contract {
    // recreate accounts
    #[private]
    #[init(ignore_state)]
    #[allow(dead_code)]
    pub fn migrate_a0() -> Self {
        #[derive(BorshDeserialize)]
        struct OldContract {
            owner_id: AccountId,
            join_fee: Balance,
            referral_fees: Vec<u64>,
            accounts: UnorderedMap<AccountId, AccountOld>,
            sales: LookupMap<u64, VSale>,
            links: LookupMap<PublicKey, AccountId>,
            num_sales: u64,
        }

        let old_contract: OldContract = env::state_read().expect("Old state doesn't exist");

        Self {
            owner_id: old_contract.owner_id,
            join_fee: old_contract.join_fee,
            referral_fees: old_contract.referral_fees,
            accounts: UnorderedMap::new(StorageKey::AccountsV1),
            sales: old_contract.sales,
            num_sales: old_contract.num_sales,
            accounts_old: old_contract.accounts,
        }
    }

    #[private]
    pub fn migrate_a1(&mut self, limit: u64) {
        // accounts_old transition
        let keys = self.accounts_old.keys_as_vector();
        let account_ids: Vec<AccountId> = (0..std::cmp::min(limit, keys.len()))
            .map(|index| keys.get(index).unwrap().into())
            .collect();

        for account_id in account_ids {
            let account_old: AccountOld = self.accounts_old.remove(&account_id).unwrap();
            let account = Account {
                referrer: account_old.referrer,
                affiliates: LookupMap::new(StorageKey::Affiliates {
                    account_id: account_id.clone(),
                }),
            };
            self.accounts
                .insert(&account_id, &VAccount::Current(account));
        }

        log!("Pending items: {}", self.accounts_old.len());
    }
}
