mod models;
mod indexer;
mod schema;
mod config;
mod errors;
mod dataloader;

use actix_web::{guard, web, App, HttpRequest, HttpResponse, HttpServer, middleware::Logger as ActixLogger};
use async_graphql::http::{GraphQLPlaygroundConfig, playground_source};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use async_graphql::{EmptyMutation, Schema, extensions};
use schema::{AppSchema, QueryRoot, SubscriptionRoot};
use indexer::SubstrateIndexerService;
use dataloader::{AppDataloader, ChainInfoLoader};
use crate::config::{CONFIG, ensure_config_files_exist, AppConfig};
use crate::errors::AppError;

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

async fn gql_playground() -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(
            GraphQLPlaygroundConfig::new("/graphql").subscription_endpoint("/ws"),
        )))
}

async fn gql_request(schema: web::Data<AppSchema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn gql_ws(
    schema: web::Data<AppSchema>,
    http_req: HttpRequest,
    payload: web::Payload,
) -> Result<HttpResponse, actix_web::Error> { // actix_web::Error is compatible with AppError via From trait if needed or map directly
    GraphQLSubscription::new(Schema::clone(&*schema))
        .start(&http_req, payload)
        .await
}

fn init_tracer() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(CONFIG.logger.level.clone()));

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .with_span_events(FmtSpan::CLOSE) // Log when spans close
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default tracing subscriber");
}

#[actix_web::main]
async fn main() -> Result<(), AppError> {
    dotenvy::dotenv().ok();
    ensure_config_files_exist()?;
    init_tracer();

    let app_config = CONFIG.clone(); // Clone once for multiple uses
    tracing::info!("Starting service with config: {:?}", app_config);

    let indexer_service = SubstrateIndexerService::new(app_config.clone());
    
    // Start the mock event generator with its own config clone
    SubstrateIndexerService::simulate_new_event(app_config.clone());

    // Create Dataloader
    let chain_info_loader = ChainInfoLoader::new(indexer_service.clone());
    let dataloader = AppDataloader::new(chain_info_loader, tokio::spawn);

    let schema = Schema::build(QueryRoot, EmptyMutation, SubscriptionRoot)
        .data(indexer_service)      // Indexer service for direct calls
        .data(dataloader)           // Dataloader for batched calls
        .data(app_config)           // App config if needed directly in resolvers
        .extension(extensions::Logger)      // Built-in logger
        .extension(extensions::Tracing)     // Tracing integration
        .extension(extensions::Analyzer)    // Query analyzer (helps prevent overly complex queries)
      //.extension(extensions::ApolloTracing) // If you need Apollo Tracing format
        .finish();

    let server_addr = CONFIG.server.address(); // Use original CONFIG for server address to avoid clone issues if it were mutable
    tracing::info!("Playground: http://{}/", server_addr);
    tracing::info!("GraphQL endpoint: http://{}/graphql", server_addr);
    tracing::info!("GraphQL subscription WebSocket: ws://{}/ws", server_addr);

    HttpServer::new(move || {
        App::new()
            .wrap(ActixLogger::default())
            .app_data(web::Data::new(schema.clone()))
            .service(web::resource("/").guard(guard::Get()).to(gql_playground))
            .service(web::resource("/graphql").guard(guard::Post()).to(gql_request))
            .service(
                web::resource("/ws")
                    .guard(guard::Get())
                    .guard(guard::Header("upgrade", "websocket"))
                    .to(gql_ws),
            )
    })
    .bind(server_addr)?
    .run()
    .await
    .map_err(AppError::Io)
} 