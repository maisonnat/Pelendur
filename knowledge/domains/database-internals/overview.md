# Database Internals - Domain Knowledge

## Overview
Bases de datos son el corazón de todo sistema. Este dominio cubre cómo funcionan por dentro — desde cómo se organizan los datos en disco hasta cómo se ejecutan las queries, se manejan transacciones concurrentes y se replica data entre nodos. Entender estos internals separa a un senior engineer que sabe qué DB usar y por qué, de alguien que solo sabe escribir SQL.

## Key Concepts

### B-Trees vs LSM Trees
- **B-Trees**: Árbol balanceado donde cada nodo es un bloque/página. Lecturas rápidas (O(log n) con fanout alto). Writes in-place. Usado en PostgreSQL, MySQL (InnoDB), Oracle. Ventajas: buenas reads, fuerte consistencia. Desventajas: write amplification por writes in-place, fragmentación.
- **LSM Trees (Log-Structured Merge)**: Writes van a un memtable en RAM, luego se flush a SSTables inmutables en disco. Compaction en background mergea SSTables. Usado en Cassandra, RocksDB, LevelDB, BigTable. Ventajas: writes ultrarrápidos (solo append), compresión eficiente. Desventajas: reads lentas (puede necesitar revisar múltiples SSTables), write amplification por compaction.
- **Bloom Filters**: Los LSM trees usan bloom filters por SSTable para evitar revisar tablas que no contienen la key.
- **Práctica**: B-Tree para OLTP con muchas reads/updates (Postgres). LSM para writes intensivos (time series, eventos, IoT).

### Query Optimization
- **Planner**: Genera un plan de ejecución. Evalúa múltiples estrategias (sequential scan, index scan, bitmap scan, join methods) y elige la de menor costo estimado basado en estadísticas.
- **Indexes**: B-tree index (equality + range), Hash index (equality only), GiST/GIN (full-text, geospatial), BRIN (data correlacionada). Composite indexes — order matters: `(a, b, c)` cubre queries con `a`, `a+b`, `a+b+c` pero NO `b` solo.
- **Join Strategies**: Nested Loop Join (bueno para datasets chicos), Hash Join (bueno para tablas grandes sin índice), Merge Join (bueno para datos ordenados).
- **Explain Plans**: Leer un `EXPLAIN ANALYZE` — buscar sequential scans en tablas grandes, estimated vs actual rows (indican estadísticas desactualizadas), loops excesivos.
- **Práctica**: `VACUUM ANALYZE` periódico (Postgres). Indexar columnas de WHERE, JOIN, ORDER BY. Cuidado con over-indexing (más writes lentos).

### Transaction Isolation Levels
- **Read Uncommitted**: Puede leer dirty writes. Raramente usado.
- **Read Committed**: Solo ve commits confirmados. Previene dirty reads pero no non-repeatable reads. Default en PostgreSQL, SQL Server, Oracle.
- **Repeatable Read**: Misma lectura dentro de la transacción da el mismo resultado. Previene non-repeatable reads pero no phantom reads. Default en MySQL/InnoDB.
- **Serializable**: Las transacciones se ejecutan como si fueran secuenciales. Previene todos los anomalies (dirty read, non-repeatable read, phantom). Más lento pero más seguro.
- **MVCC (Multi-Version Concurrency Control)**: Cada transacción ve un snapshot del dato al momento que empezó. Las escrituras crean nuevas versiones, no sobrescriben. Permite alta concurrencia sin locks de lectura. Implementaciones: PostgreSQL (cada versión es una tupla), MySQL/InnoDB (undo log).
- **Anomalies**: Dirty read, non-repeatable read, phantom read, serialization anomaly, write skew.

### Replication
- **WAL Shipping (Physical Replication)**: Enviar el Write-Ahead Log del primary al standby. El standby reproduce los cambios. Usado en PostgreSQL streaming replication. Más bajo nivel y preciso.
- **Statement-Based Replication**: Replicar las queries SQL literales. Más compacto pero frágil (funciones no-determinísticas como NOW() o RAND() dan resultados distintos). Usado en MySQL (binlog statement mode).
- **Logical Replication**: Replicar cambios a nivel de fila (insert, update, delete) como eventos. Más flexible — permite replicar subsets de tablas, distintos schemas. Usado en PostgreSQL logical replication, Debezium (CDC).
- **Synchronous vs Asynchronous**: Sync: el primary espera confirmación del replica antes de committear (data loss = 0 pero latency alta). Async: el primary commitea sin esperar (performance pero posible data loss).
- **Práctica**: PostgreSQL streaming replication para HA (synchronous para transacciones críticas). Logical replication para migraciones (pglogical). CDC con Debezium + Kafka para event sourcing.

### Buffer Pool & Page Cache
- **Buffer Pool**: Área en RAM donde la DB cachea páginas/pags de datos. El corazón del performance de cualquier DB. PostgreSQL lo llama shared_buffers, MySQL/InnoDB lo llama innodb_buffer_pool_size.
- **Page eviction policies**: Clock-scan (PostgreSQL), LRU (MySQL). Tuning: shared_buffers típicamente 25% de RAM en Postgres.
- **Double Buffering**: Cuando el OS también cachea (page cache) + la DB cachea (buffer pool) = dos copias en RAM. PostgreSQL recomienda shared_buffers modesto (25%) y dejar el resto al OS. InnoDB prefiere un buffer pool grande (70-80%).
- **Práctica**: Monitorear cache hit ratio (debe ser >99%). Si es bajo, aumentar buffer pool o revisar índices.

