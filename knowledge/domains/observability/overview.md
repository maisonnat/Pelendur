# Observability - Domain Knowledge

## Overview
Observability is the ability to understand the internal state of a system by examining its external outputs. In modern distributed systems, observability encompasses metrics, traces, and logs (the three pillars) plus emerging practices like continuous profiling and real-user monitoring.

## Key Concepts

### The Three Pillars
- **Metrics**: Numeric measurements aggregated over time (counters, gauges, histograms). Prometheus is the de-facto standard for collection and alerting.
- **Traces**: End-to-end request flow across service boundaries. Distributed tracing (Jaeger, Zipkin, OpenTelemetry) tracks latency budgets and dependency maps.
- **Logs**: Discrete event records with structured context. The ELK stack (Elasticsearch, Logstash, Kibana) and Loki are common backends.

### SRE Fundamentals
- **SLI** (Service Level Indicator): Quantitative measure of service behavior (e.g., latency p99, error rate, throughput).
- **SLO** (Service Level Objective): Target value for an SLI (e.g., p99 latency < 200ms).
- **SLA** (Service Level Agreement): Contractual commitment based on SLOs with consequences for breach.
- **Error Budget**: 1 - availability target. The acceptable amount of unreliability per period.
- **Toil**: Operational work that is manual, repetitive, automatable, and devoid of enduring value.

### Observability Patterns
- **RED Method**: Rate (requests/sec), Errors (failed requests), Duration (latency histograms) — for request-driven services.
- **USE Method**: Utilization, Saturation, Errors — for resource-based analysis (CPU, memory, network, disk).
- **Golden Signals**: Latency, Traffic, Errors, Saturation (Google SRE).
- **Cardinality explosion**: High-dimensional metric labels can overwhelm TSDB backends. Control label cardinality aggressively.

### Modern Tooling Landscape
- **OpenTelemetry**: Vendor-neutral instrumentation standard for traces, metrics, and logs. Merged OpenTracing and OpenCensus.
- **eBPF**: Kernel-level observability without application changes (Cilium, Pixie, Tetragon).
- **Continuous Profiling**: Always-on CPU/memory profiling (Pyroscope, Parca, Datadog Continuous Profiler).
- **Synthetic Monitoring**: Proactive endpoint probing to catch issues before users do.

## Common Interview Questions

1. "How would you design an observability strategy for a microservices platform handling 100K requests per second?"
2. "Explain the difference between monitoring and observability. When is monitoring sufficient?"
3. "Your p99 latency suddenly tripled. Walk me through your debugging process."
4. "How do you handle cardinality explosion in metrics? Give a concrete example."
5. "Design an alerting strategy that minimizes alert fatigue while catching real incidents."
6. "What is distributed tracing and how does it work? How do you propagate trace context across service boundaries?"
7. "Explain error budgets. How would you use them to balance reliability with feature velocity?"
8. "How would you instrument a legacy service that has no tracing or structured logging?"
9. "Compare push-based vs pull-based metrics collection. What are the tradeoffs?"
10. "How do you measure and improve on-call quality of life?"

## STAR Story Triggers
- monitoring, observability, metrics, tracing, logging, alerting, SRE, SLA, SLO, error budget, incident, on-call, pager, Prometheus, Grafana, OpenTelemetry, Jaeger, Datadog, latency, throughput, p99, uptime, reliability, dashboard, SLI, toil
