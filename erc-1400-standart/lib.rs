#![cfg_attr(not(feature = "std"), no_std)]
use ink_lang as ink;
pub mod models;

#[ink::contract]
mod erc1400 {
    use super::*;
    use models::doc::*;

    use ink_prelude::{string::String, vec::Vec};
    use ink_storage::{collections::HashMap as StorageHashMap, Lazy };

    #[ink(storage)]
    pub struct Erc1400 {
        symbol: Lazy<String>,
        granularity: Lazy<Balance>,
        total_supply: Balance,
        balances: StorageHashMap<AccountId, Balance>,
        allow: StorageHashMap<AccountId, Balance>,
        documents: Vec<Document>,
        total_paritions: Vec<Hash>,
        partitions_of: StorageHashMap<AccountId, Vec<Hash>>,
        balance_of_partition: StorageHashMap<(AccountId, Hash), Balance >,
        owner: Lazy<AccountId>,
        authorized_operator: StorageHashMap<AccountId, bool>,
        controllers: StorageHashMap<AccountId, bool>,
        allow_by_partition: StorageHashMap<(AccountId, Hash), Balance >,
        authorized_operator_by_partition: StorageHashMap<(AccountId, Hash), bool>,
        controllers_by_partition: StorageHashMap<(AccountId, Hash), bool>
    }

    #[ink(event)]
    pub struct TransferByPartition {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        amount: Option<AccountId>,
        #[ink(topic)]
        data: Option<AccountId>,
    }

    impl Erc1400 {
        #[ink(constructor)]
        pub fn new(token_symbol: String, granularity: Balance) -> Self {
            let caller = Self::env().caller();

            Self { 
                symbol: Lazy::new(token_symbol),
                granularity: Lazy::new(granularity),
                total_supply: 0,
                balances: StorageHashMap::new(),
                allow: StorageHashMap::new(),
                documents: Vec::new(),
                total_paritions: Vec::new(),
                partitions_of: StorageHashMap::new(),
                balance_of_partition: StorageHashMap::new(),
                owner: Lazy::new(caller),
                authorized_operator: StorageHashMap::new(),
                controllers: StorageHashMap::new(),
                allow_by_partition: StorageHashMap::new(),
                authorized_operator_by_partition: StorageHashMap::new(),
                controllers_by_partition: StorageHashMap::new()
            }
        }

    }
}