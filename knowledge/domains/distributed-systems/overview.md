# Distributed Systems - Domain Knowledge

## Overview
Sistemas distribuidos son colecciones de computadoras independientes que se presentan como un sistema único y coherente. Este dominio cubre los patrones, tradeoffs y principios fundamentales necesarios para diseñar sistemas que operen de manera confiable a través de múltiples nodos, redes no confiables y fallos parciales. Es el núcleo de toda arquitectura moderna a escala — desde bases de datos distribuidas hasta sistemas de streaming y microservicios.

## Key Concepts

### Consensus Algorithms
- **Paxos**: El algoritmo teórico original para consenso en sistemas distribuidos. Fases: Prepare/Promise, Accept/Accepted. Garantiza seguridad (safety) bajo fallos de nodos pero es notoriamente difícil de implementar correctamente. Usado en Chubby (Google) y ZooKeeper.
- **Raft**: Diseñado como alternativa comprensible a Paxos. Divide el problema en: leader election, log replication, safety. Usa términos (terms) como épocas lógicas. Un líder elegido por mayoría, los followers replican entradas. Los logs deben committearse en una mayoría antes de aplicarse. Usado en etcd, Consul, Kafka (KRaft desde 2.8).
- **Zab (ZooKeeper Atomic Broadcast)**: Similar a Raft pero diseñado para el patrón primary-backup de ZooKeeper. Procesa writes secuencialmente en el líder, reads desde cualquier nodo.
- **Práctica**: Elección de líder requiere `⌊n/2⌋ + 1` votos (quorum mayoritario). Con n impares se toleran `⌊(n-1)/2⌋` fallos. Para tolerar 2 fallos → 5 nodos.

### CAP Theorem y Tradeoffs
- **CAP**: Un sistema distribuido puede proveer máximo dos de: Consistency, Availability, Partition Tolerance. Particiones son inevitables en redes reales, por lo que el tradeoff real es CP vs AP.
- **CP (Consistency + Partition Tolerance)**: Prioriza que todos vean el mismo estado. Si hay partición, se rechazan writes hasta que sane. Ej: sistemas bancarios, ZooKeeper.
- **AP (Availability + Partition Tolerance)**: Prioriza que el sistema responda siempre. Puede devolver datos obsoletos durante una partición. Ej: DNS, sistemas de recomendación.
- **PACELC**: Extensión que considera también el comportamiento normal (sin partición): Tradeoff entre Latency y Consistency incluso cuando no hay partición. DynamoDB elige AC (availability + latency) con consistencia eventual; sistemas SQL eligen PC (partition-tolerant + consistent).
- **Práctica**: Para sistemas reales, elegir el consistency model correcto según la operación (no todo el sistema). Ej: en un e-commerce, el catálogo puede ser eventualmente consistente, pero el checkout requiere consistencia fuerte.

### Sharding y Partitioning
- **Range-based**: Particionar datos por rango de valores (ej: user_id 1-10000 → shard A). Simple pero puede causar hotspots si los rangos no están balanceados.
- **Hash-based**: Usar hash(key) % N para asignar shard. Distribuye uniformemente pero añadir/remover shards requiere rehash de todos los datos → **Consistent Hashing** resuelve esto minimizando el rebalanceo.
- **Directory-based**: Mantener un lookup service que sabe qué shard tiene qué dato. Flexible pero introduce latencia y single point of failure (SPOF) en el lookup.
- **Desafíos clave**: (1) Cross-shard queries son costosas (scatter-gather). (2) Rebalanceo cuando se agregan nodos. (3) Hotspot avoidance cuando una key es muy popular (ej: celebrity users en Twitter).
- **Práctica**: Usar shard key basada en el patrón de acceso, no en el modelo de datos. Monitorear skew. Cassandra usa consistent hashing con virtual nodes. MongoDB usa range-based con chunks.

### Consistency Models
- **Strong Consistency (Linearizability)**: Todas las operaciones parecen ejecutarse en orden atómico. Cada read ve el write más reciente. Más lento pero predecible. Requiere coordination entre nodos.
- **Eventual Consistency**: Sin writes nuevos, todos los reads eventualmente retornan el mismo valor. Barato, altamente disponible. Usado en DNS, DynamoDB.
- **Causal Consistency**: Las operaciones relacionadas causalmente se ven en orden. Operaciones concurrentes pueden verse en distinto orden. Compromiso entre strong y eventual.
- **Read-after-write (Read-your-writes)**: Un cliente siempre ve lo que escribió. Esencial para UX (ej: después de publicar un tweet, debe aparecer inmediatamente).
- **Session Consistency**: Consistencia solo dentro de la sesión de un cliente. Los cambios son visibles para ese cliente, pero no necesariamente para otros.
- **Quorum-based**: Controlar consistencia via N (total replicas), W (write quorum), R (read quorum). W + R > N → strong consistency. W + R <= N → eventual consistency.

