# Chain Metadata GraphQL Service

A GraphQL gateway built in Rust to expose chain metadata and event queries from Substrate indexers.

## Features

- GraphQL API for chain metadata
- GraphQL API for event queries
- Real-time subscriptions for events
- Batching for improved client performance

## TODO

- [ ] Initial project setup
- [ ] Basic GraphQL schema with `async-graphql`
- [ ] HTTP server setup with `actix-web`
- [ ] Define types for Substrate metadata
- [ ] Define types for Substrate events
- [ ] Implement query resolvers for metadata
- [ ] Implement query resolvers for events
- [ ] Placeholder for Substrate indexer interaction logic
- [ ] Implement GraphQL subscriptions
- [ ] Implement query batching
- [ ] Configuration (indexer endpoints, etc.)
- [ ] Error handling
- [ ] Logging
- [ ] Testing
- [ ] Documentation 