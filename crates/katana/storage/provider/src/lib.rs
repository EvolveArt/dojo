pub mod traits;

/// The main entrypoint for accessing blockchain storage.
///
/// Provide generalization over the inner storage provider for interacting with the blockchain
/// data.
pub struct BlockchainProvider<Db> {
    provider: Db,
}

impl<Db> BlockchainProvider<Db> {
    pub fn new(provider: Db) -> Self {
        Self { provider }
    }
}
