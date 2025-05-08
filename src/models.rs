use async_graphql::{SimpleObject, InputObject, ID};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(complex)] // Indicates that we will have complex fields resolved by methods
pub struct ChainInfo {
    pub id: ID,
    pub name: String,
    pub version: String,
    pub token_symbol: String,
    pub decimals: u8,
    pub ssv58_prefix: u16, // Substrate's SS58 address format prefix
    #[graphql(skip)] // Often metadata doesn't change, so skip for now in direct event generation
    pub last_updated: DateTime<Utc>,
}

#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(complex)] // Indicates that we will have complex fields resolved by methods
pub struct Event {
    pub id: ID,
    pub block_number: u64,
    pub extrinsic_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub pallet_name: String,
    pub event_name: String,
    // Store data as JSON string for flexibility, or define specific structures if known
    pub data: serde_json::Value, 
    #[graphql(skip)]
    pub chain_id: ID, // Foreign key to ChainInfo
}


#[derive(InputObject, Debug)]
pub struct EventFilterInput {
    pub pallet_name_eq: Option<String>,
    pub event_name_eq: Option<String>,
    pub block_number_gte: Option<u64>,
    pub block_number_lte: Option<u64>,
}

// Example of how you might represent some event data more concretely
#[derive(SimpleObject, Clone, Debug, Serialize, Deserialize)]
pub struct TransferEventData {
    from: String, // Typically an AccountId string
    to: String,   // Typically an AccountId string
    amount: u128,
}

// Placeholder for what might come from an indexer
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RawEventData {
    pub pallet: String,
    pub variant: String,
    pub fields: serde_json::Value, // This could be a map or a list depending on the event
} 