### Concurrency Control
- **2PL (Two-Phase Locking)**: Fase de expansión (adquirir locks) y fase de contracción (soltarlos). Garantiza serializability pero puede causar deadlocks. MySQL usa 2PL con detección de deadlocks (victim rollback).
- **OCC (Optimistic Concurrency Control)**: Lee sin locks, valida al commit. Bueno para workloads con baja contención. Usado en VoltDB, FoundationDB.
- **MVCC**: Versiones múltiples permiten lecturas sin bloqueo. Las escrituras lockean solo la fila objetivo. PostgreSQL usa una combinación de MVCC + SI (Snapshot Isolation). Previene write skew solo en Serializable.
- **Deadlock Detection**: La DB detecta ciclos de locks y elige un victim (la transacción más joven o con menor costo) para rollback. PostgreSQL retorna error 40P01. MySQL deadlock automático.
- **Práctica**: Mantener transacciones cortas. Acceder las tablas en el mismo orden siempre. Usar `NOWAIT` o `SKIP LOCKED` cuando sea aceptable.

## Common Interview Questions

1. **"Explicá la diferencia entre B-Trees y LSM Trees. ¿Cuándo usarías cada uno?"**
   B-Tree: reads rápidas, writes in-place, buena consistencia. Ideal para OLTP (Postgres). LSM Tree: writes secuenciales muy rápidos, compresión eficiente, compaction overhead. Ideal para time-series, IoT, write-heavy (Cassandra, RocksDB). En la práctica: sistemas de logging → LSM, sistemas financieros → B-Tree.

2. **"¿Cómo leés y debuggeás un plan de ejecución de query lenta en PostgreSQL?"**
   `EXPLAIN ANALYZE` la query. Buscar: sequential scans en tablas grandes (falta índice), row estimates vs actual (estadísticas desactualizadas), nested loops excesivos. Fix: agregar índice compuesto, ajustar `work_mem` para sorts, `VACUUM ANALYZE` para estadísticas frescas.

3. **"Explicá MVCC. ¿Cómo mantiene PostgreSQL las versiones de filas?"**
   Cada tupla tiene `xmin` (transacción que la creó) y `xmax` (transacción que la eliminó/modificó). Cada transacción ve un snapshot: tuplas con xmin commitado y xmax no commitado o invisible. Las tuplas muertas son limpiadas por VACUUM. Esto permite lecturas sin locks.

4. **"¿Qué niveles de aislamiento existen y qué anomalías previene cada uno?"**
   Read Uncommitted (solo dirty read previene), Read Committed (+ non-repeatable read previene), Repeatable Read (+ phantom read previene), Serializable (previene todo). PostgreSQL trata Read Uncommitted como Read Committed. Serializable usa SSI (Serializable Snapshot Isolation) para detectar conflictos serializables.

5. **"Diseñá un sistema de replicación con failover automático para una DB PostgreSQL."**
   Streaming replication síncrona con 2 standbys. Patroni para HA: monitorea health de nodos, maneja failover automático. Etcd/Consul para consensus. Cuando el primary falla, Patroni promueve el standby mejor posicionado. PgBouncer para connection pooling con auto-detection del nuevo primary. WAL archiving con WAL-G para PITR.

6. **"¿Qué es write amplification y cómo se mitiga en LSM Trees?"**
   Write amplification = datos escritos en disco / datos nuevos. En LSM: cada compaction reescribe datos. Tiered compaction (Cassandra) tiene menos amplification que leveled compaction (RocksDB). Mitigación: ajustar size ratio entre niveles, usar bloom filters, elegir compaction strategy según workload.

7. **"¿Cómo funciona el buffer pool? ¿Cómo tuneás shared_buffers?"**
   Buffer pool cachea páginas en RAM. En Postgres: shared_buffers ~25% de RAM. El resto va al OS page cache (Postgres confía en el OS para I/O). En InnoDB: buffer pool ~70-80% de RAM (InnoDB maneja su propio I/O). Monitorear con `pg_buffercache` (Postgres) o `Innodb_buffer_pool_reads` (MySQL).

8. **"Explicá la diferencia entre optimistic y pessimistic concurrency control."**
   Pessimistic (2PL): locks preventivos. Bueno para alta contención. Riesgo de deadlocks. Optimistic (OCC): sin locks, valida al commit. Bueno para baja contención. Si hay conflicto, rollback. MVCC es un híbrido: lectores no bloquean escritores ni viceversa.

9. **"¿Qué son los hot spots en indexing y cómo los evitás?"**
   Hot spots: inserts secuenciales en el mismo bloque del B-Tree (ej: auto-increment IDs). Causa contención en writes y page splits. Soluciones: usar UUIDs o keys aleatorias (pero fragmentan el índice), hash-partitioning, usar HASH index o BRIN para datos secuenciales.

10. **"Diseñá un sistema de CDC (Change Data Capture) para sincronizar una DB transaccional con un data warehouse."**
    Debezium + Kafka. Debezium se conecta al WAL de PostgreSQL (logical replication slot) y emite eventos CDC a Kafka topics. Un consumer escribe al DWH (Snowflake/BigQuery/Redshift). Schema registry asegura compatibilidad. Para backfills: snapshot inicial + streaming continuo.

## STAR Story Triggers
- B-Tree, LSM Tree, SSTable, memtable, compaction, write amplification, Bloom filter, index, composite index, query plan, EXPLAIN ANALYZE, sequential scan, index scan, join, nested loop, hash join, merge join, MVCC, transaction isolation, read committed, repeatable read, serializable, snapshot isolation, dirty read, phantom read, non-repeatable read, write skew, 2PL, OCC, deadlock, buffer pool, shared_buffers, page cache, WAL, replication, streaming replication, logical replication, CDC, Debezium, failover, Patroni, PITR, hot spot, sharding, partitioning, connection pooling, PgBouncer, vacuum, autovacuum, analyze, statistics, cardinality, optimizer, planner, cost model
