# DevOps and SRE - Domain Knowledge

## Overview
DevOps and Site Reliability Engineering (SRE) encompass the practices, tools, and cultural philosophy for delivering software rapidly and reliably. This domain covers CI/CD pipelines, infrastructure as code, container orchestration, and incident management.

## Key Concepts

### CI/CD Pipeline Design
- **Continuous Integration**: Automated build + test on every commit. Target: <10 minute feedback loop.
- **Continuous Delivery**: Every commit that passes CI is potentially deployable. Requires automated deployment pipelines and feature flags.
- **Continuous Deployment**: Every passing commit is automatically deployed to production. Highest maturity level.
- **Pipeline Stages**: Build, unit test, integration test, security scan, staging deploy, smoke test, production deploy, canary analysis.
- **Artifact Management**: Docker images in registries (ECR, GCR, Harbor), versioned and immutable. Never mutate a deployed artifact.

### Infrastructure as Code (IaC)
- **Terraform**: Declarative IaC with state management. Key patterns: remote state backend, workspace isolation, module composition.
- **State Management**: Remote backends (S3+DynamoDB, GCS), state locking to prevent concurrent modifications.
- **GitOps**: Declarative infrastructure stored in Git, automated reconciliation (ArgoCD, Flux). Git is the single source of truth.
- **Immutable Infrastructure**: Replace rather than modify. Blue-green deployments, canary releases. Servers are cattle, not pets.

### Container Orchestration
- **Kubernetes Core Objects**: Pods, Deployments, Services, ConfigMaps, Secrets, Ingress, PersistentVolumes.
- **Helm**: Package manager for K8s. Charts define templated manifests. Values files for environment-specific configuration.
- **Resource Management**: Requests (guaranteed minimum) vs Limits (maximum allowed). QoS classes: Guaranteed, Burstable, BestEffort.
- **Health Checks**: Liveness probes (restart if dead), readiness probes (remove from service if not ready), startup probes (give time to initialize).
- **Horizontal Pod Autoscaler (HPA)**: Scale based on CPU, memory, or custom metrics. Requires Metrics Server.
- **Network Policies**: Pod-level firewall rules. Default-deny is the secure baseline.

### Deployment Strategies
- **Rolling Update**: Default in K8s. Gradual replacement with configurable surge and unavailability.
- **Blue-Green**: Two identical environments, switch traffic instantly. Fast rollback but requires double capacity.
- **Canary**: Route small percentage of traffic to new version, monitor metrics, progressive rollout. Flagger and Argo Rollouts automate this.
- **Feature Flags**: Decouple deployment from release. LaunchDarkly, Unleash, or simple config-driven toggles.

### Incident Management
- **Incident Lifecycle**: Detection, Triage, Mitigation, Resolution, Postmortem.
- **Blameless Postmortem**: Focus on systemic causes, not individual mistakes. Action items must be specific, assigned, and tracked.
- **Runbooks**: Documented procedures for common incidents. Reduces MTTR and on-call anxiety.
- **Alert Design**: Alert on symptoms (user-facing impact), not causes. Every alert should be actionable.
- **On-Call Rotation**: Fair rotation, compensation, clear escalation paths. Target: <25% time on toil.

### Security in the Pipeline
- **Container Scanning**: Trivy, Snyk, Grype for vulnerability detection in images.
- **Secret Management**: Vault, AWS Secrets Manager, Sealed Secrets. Never commit secrets to Git.
- **RBAC**: Principle of least privilege. Service accounts per workload in K8s.
- **Supply Chain**: Sigstore for artifact signing, SBOM (Software Bill of Materials) generation.

## Common Interview Questions

1. "Design a CI/CD pipeline for a microservices application with 20 services."
2. "How do you handle database migrations in a zero-downtime deployment?"
3. "A production incident just started. Walk me through your response process."
4. "How do you manage configuration and secrets across multiple environments?"
5. "Explain the difference between Kubernetes requests and limits. What happens when you misconfigure them?"
6. "How would you implement canary deployments? What metrics would you monitor?"
7. "Design a monitoring and alerting strategy for a K8s cluster running 50 microservices."
8. "How do you handle infrastructure drift in a GitOps workflow?"
9. "Compare Terraform and Pulumi. When would you choose one over the other?"
10. "How do you reduce toil in an SRE team? Give a concrete example of an automation you'd build."

## STAR Story Triggers
- CI/CD, pipeline, deployment, Docker, Kubernetes, K8s, Helm, Terraform, infrastructure, GitOps, ArgoCD, canary, blue-green, incident, postmortem, on-call, alerting, runbook, security, RBAC, scanning, secret, container, orchestration, rollout, provisioning, automation, DevOps, SRE, MTTR, uptime, downtime, rollback
