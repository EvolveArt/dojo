pub trait StateProvider {
    /// Returns the value of a contract storage.
    fn storage(&self, address: (), storage_key: ());
    /// Returns the class hash of a contract.
    fn class_hash_of_contrat(&self, address: ());
    /// Returns the class definition of a contract class given its class hash.
    fn class_by_hash(&self, hash: ());
}

pub trait TransactionProvider {
    /// Returns all the transactions for a given block.
    fn transactions_by_block(&self, block_id: ());
    /// Returns a transaction given its hash.
    fn transaction_by_hash(&self, hash: ());
}

pub trait ReceiptProvider {
    /// Returns the transaction receipt given a transaction hash.
    fn receipt_by_hash(&self, hash: ());
    /// Returns all the receipts for a given block.
    fn receipts_by_block(&self, block_id: ());
}

pub trait BlockProvider {
    /// Returns a block by its hash.
    fn block(&self, id: ());
}
