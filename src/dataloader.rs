use async_graphql::dataloader::Loader;
use async_graphql::{ID, FieldResult};
use std::collections::HashMap;
use std::sync::Arc;
use crate::models::ChainInfo;
use crate::indexer::SubstrateIndexerService;
use crate::errors::AppError;
use tracing::instrument;

// Define a key for our Dataloader
#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct ChainInfoLoaderKey(pub ID);

// Implement the Loader trait for ChainInfo
pub struct ChainInfoLoader {
    indexer_service: SubstrateIndexerService, // Use the existing service
}

impl ChainInfoLoader {
    pub fn new(indexer_service: SubstrateIndexerService) -> Self {
        Self { indexer_service }
    }
}

#[async_trait::async_trait]
impl Loader<ChainInfoLoaderKey> for ChainInfoLoader {
    type Value = ChainInfo;
    type Error = Arc<AppError>; // Dataloader errors must be Arced

    #[instrument(name = "load_chain_infos", skip(self, keys))]
    async fn load(&self, keys: &[ChainInfoLoaderKey]) -> Result<HashMap<ChainInfoLoaderKey, Self::Value>, Self::Error> {
        tracing::debug!("Dataloader: loading ChainInfo for keys: {:?}", keys);
        let ids_to_fetch: Vec<ID> = keys.iter().map(|k| k.0.clone()).collect();
        
        // Use the batch fetch method from the service
        let chain_infos_map = self
            .indexer_service
            .get_chain_infos_batch(&ids_to_fetch)
            .await
            .map_err(|e| Arc::new(AppError::ServiceError(format!("Failed to batch fetch chain_infos: {}", e))))?;

        // Convert the result back to HashMap<ChainInfoLoaderKey, ChainInfo>
        let result = chain_infos_map
            .into_iter()
            .map(|(id, chain_info)| (ChainInfoLoaderKey(id), chain_info))
            .collect();
        
        Ok(result)
    }
}

// Convenience type for the Dataloader
pub type AppDataloader = async_graphql::dataloader::DataLoader<ChainInfoLoader>; 