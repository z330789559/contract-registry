#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod stable_currency {

    use ink_storage::{collections::HashMap as StorageHashMap, lazy::Lazy};
    use ink_prelude::string::String;

    #[derive(Debug, PartialEq, Eq, scale::Encode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        InsufficientBalance,
        InsufficientAllowance,
        OnlyOwner,
        NotPermission
    }

    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(storage)]
    pub struct StableCurrency {
        ///Owner of Contract.
        owner: Lazy<AccountId>,
        /// Total token supply.
        total_supply: Lazy<Balance>,
        /// Mapping from owner to number of owned token.
        balances: StorageHashMap<AccountId, Balance>,
        /// Mapping of the token amount which an account is allowed to withdraw from another account.
        allowances: StorageHashMap<(AccountId, AccountId), Balance>,
        /// Token symbol
        symbol: String,
        ///ecrow balance
        escrow_balances: StorageHashMap<(AccountId, AccountId), Balance>,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        value: Balance,
    }
    /// Event emitted when an approval occurs that `spender` is allowed to withdraw
    /// up to the amount of `value` tokens from `owner`.
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        #[ink(topic)]
        value: Balance,
    }

    impl StableCurrency {
        #[ink(constructor)]
        pub fn new(initial_supply: Balance, symbol: String) -> Self {
            let caller = Self::env().caller();
            let mut balances = StorageHashMap::new();
            balances.insert(caller, initial_supply);

            Self {
                owner: Lazy::new(caller),
                total_supply: Lazy::new(initial_supply),
                escrow_balances: StorageHashMap::new(),
                allowances: StorageHashMap::new(),
                balances,
                symbol
            }
        }

        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            *self.total_supply
        }

        #[ink(message)]
        pub fn token_symbol(&self) -> String {
            self.symbol.clone()
        }

        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balance_of_or_zero(&owner)
        }

        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, value: Balance) -> bool {
            // Record the new allowance.
            let owner = self.env().caller();
            self.allowances.insert((owner, spender), value);

            // Notify offchain users of the approval and report success.
            self.env().emit_event(Approval {
                owner,
                spender,
                value,
            });
            true
        }

        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
            self.allowance_of_or_zero(&owner, &spender)
        }

        #[ink(message)]
        pub fn transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: Balance,
        ) -> Result<()> {
            // Ensure that a sufficient allowance exists.
            let caller = self.env().caller();
            let allowance = self.allowance_of_or_zero(&from, &caller);
            if allowance < value {
                return Err(Error::InsufficientAllowance);
            }

            self.transfer_from_to(from, to, value)?;
            self.allowances.insert((from, caller), allowance - value);
            Ok(())
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            self.transfer_from_to(self.env().caller(), to, value)
        }

        #[ink(message)]
        pub fn transfer_ownership(&mut self, to: AccountId) -> Result<()> {
            self.only_owner(self.env().caller())?;
            *self.owner = to;
            Ok(())
        }

        #[ink(message)]
        pub fn inc_supply(&mut self, value: Balance) -> Result<()> {
            let caller = self.env().caller();
            self.only_owner(caller)?;

            let owner_balance = self.balance_of_or_zero(&caller);
            *self.total_supply += value;
            self.balances.insert(caller, owner_balance + value);

            Ok(())
        }

        ///Decrement total supply only by owner.
        #[ink(message)]
        pub fn dec_supply(&mut self, value: Balance) -> Result<()> {
            let caller = self.env().caller();
            self.only_owner(caller)?;

            let owner_balance = self.balance_of_or_zero(&caller);
            if owner_balance < value {
                return Err(Error::InsufficientBalance);
            }
            *self.total_supply -= value;
            self.balances.insert(caller, owner_balance - value);

            Ok(())
        }

        #[ink(message)]
        pub fn create_payment(&mut self, seller: AccountId, value: u64) -> Result<()> {
            let order = self.env().caller();
            let fee = self.cal_fee(value);
            let value = (value.checked_add(fee).unwrap_or(0)) as Balance;

            let order_balance = self.balance_of_or_zero(&order);
            if order_balance < value {
                return Err(Error::InsufficientBalance);
            }
            self.balances.insert(order, order_balance - value);

            let escrow_balance = self.escrow_of_or_zero(&order, &seller);
            self.escrow_balances
                .insert((order, seller), escrow_balance + value);
            Ok(())
        }

        #[ink(message)]
        pub fn complete_payment(&mut self, from: AccountId, to: AccountId) -> Result<()> {
            let caller = self.env().caller();

            if caller.clone() == from || caller.clone() == *self.owner {
                let esbalance = self.escrow_of_or_zero(&from, &to);
                let fee = (self.cal_fee(esbalance as u64)) as Balance;
                let esbalance = esbalance - fee;
                
                let tobalance = self.balance_of_or_zero(&to);
                self.balances.insert(to, esbalance + tobalance);
                
                let callbalance = self.balance_of_or_zero(&caller.clone());
                self.balances.insert(caller, fee + callbalance);
                
                self.escrow_balances.insert((from, to), 0);

                Ok(())
            } else {
                Err(Error::NotPermission)
            }
        }

        #[ink(message)]
        pub fn refund(&mut self, from: AccountId, to: AccountId) -> Result<()> {
            let caller = self.env().caller();
            let esbalance = self.escrow_of_or_zero(&from, &to);

            if caller.clone() == to || caller.clone() == *self.owner {
                let balance = self.balance_of_or_zero(&from);
                self.balances.insert(from, esbalance + balance);

                self.escrow_balances.insert((from, to), 0);

                Ok(())
            } else {
                Err(Error::NotPermission)
            }
        }

        #[ink(message)]
        pub fn escrow_balance(&self, from: AccountId, to: AccountId) -> Balance {
            self.escrow_of_or_zero(&from, &to)
        }

        fn transfer_from_to(
            &mut self,
            from: AccountId,
            to: AccountId,
            value: Balance,
        ) -> Result<()> {
            let from_balance = self.balance_of_or_zero(&from);
            if from_balance < value {
                return Err(Error::InsufficientBalance);
            }

            // Update the sender's balance.
            self.balances.insert(from, from_balance - value);

            // Update the receiver's balance.
            let to_balance = self.balance_of_or_zero(&to);
            self.balances.insert(to, to_balance + value);

            self.env().emit_event(Transfer {
                from: Some(from),
                to: Some(to),
                value,
            });
            Ok(())
        }

        fn escrow_of_or_zero(&self, order: &AccountId, seller: &AccountId) -> Balance {
            *self.escrow_balances.get(&(*order, *seller)).unwrap_or(&0)
        }

        fn cal_fee(&self, value: u64) -> u64 {
            let mut fee = value.checked_div(100).unwrap_or(1);
            if fee == 0 {
                fee = 1;
            }
            fee
        }

        fn only_owner(&self, caller: AccountId) -> Result<()> {
            if *self.owner == caller {
                Ok(())
            } else {
                return Err(Error::OnlyOwner);
            }
        }

        fn balance_of_or_zero(&self, owner: &AccountId) -> Balance {
            *self.balances.get(owner).unwrap_or(&0)
        }

        fn allowance_of_or_zero(&self, owner: &AccountId, spender: &AccountId) -> Balance {
            *self.allowances.get(&(*owner, *spender)).unwrap_or(&0)
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;

        use ink_lang as ink;

        #[ink::test]
        fn new_works() {
            let contract = StableCurrency::new(777, "rsel".to_string());
            assert_eq!(contract.total_supply(), 777);
            assert_eq!(contract.symbol, "rsel".to_owned());
            assert_ne!(contract.symbol, "sel".to_owned())
        }

        #[ink::test]
        fn balance_works() {
            let contract = StableCurrency::new(100, "rsel".to_string());
            assert_eq!(contract.total_supply(), 100);
            assert_eq!(contract.balance_of(AccountId::from([0x1; 32])), 100);
            assert_eq!(contract.balance_of(AccountId::from([0x0; 32])), 0);
        }

        #[ink::test]
        fn transfer_works() {
            let mut contract = StableCurrency::new(100, "rsel".to_string());
            assert_eq!(contract.balance_of(AccountId::from([0x1; 32])), 100);
            assert_eq!(contract.transfer(AccountId::from([0x0; 32]), 10), Ok(()));
            assert_eq!(contract.balance_of(AccountId::from([0x0; 32])), 10);
            assert_ne!(contract.transfer(AccountId::from([0x0; 32]), 100), Ok(()));
        }

        #[ink::test]
        fn transfer_from_works() {
            let mut contract = StableCurrency::new(100, "rsel".to_string());
            assert_eq!(contract.balance_of(AccountId::from([0x1; 32])), 100);
            contract.approve(AccountId::from([0x1; 32]), 20);
            contract
                .transfer_from(AccountId::from([0x1; 32]), AccountId::from([0x0; 32]), 10)
                .unwrap();
            assert_eq!(contract.balance_of(AccountId::from([0x0; 32])), 10);
        }

        #[ink::test]
        fn onlyowner_works() {
            let contract = StableCurrency::new(777, "rsel".to_string());
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
        }

        #[ink::test]
        fn transfer_ownership_works() {
            let mut contract = StableCurrency::new(777, "rsel".to_string());
            assert_eq!(contract.only_owner(AccountId::from([0x1; 32])), Ok(()));
            contract
                .transfer_ownership(AccountId::from([0x0; 32]))
                .unwrap();
            assert_eq!(contract.only_owner(AccountId::from([0x0; 32])), Ok(()));
        }

        #[ink::test]
        fn inc_subpply_works() {
            let mut contract = StableCurrency::new(777, "rsel".to_string());
            contract.inc_supply(1000).unwrap();
            assert_eq!(contract.total_supply(), 1777);
            assert_eq!(contract.balance_of(AccountId::from([0x1; 32])), 1777);
        }

        #[ink::test]
        fn dec_subpply_works() {
            let mut contract = StableCurrency::new(777, "rsel".to_string());
            contract.dec_supply(10).unwrap();
            assert_eq!(contract.total_supply(), 767);
            assert_eq!(contract.balance_of(AccountId::from([0x1; 32])), 767);
        }

        #[ink::test]
        fn createpayment_works() {
            let mut contract = StableCurrency::new(100,  "rsel".to_string());
            let buyer = AccountId::from([0x1; 32]);
            let seller = AccountId::from([0x0; 32]);
            assert_eq!(contract.balance_of(buyer), 100);
            assert_eq!(contract.create_payment(seller, 30), Ok(()));
            assert_eq!(contract.balance_of(buyer), 69);
            assert_eq!(contract.escrow_balance(buyer, seller), 31);
        }

        #[ink::test]
        fn completepaymet_work() {
            let mut contract = StableCurrency::new(100,  "rsel".to_string());
            let buyer = AccountId::from([0x1; 32]);
            let seller = AccountId::from([0x0; 32]);
            assert_eq!(contract.create_payment(seller, 30), Ok(()));
            assert_eq!(contract.balance_of(seller), 0);
            assert_eq!(contract.complete_payment(buyer, seller), Ok(()));
            assert_eq!(contract.balance_of(seller), 30);
            assert_eq!(contract.create_payment(seller, 30), Ok(()));
            assert_eq!(contract.complete_payment(buyer, seller), Ok(()));
            assert_eq!(contract.balance_of(seller), 60);
        }

        #[ink::test]
        fn refund_work() {
            let mut contract = StableCurrency::new(100,  "rsel".to_string());
            let buyer = AccountId::from([0x1; 32]);
            let seller = AccountId::from([0x0; 32]);
            assert_eq!(contract.create_payment(seller, 30), Ok(()));
            assert_eq!(contract.balance_of(buyer), 69);
            assert_eq!(contract.refund(buyer, seller), Ok(()));
            assert_eq!(contract.balance_of(buyer), 100);
        }
    }
}
