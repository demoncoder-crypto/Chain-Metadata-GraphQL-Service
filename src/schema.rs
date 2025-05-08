use crate::models::{ChainInfo, Event, EventFilterInput};
use crate::indexer::SubstrateIndexerService;
use crate::errors::AppError;
use crate::dataloader::{AppDataloader, ChainInfoLoaderKey, ChainInfoLoader};
use async_graphql::{
    Context, Object, FieldResult, Subscription, ID, Schema, EmptyMutation, ComplexObject, dataloader::DataLoader
};
use tokio_stream::Stream;
use futures_util::stream::StreamExt;
use std::sync::Arc;
use tracing::instrument;

// Define the Query root object
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    #[instrument(skip(self))]
    async fn health_check(&self) -> String {
        "OK".to_string()
    }

    #[instrument(skip(self))]
    async fn echo(&self, message: String) -> String {
        message
    }

    #[instrument(name = "query.chain_info", skip_all)]
    async fn chain_info<'ctx>(
        &self,
        ctx: &Context<'ctx>,
    ) -> FieldResult<ChainInfo> {
        let indexer_service = ctx.data::<SubstrateIndexerService>()?;
        indexer_service.get_chain_info().await
    }

    #[instrument(name = "query.event", skip_all, fields(id))]
    async fn event<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        id: ID,
    ) -> FieldResult<Option<Event>> {
        let indexer_service = ctx.data::<SubstrateIndexerService>()?;
        indexer_service.get_event_by_id(id).await
    }

    #[instrument(name = "query.events", skip_all, fields(filter))]
    async fn events<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        filter: Option<EventFilterInput>,
    ) -> FieldResult<Vec<Event>> {
        let indexer_service = ctx.data::<SubstrateIndexerService>()?;
        indexer_service.list_events(filter).await
    }
}

// Define the Subscription root object
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    #[instrument(name = "subscription.events", skip_all)]
    async fn events<'ctx>(
        &self,
        ctx: &Context<'ctx>
    ) -> impl Stream<Item = Event> + 'ctx {
        let indexer_service = ctx.data_unchecked::<SubstrateIndexerService>().clone();
        Box::pin(async_stream::stream! {
            let mut inner_stream = indexer_service.watch_events().await;
            while let Some(event) = inner_stream.next().await {
                yield event;
            }
        })
    }
}

// Schema type
pub type AppSchema = Schema<QueryRoot, EmptyMutation, SubscriptionRoot>;

// Complex object implementations for relationships
#[ComplexObject]
impl Event {
    #[instrument(name = "event.chain", skip(self, ctx), fields(id, pallet_name, event_name))]
    async fn chain<'ctx>(&self, ctx: &Context<'ctx>) -> FieldResult<ChainInfo> {
        let loader = ctx.data::<DataLoader<ChainInfoLoader>>()?;
        match loader.load_one(ChainInfoLoaderKey(self.chain_id.clone())).await? {
            Some(chain_info) => Ok(chain_info),
            None => Err(AppError::NotFound(format!("ChainInfo not found for ID: {}", self.chain_id)).into()),
        }
    }
}

#[ComplexObject]
impl ChainInfo {
    #[instrument(name = "chain_info.current_block_height", skip(self, _ctx), fields(id, name))]
    async fn current_block_height<'ctx>(&self, _ctx: &Context<'ctx>) -> FieldResult<u64> {
        let store_lock = crate::indexer::MOCK_EVENT_STORE.lock()
            .map_err(|e| AppError::Internal(format!("Failed to lock event store for block height: {}", e)))?;
        let max_block = store_lock.events.values().map(|e| e.block_number).max().unwrap_or(0);
        Ok(max_block)
    }
} 