### Distributed Transactions
- **2PC (Two-Phase Commit)**: Coordinator envía prepare a todos los participantes. Todos votan yes/no. Si todos yes → commit, si algún no → abort. Problemas: bloqueante si el coordinator falla, no tolera particiones de red.
- **3PC (Three-Phase Commit)**: Agrega fase de pre-commit para mitigar el bloqueo. Tolera mejor ciertos fallos pero sigue sin resolver particiones de red.
- **SAGA**: Secuencia de transacciones locales con compensaciones. Cada paso tiene un undo. No requiere locks globales. Más escalable que 2PC. Usado en microservicios. Orquestada (un coordinator dirige) o coreografiada (cada servicio escucha eventos y reacciona).
- **TCC (Try-Confirm/Cancel)**: Fase try (reserva recursos), fase confirm (ejecuta), o cancel (libera). Usado en sistemas financieros.
- **Práctica**: Preferir SAGA sobre 2PC en microservicios. 2PC tiene sentido cuando la atomicidad es más importante que la disponibilidad (ej: transferencias bancarias).

### Tiempo en Sistemas Distribuidos
- **Clock Skew**: Los relojes físicos en distintas máquinas nunca están perfectamente sincronizados. NTP reduce skew pero no lo elimina (típicamente 1-50ms).
- **Lamport Clocks**: Relojes lógicos que capturan orden causal. Cada nodo mantiene un contador. Inconsistente: C(A) < C(B) no implica que A ocurrió antes que B.
- **Vector Clocks**: Cada nodo mantiene un vector [nodo1: contador, nodo2: contador, ...]. Permiten detectar concurrencia y causalidad. Usados en DynamoDB. Escalan O(n) — problemático con muchos nodos.
- **Hybrid Logical Clocks (HLC)**: Combina reloj físico + lógico. Captura tiempo real bound + orden causal. Menor overhead que vector clocks. Usado en CockroachDB.
- **Práctica**: Para detectar conflictos en sistemas multi-leader o CRDTs, usar vector clocks. Para timestamping de eventos con bound real, usar HLC.

### Leader Election y Coordination
- **Leader Election**: Proceso para elegir un nodo líder en un clúster. Implementaciones: Bully algorithm (elige el nodo con mayor ID), Raft (votación por términos), ZooKeeper (secuential ephemeral znodes).
- **Distributed Locks**: Mecanismo para asegurar acceso exclusivo a un recurso compartido. Implementaciones: Redis Redlock (controvertido — Martin Kleppmann demostró fallos), ZooKeeper locks, etcd con TTL.
- **Gossip Protocol**: Propagación de información epidémica. Cada nodo intercambia estado con un subconjunto aleatorio de nodos periódicamente. Usado en Cassandra, AWS DynamoDB, Consul. Convergencia O(log N). Parámetros: fanout (cuántos nodos contactar cada ronda), TTL (cuántas rondas vive un mensaje).
- **Práctica**: No implementar consensus desde cero — usar etcd, ZooKeeper o Consul. Raft es más fácil de entender que Paxos pero igual de fácil de implementar incorrectamente.

### Failure Detection y Recovery
- **Phi Accrual Failure Detector**: Detecta fallos basado en la probabilidad de que un heartbeat tarde más de lo esperado. No usa timeout fijo — el threshold es adaptativo. Usado en Cassandra y Akka.
- **SWIM (Scalable Weakly-consistent Infection-style)**: Protocolo de membership + failure detection. Cada nodo pinge un peer aleatorio. Si no hay respuesta, pide a otro nodo que haga ping indirecto (suspicion). Si el sospechoso no responde, se declara muerto y se propaga por gossip. Usado en HashiCorp Serf/Consul.
- **Split Brain**: Escenario donde dos nodos creen ser el líder. Prevención con fencing tokens (mecanismo que invalida writes de líderes depuestos), majority quorum (un líder solo con mayoría), STONITH (Shoot The Other Node In The Head).

### Replication
- **Single-Leader (Master-Slave)**: Un nodo acepta writes, replica a followers. Simple, consistencia fuerte posible. Punto único de fallo para writes.
- **Multi-Leader (Master-Master)**: Varios nodos aceptan writes. Mayor disponibilidad pero conflictos de escritura. Usado en sistemas multi-DC (CouchDB, MySQL group replication).
- **Leaderless (Dynamo-style)**: Cualquier nodo acepta writes. Usa read repair + anti-entropy para convergencia. Usado en Cassandra, Riak, AWS DynamoDB.
- **Conflict Resolution**: Last-Write-Wins (LWW) con timestamp, CRDTs (Conflict-free Replicated Data Types), aplicación de merging custom.

## Common Interview Questions

1. **"Diseña un sistema de almacenamiento clave-valor distribuido como DynamoDB."**
   Clave: partition con consistent hashing, replicación N=3, read repair + vector clocks para conflictos, gossip para membership. Explicar quorum W/R configurables y hint-handoff durante fallos.

