# Backend at Scale - Domain Knowledge

## Overview
Backend engineering at scale covers the patterns, tradeoffs, and architectural decisions required to build systems that handle high traffic, large data volumes, and strict reliability requirements. This domain covers distributed systems theory applied to real-world backend services.

## Key Concepts

### Distributed Systems Fundamentals
- **CAP Theorem**: A distributed system can provide at most two of: Consistency, Availability, and Partition tolerance. In practice, partition tolerance is mandatory, so the real tradeoff is CP vs AP.
- **Consistency Models**: Strong consistency (linearizable), eventual consistency, causal consistency, read-after-write consistency. Each has latency and availability implications.
- **Idempotency**: Operations that produce the same result when executed multiple times. Critical for retry-safe APIs and message processing.
- **Exactly-once Semantics**: Achieved through idempotency + deduplication, not through transport guarantees alone.

### Database Patterns
- **Connection Pooling**: PgBouncer (transaction-mode) for PostgreSQL. Avoid one-connection-per-request patterns.
- **Read Replicas**: Scale read throughput by routing queries to replicas. Requires awareness of replication lag.
- **Sharding/Partitioning**: Horizontal scaling by distributing data across nodes. Key challenges: cross-shard queries, rebalancing, hotspot avoidance.
- **Indexing Strategy**: Composite indexes for common query patterns, covering indexes to avoid heap lookups, partial indexes for filtered queries. Monitor index usage with `pg_stat_user_indexes`.
- **CQRS**: Separate read and write models to optimize each independently. Event sourcing often pairs with CQRS.

### Caching Strategies
- **Cache-aside (Lazy Loading)**: Application checks cache first, on miss loads from DB and populates cache.
- **Write-through**: Writes go to cache and DB simultaneously. Always consistent but higher write latency.
- **Write-behind**: Writes go to cache first, asynchronously flushed to DB. Higher throughput with risk of data loss.
- **Cache Invalidation**: Time-based TTL, event-driven invalidation, versioned keys. "There are only two hard things in Computer Science: cache invalidation and naming things."
- **Thundering Herd**: When a cached item expires and many requests hit the DB simultaneously. Mitigate with lock-and-load or probabilistic early expiration.

### API Design
- **Pagination**: Cursor-based (stable, efficient) vs offset-based (simple but slow on large datasets).
- **Rate Limiting**: Token bucket, sliding window, fixed window algorithms. Essential for public APIs.
- **Versioning**: URL path (/v1/), header-based, query parameter. Each has tradeoffs in coupling and discoverability.
- **Backpressure**: When downstream is slow, propagate pressure upstream rather than buffering indefinitely.

### Message Queues and Event Streaming
- **Kafka**: Durable, ordered, replayable event log. Topics partitioned for parallel consumption. At-least-once delivery by default.
- **RabbitMQ**: Traditional message broker with sophisticated routing. Messages acknowledged after processing.
- **Dead Letter Queues**: Where failed messages go for inspection. Essential for reliability.
- **Schema Registry**: Enforce contracts on event payloads (Avro, Protobuf). Prevents breaking consumers.

## Common Interview Questions

1. "Design a URL shortener that handles 100M new URLs per day."
2. "How would you handle a database that's become the bottleneck in your system?"
3. "Explain the CAP theorem. Which side would you choose for a payment system and why?"
4. "How do you ensure exactly-once processing in a distributed message system?"
5. "Your service's p99 latency is dominated by database queries. What optimization strategies would you apply?"
6. "Design a rate limiter for a public API. What algorithm would you use and why?"
7. "How do you handle schema migrations in a zero-downtime deployment?"
8. "Explain the thundering herd problem. How would you prevent it in a caching layer?"
9. "Compare Kafka and RabbitMQ. When would you choose one over the other?"
10. "How would you design for multi-region data replication with low read latency?"

## STAR Story Triggers
- scalability, performance, latency, throughput, caching, database, PostgreSQL, Redis, microservices, API, Kafka, message queue, distributed, partitioning, sharding, replication, connection pool, N+1, optimization, bottleneck, idempotent, rate limit, backpressure, event-driven, CQRS, event sourcing
