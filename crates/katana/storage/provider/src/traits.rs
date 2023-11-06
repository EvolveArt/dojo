use std::ops::RangeInclusive;

pub trait ContractReader {
    /// Returns the contract information given its address.
    fn contract(&self, address: ());
}

pub trait StateProvider: ContractReader {
    /// Returns the class definition of a contract class given its class hash.
    fn class(&self, hash: ());

    /// Returns the nonce of a contract.
    fn nonce(&self, address: ());

    /// Returns the value of a contract storage.
    fn storage(&self, address: (), storage_key: ());

    /// Returns the class hash of a contract.
    fn class_hash_of_contract(&self, address: ());

    /// Returns the compiled class hash for the given class hash.
    fn compiled_class_hash_of_class_hash(&self, hash: ());
}

pub trait TransactionProvider {
    /// Returns all the transactions for a given block.
    fn transactions_by_block(&self, block_id: ());

    /// Returns a transaction given its hash.
    fn transaction_by_hash(&self, hash: ());

    /// Returns the transaction at the given block and its exact index in the block.
    fn transaction_by_block_and_idx(&self, block_id: (), idx: ());
}

pub trait ReceiptProvider {
    /// Returns the transaction receipt given a transaction hash.
    fn receipt_by_hash(&self, hash: ());

    /// Returns all the receipts for a given block.
    fn receipts_by_block(&self, block_id: ());
}

pub trait BlockProvider {
    /// Returns a block by its id.
    fn block(&self, id: ());

    /// Returns all available blocks in the given range.
    fn blocks_in_range(&self, range: RangeInclusive<u64>);

    /// Returns the block based on its hash.
    fn block_by_hash(&self, hash: ()) {
        self.block(hash.into());
    }

    /// Returns the block based on its number.
    fn block_by_number(&self, number: ()) {
        self.block(number.into());
    }
}