2. **"¿Cómo funciona Raft? Describí el proceso de leader election."**
   Tres estados: follower, candidate, leader. Candidates piden votos con election timeout aleatorio (150-300ms). Mayoría de votos → leader. El líder envía heartbeats periódicos. Si un follower no recibe heartbeat → term expira → nueva elección. Ejemplo: 5 nodos → tolera 2 fallos.

3. **"Explicá la diferencia entre consistencia eventual y strong consistency. ¿Cuándo usarías cada una?"**
   Strong: reads siempre ven el último write. Requiere coordination entre réplicas (W+R > N). Usar en pagos, inventarios. Eventual: sin writes nuevos, converge. Usar en timelines sociales, analytics. En la práctica: sistemas híbridos con consistencia configurable por operación (ej: DynamoDB con ConsistentRead=true/false por query).

4. **"¿Cómo implementarías transacciones distribuidas sin 2PC?"**
   SAGA pattern. Orquestada: un coordinator que maneja la secuencia + compensaciones. Coreografiada: cada servicio escucha eventos y ejecuta su paso o su rollback. Ejemplo práctico: reserva de viaje — reservar vuelo → reservar hotel → pagar. Si hotel falla → cancelar vuelo (compensación).

5. **"Diseñá un sistema de mensajería como WhatsApp que garantice orden de mensajes."**
   Servidores asignan timestamp lógico (Lamport clock por chat). Cada mensaje tiene un server-issued seq id. Offline delivery: store-and-forward. Conflictos: last-writer-wins por mensaje. Escalar con particionamiento por chat_id (cada chat en un servidor, replicado para HA).

6. **"¿Qué es consistent hashing? ¿Por qué es mejor que hash modular para sistemas distribuidos?"**
   Hash(key) mapea a un punto en un anillo circular. Los nodos también se mapean al anillo. Cada key va al siguiente nodo en sentido horario. Agregar/quitar nodos solo afecta las keys vecinas inmediatas (no todo el dataset). Virtual nodes mitigan skew. Ej: Cassandra con 256 virtual nodes por nodo físico.

7. **"Explicá la paradoja del CAP con PACELC. ¿Cómo la aplicás al diseñar una base de datos global?"**
   PACELC: Si hay Partition → tradeoff C vs A; Si no hay (Else) → tradeoff L (latency) vs C (consistency). Para DB global con multi-region writes: elegir AP durante particiones (eventual consistency con conflict resolution vía CRDTs). Sin partición: priorizar latencia local sobre consistencia cross-region. CockroachDB elige CP (global consistency via Raft) a costa de latency en writes cross-DC.

8. **"How does Google Spanner achieve external consistency (linearizability) across global data centers?"**
   TrueTime API (GPS + atomic clocks) provee un bound de incertidumbre de timestamp (ε ~1-7ms). Spanner asigna timestamps con commit-wait: espera ε después del TrueTime actual antes de hacer visible un commit. Esto garantiza que todos los relojes ven el orden correcto. Usa Paxos para consensus y 2PC para transacciones cross-shard.

9. **"¿Qué problema resuelven los CRDTs y cómo funcionan?"**
   CRDTs permiten que réplicas se actualicen concurrentemente sin coordinación y converjan al mismo estado. Dos tipos: state-based (merge de estados completos) y operation-based (merge de ops commutativas). Ejemplos: G-Counter (solo incrementa, suma de contadores por réplica), OR-Set (set con add/remove, usando tags para resolver add-before-remove). Usados en Riak, Redis (CRDT maps), Figma (colaboración real-time).

10. **"Diseñá un sistema de rate limiting distribuido que funcione a nivel global."**
    Dos enfoques: (1) Centralizado: Redis cluster con sorted sets (sliding window). Problema: latencia cross-region. (2) Distribuido: cada región hace rate limiting local con un pequeño overhead compartido via gossip. Usar token bucket por región + sincronización periódica. Preferir sliding window log para evitar burst abuse. Clave: usar consistencia eventual para rate limiting (un par de requests extra durante sincronización es aceptable).

## STAR Story Triggers
- distributed, consensus, Raft, Paxos, Zab, leader election, quorum, majority, replica, replication, sharding, partitioning, consistent hashing, CAP theorem, PACELC, consistency model, strong consistency, eventual consistency, causal consistency, read-your-writes, session consistency, linearizability, vector clocks, Lamport clocks, HLC, TrueTime, Spanner, 2PC, 3PC, SAGA, TCC, transaction, distributed transaction, compensation, CRDT, gossip protocol, failure detection, phi accrual, SWIM, split brain, fencing, DynamoDB, Cassandra, ZooKeeper, etcd, coordinator, atomic commit, time skew, clock synchronization, NTP, global database, multi-region, cross-shard, hot spot, rebalance, virtual nodes, rate limiting distributed, token bucket, sliding window, deadlock distributed, two-phase locking
