use crate::models::{ChainInfo, Event, EventFilterInput};
use crate::errors::AppError;
use crate::config::AppConfig;
use async_graphql::{ID, FieldResult};
use chrono::{Utc, Duration as ChronoDuration};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast::{self, Sender as BroadcastSender};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::Stream;
use uuid::Uuid;
use rand::Rng;
use serde_json::json;
use tracing::{info, error, instrument};

// Mock data store
// ChainInfo remains fairly static, so Lazy is fine.
pub static MOCK_CHAIN_INFO: Lazy<ChainInfo> = Lazy::new(|| ChainInfo {
    id: ID::from("polkadot-mainnet-mock"),
    name: "Polkadot (Mock)".to_string(),
    version: "0.9.99-mock".to_string(),
    token_symbol: "MDOT".to_string(),
    decimals: 10,
    ssv58_prefix: 0,
    last_updated: Utc::now(),
});

struct MockEventStore {
    events: HashMap<ID, Event>,
    event_sender: BroadcastSender<Event>,
    chain_id: ID,
}

// Use an Arc<Mutex<...>> for the event store to allow it to be shared safely across async tasks
// and potentially with Dataloaders later.
pub static MOCK_EVENT_STORE: Lazy<Arc<Mutex<MockEventStore>>> = Lazy::new(|| {
    let (tx, _) = broadcast::channel(100);
    let chain_id = MOCK_CHAIN_INFO.id.clone();
    let mut events = HashMap::new();
    for i in 0..5 {
        let event_id = ID::from(Uuid::new_v4().to_string());
        let event = Event {
            id: event_id.clone(),
            block_number: 10000 + i,
            extrinsic_id: Some(format!("0x{}", Uuid::new_v4().to_simple())),
            timestamp: Utc::now() - ChronoDuration::seconds((5 - i) as i64 * 10),
            pallet_name: "Balances".to_string(),
            event_name: "Transfer".to_string(),
            data: json!({ "from": "Alice", "to": "Bob", "amount": (100 + i) * 1_000_000_000_000u128 }),
            chain_id: chain_id.clone(),
        };
        events.insert(event_id, event);
    }
    Arc::new(Mutex::new(MockEventStore {
        events,
        event_sender: tx,
        chain_id,
    }))
});

#[derive(Clone)]
pub struct SubstrateIndexerService {
    config: Arc<AppConfig>, // Share config via Arc
    // In a real app, this might hold a DB connection pool or an HTTP client for the indexer
    event_store: Arc<Mutex<MockEventStore>>,
}

impl SubstrateIndexerService {
    #[instrument(skip(config))]
    pub fn new(config: AppConfig) -> Self {
        info!("Initializing SubstrateIndexerService");
        Self {
            config: Arc::new(config),
            event_store: MOCK_EVENT_STORE.clone(),
        }
    }

    #[instrument(skip(self))]
    pub async fn get_chain_info(&self) -> FieldResult<ChainInfo> {
        // In a real scenario, this might involve an async call.
        // For Dataloader, we might need a version that takes multiple IDs.
        Ok(MOCK_CHAIN_INFO.clone())
    }
    
    // Example for Dataloader: batch fetch chain_infos
    #[instrument(skip(self, ids))]
    pub async fn get_chain_infos_batch(&self, ids: &[ID]) -> FieldResult<HashMap<ID, ChainInfo>> {
        let mut result = HashMap::new();
        for id in ids {
            // Simulate fetching; in our mock, only one chain info exists
            if *id == MOCK_CHAIN_INFO.id {
                result.insert(id.clone(), MOCK_CHAIN_INFO.clone());
            }
        }
        Ok(result)
    }

    #[instrument(skip(self))]
    pub async fn get_event_by_id(&self, id: ID) -> FieldResult<Option<Event>> {
        let store = self.event_store.lock().map_err(|e| AppError::Internal(format!("Failed to lock event store: {}", e)))?;
        Ok(store.events.get(&id).cloned())
    }

    #[instrument(skip(self, filter))]
    pub async fn list_events(&self, filter: Option<EventFilterInput>) -> FieldResult<Vec<Event>> {
        let store = self.event_store.lock().map_err(|e| AppError::Internal(format!("Failed to lock event store: {}", e)))?;
        let mut events: Vec<Event> = store.events.values().cloned().collect();

        if let Some(f) = filter {
            events.retain(|e| {
                let mut matches = true;
                if let Some(pallet_name) = &f.pallet_name_eq {
                    matches &= &e.pallet_name == pallet_name;
                }
                if let Some(event_name) = &f.event_name_eq {
                    matches &= &e.event_name == event_name;
                }
                if let Some(bn_gte) = f.block_number_gte {
                    matches &= e.block_number >= bn_gte;
                }
                if let Some(bn_lte) = f.block_number_lte {
                    matches &= e.block_number <= bn_lte;
                }
                matches
            });
        }
        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Sort by newest first
        Ok(events)
    }

    #[instrument(skip(self))]
    pub async fn watch_events(&self) -> impl Stream<Item = Event> {
        let rx = self.event_store.lock().unwrap().event_sender.subscribe(); // unwrap is fine here for a global static
        BroadcastStream::new(rx).filter_map(|result| async move {
            match result {
                Ok(event) => Some(event),
                Err(e) => {
                    error!("Broadcast receive error: {}", e);
                    None
                }
            }
        })
    }

    #[instrument(skip(config))]
    pub fn simulate_new_event(config: AppConfig) { // Pass config directly or use Arc
        info!("Starting mock event simulation task.");
        let min_delay = config.mock_event_min_delay_secs;
        let max_delay = config.mock_event_max_delay_secs;
        let event_store_arc = MOCK_EVENT_STORE.clone();

        tokio::spawn(async move {
            let mut rng = rand::thread_rng();
            loop {
                let delay_secs = rng.gen_range(min_delay..=max_delay);
                tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
                
                let mut store_guard = event_store_arc.lock().unwrap(); // unwrap is fine here, panic on poison is intended for global static
                let event_id = ID::from(Uuid::new_v4().to_string());
                let block_num = store_guard.events.values().map(|e| e.block_number).max().unwrap_or(10000) + rng.gen_range(1..5);
                
                let (pallet_name, event_name, data) = if rng.gen() {
                    ("System".to_string(), "NewAccount".to_string(), json!({ "account": Uuid::new_v4().to_string(), "balance": rng.gen_range(0..1000) }))
                } else {
                    ("Timestamp".to_string(), "TimestampSet".to_string(), json!({ "now": Utc::now().timestamp_millis() }))
                };

                let new_event = Event {
                    id: event_id.clone(),
                    block_number: block_num,
                    extrinsic_id: Some(format!("0x{}", Uuid::new_v4().to_simple())),
                    timestamp: Utc::now(),
                    pallet_name: pallet_name.clone(),
                    event_name: event_name.clone(),
                    data,
                    chain_id: store_guard.chain_id.clone(),
                };

                store_guard.events.insert(event_id, new_event.clone());
                match store_guard.event_sender.send(new_event.clone()) {
                    Ok(receivers) => info!(event_id = %new_event.id, %pallet_name, %event_name, receivers, "Simulated and broadcasted new event."),
                    Err(e) => error!("Failed to broadcast new event: {}", e),
                }
            }
        });
    }
} 