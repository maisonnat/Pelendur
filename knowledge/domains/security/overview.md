# Security - Domain Knowledge

## Overview
Seguridad en sistemas modernos abarca desde proteger el código hasta defender la infraestructura. Este dominio cubre los fundamentos que todo senior engineer debe conocer: cómo autenticar y autorizar usuarios, cómo proteger datos en reposo y en tránsito, cómo defenderse contra ataques comunes, y cómo construir sistemas seguros desde el diseño (security by design).

## Key Concepts

### OWASP Top 10
Los 10 riesgos de seguridad más críticos en web applications según OWASP (2021):
1. **Broken Access Control** — #1. Usuarios acceden a recursos que no deberían (IDOR, path traversal, privilege escalation). Fix: server-side authorization checks siempre.
2. **Cryptographic Failures** — Datos sensibles sin encriptar, TLS mal configurado, hashes débiles. Fix: encriptar todo (encryption at rest + in transit), usar algoritmos modernos (AES-256, SHA-256, bcrypt/Argon2).
3. **Injection** — SQLi, NoSQLi, OS command injection, LDAP injection. Fix: prepared statements/parameterized queries, input validation, ORMs seguros.
4. **Insecure Design** — Arquitectura sin threats considerados. Fix: threat modeling al diseñar (STRIDE), security reviews en design phase.
5. **Security Misconfiguration** — Default creds, puertos abiertos, debug endpoints en prod. Fix: hardening automático, scanning periódico, mínimo privilegio.
6. **Vulnerable & Outdated Components** — Dependencies con CVEs conocidas. Fix: SBOM, dependency scanning (Dependabot, Snyk), updates automáticos.
7. **Identification & Auth Failures** — Session management débil, credenciales default, MFA faltante. Fix: OAuth 2.0/OIDC, MFA, passwordless (WebAuthn).
8. **Software & Data Integrity Failures** — CI/CD sin firmar, update channels inseguros. Fix: firmar artifacts, verify checksums, supply-chain security.
9. **Security Logging & Monitoring Failures** — No detectás ataques porque no logueás. Fix: logging centralizado, SIEM, alertas en tiempo real.
10. **SSRF (Server-Side Request Forgery)** — El servidor hace requests a URLs controladas por el atacante. Fix: allowlists de URLs, validación de redirects, network segmentation.

### Authentication & Authorization
- **OAuth 2.0**: Framework de delegación de acceso. Roles: resource owner (user), client (app), authorization server, resource server. Flows: Authorization Code (más seguro, con PKCE), Implicit (deprecated), Client Credentials (machine-to-machine).
- **OpenID Connect (OIDC)**: Capa de identidad sobre OAuth 2.0. Agrega ID Token (JWT) con claims del usuario. Ej: "Login with Google" usa OIDC.
- **JWT (JSON Web Tokens)**: Token auto-contenido con claims firmado (JWS) o encriptado (JWE). Stateless — el server no necesita sesión. Cuidado: JWT no expirado puede usarse hasta su expiración (revocación requiere blacklist).
- **SAML**: XML-based, usado en enterprise SSO. Más verboso que OIDC. El Identity Provider (IdP) envía assertions al Service Provider (SP).
- **RBAC (Role-Based Access Control)**: Permisos asignados a roles, roles asignados a usuarios. Simple pero puede volverse rígido.
- **ABAC (Attribute-Based Access Control)**: Permisos basados en atributos (user, resource, environment). Más granular, más complejo. Usado en AWS IAM (policies).
- **Práctica**: OAuth 2.0 + OIDC para APIs externas. JWT para microservicios internos (con short TTL + refresh). SAML para integraciones enterprise legacy.

### Zero Trust Architecture
- **Principio**: "Never trust, always verify." No hay perímetro de red seguro. Cada request debe ser autenticado y autorizado explícitamente.
- **Micro-segmentation**: Dividir la red en segmentos aislados. Cada servicio solo puede comunicarse con servicios específicos (service mesh, network policies en Kubernetes).
- **Least Privilege**: Cada identidad (humana o servicio) tiene exactamente los permisos que necesita — ni más.
- **Continuous Verification**: No basta con auth al login. Cada request se evalúa en tiempo real: device posture, location, behavior anomalies.
- **BeyondCorp (Google)**: Implementación pionera. Acceso basado en identidad+device, no en IP. Zero trust VPN (Cloudflare Access, Tailscale, Zscaler).
- **Práctica**: Service mesh (Istio/Linkerd) con mTLS + RBAC. API gateways con authz policies (OAuth2 Proxy, Pomerium).

### Encryption at Rest & In Transit
- **Symmetric (AES)**: Misma clave para encrypt/decrypt. Rápido. AES-256-GCM (authenticated encryption) es el estándar moderno. Usos: disk encryption, database encryption, file encryption.
- **Asymmetric (RSA, ECDSA)**: Clave pública/privada. Más lento. Usos: key exchange (TLS handshake), digital signatures (code signing, JWT signing).
- **TLS 1.3**: Hanshake más rápido (1-RTT vs 2-RTT en TLS 1.2). Remueve cifrados inseguros (RSA key exchange, CBC). Perfect Forward Secrecy por defecto (ECDHE).
- **mTLS (Mutual TLS)**: Cliente y server se autentican mutuamente. Esencial en service mesh. Cada service tiene un certificate emitido por la CA del mesh.
- **Encryption at Rest**: AES-256 para datos en disco. AWS KMS / GCP Cloud KMS / HashiCorp Vault para key management. Transparent Data Encryption (TDE) en DBs.
- **Hashing**: bcrypt/Argon2 para passwords (cost factor ajustable para ser lento), SHA-256 para integrity checks (no para passwords).

### Network Security
- **WAF (Web Application Firewall)**: Filtra traffic HTTP/HTTPS. Protege contra OWASP Top 10, bots, DDoS. Cloudflare WAF, AWS WAF, ModSecurity (open source con CRS).
- **IDS/IPS (Snort, Suricata)**: Monitorea tráfico de red en busca de patrones maliciosos. IPS puede bloquear activamente.
- **DDoS Protection**: Cloudflare (anycast + rate limiting), AWS Shield, Google Cloud Armor. Estrategias: rate limiting en edge, scrubbing centers, auto-scaling.
- **Network Segmentation**: VLANs, subnets aisladas, security groups (AWS) / firewall rules. Bases de datos nunca en la misma subnet que web servers públicos.
- **API Security**: Rate limiting, API keys, OAuth scopes, request validation (OpenAPI spec), schema enforcement (JSON Schema).

### Secret Management
- **El problema**: API keys, DB passwords, TLS certs en código, .env files, config maps, logs. Cada leak de secret es un breach potencial.
- **Vault (HashiCorp)**: Dynamic secrets (genera credenciales on-demand con TTL corto), leasing & rotation, audit logging, encryption as a service.
- **Cloud-native**: AWS Secrets Manager, GCP Secret Manager, Azure Key Vault. Integrados con IAM para access control.
- **Kubernetes**: External Secrets Operator que sync secrets de Vault/AWS a k8s secrets. Sealed Secrets para gitops. CSI driver para montar secrets como volúmenes.
- **Rotation**: Automática con TTL cortos. En Vault: database credentials rotan cada 24h, AWS IAM keys cada 30 días.
- **Práctica**: Nunca secrets en git. Usar `sops` (Mozilla) para encriptar secretos en repositorios. `git-secrets` para prevenir commits con secrets.

### Supply Chain Security
- **SBOM (Software Bill of Materials)**: Inventario de todas las dependencies de un proyecto. Formato SPDX o CycloneDX. Generado automáticamente (Syft).
- **Dependency Scanning**: Dependabot (GitHub), Snyk, Trivy, Renovate. Detecta CVEs en dependencies y abre PRs de fix automáticos.
- **Sigstore**: Firmado de artifacts. Cosign para container images, Fulcio para certificados efímeros. Rekor para transparency log.
- **SLSA (Supply-chain Levels for Software Artifacts)**: Framework de madurez de supply chain security. Nivel 1: build script. Nivel 2+3: build service con provenance attestation.
- **Práctica**: Dependabot automático. SCAN all PRs para CVEs. Firmar releases con cosign. Usar distroless images (minimizar superficie de ataque).

## Common Interview Questions

1. **"Explicá la diferencia entre autenticación y autorización. ¿Cómo implementarías ambas en microservicios?"**
   AuthN = "who you are" (login, JWT, OIDC). AuthZ = "what you can do" (policies, RBAC). En microservicios: API Gateway maneja authN (valida JWT), cada servicio implementa authZ con OPA (Open Policy Agent) o Casbin. Políticas centralizadas en un sidecar (Envoy ext_authz).

2. **"¿Cómo protegerías una API REST contra ataques comunes?"**
   Rate limiting por API key + IP (Redis sliding window). Input validation con JSON Schema. Auth con OAuth 2.0 + scopes. CORS restrictivo. TLS 1.3. WAF Cloudflare. Audit logging de cada request. HSTS + CSP headers. Never trust client-side validation.

3. **"Explicá el flujo OAuth 2.0 Authorization Code con PKCE."**
   Client genera code_verifier + code_challenge. Redirect al auth server → user login → auth server redirige con authorization code. Client envía code + code_verifier al token endpoint. Server verifica que el code_challenge matchea. Devuelve access_token + refresh_token + id_token (si es OIDC).

4. **"¿Cómo manejás secrets en un equipo de 20 developers sin que se filtren?"**
   HashiCorp Vault para dynamic secrets (DB creds con TTLs). External Secrets Operator en k8s. sops para encriptar secrets en git (descifrados solo en CI/CD con KMS). Pre-commit hooks (git-secrets, detect-secrets). Rotación automática cada 24h. Audit logging de acceso a secrets.

5. **"¿Qué es Zero Trust y cómo lo implementarías en una empresa que migra a cloud?"**
   Trust no implícito por estar en la red corporativa. Cada request es verificado. Pasos: (1) Identity-first con SSO + MFA. (2) Device posture check. (3) Micro-segmentation de red. (4) Service mesh con mTLS. (5) Continuous authz con OPA. (6) Logging centralizado + SIEM para detectar anomalies.

6. **"Explicá la diferencia entre cifrado simétrico y asimétrico. ¿Dónde se usa cada uno?"**
   Simétrico (AES): misma clave para encrypt/decrypt, rápido. Usos: encriptar datos en disco, database encryption, cifrado de archivos. Asimétrico (RSA/ECDSA): par de claves, lento. Usos: TLS handshake (key exchange), firmar JWTs, code signing, SSH keys. Híbrido: TLS usa asimétrico para handshake, luego simétrico para datos.

7. **"¿Cómo manejarías un breach de seguridad donde un atacante accedió a la DB de producción?"**
   1) Contain: aislar servidor comprometido, rotar todas las credenciales. 2) Investigate: forensic analysis de logs, determinar entry point y data accessed. 3) Communicate: notificar a stakeholders, Data Protection Authority (si hay datos personales), usuarios afectados. 4) Remediate: patch vulnerabilidad, mejorar monitoreo, implementar encryption at rest. 5) Post-mortem: root cause analysis, mejorar security controls, tabletop exercises.

8. **"Explicá SSRF (Server-Side Request Forgery). ¿Cómo lo prevenís?"**
   SSRF: el atacante hace que el server haga requests a recursos internos (metadata endpoints cloud, internal services). Prevención: allowlist de URLs permitidas, validación de redirects, network segmentation (metadata endpoints bloqueados a nivel de firewall), uso de IMDSv2 (AWS) que requiere session token, validate response content-type.

9. **"¿Cómo implementarías supply chain security en un proyecto open source?"**
   SBOM con Syft al build. Dependency scanning con Dependabot + Snyk. Firmar releases con cosign + GitHub Attestations. SLSA level 3+: build from hermetic CI, provenance attestation. Verificar upstream deps (no instalar paquetes desconocidos). Sigstore para transparency log.

10. **"Diseñá un sistema de logging de seguridad que detecte ataques en tiempo real."**
    Logs estructurados de todos los servicios (jaeger para tracing, structured logging con correlation IDs). Pipeline: Filebeat/Vector → Kafka → Logstash/Fluentd → Elasticsearch. SIEM (Wazuh/Splunk) con reglas de correlación: múltiples 401 en corto tiempo, accesos a endpoints sensibles, patterns de SQLi/XSS en requests. Alertas en PagerDuty/Slack. Dashboard de security metrics.

## STAR Story Triggers
- OWASP, injection, SQLi, XSS, CSRF, SSRF, IDOR, broken access control, path traversal, privilege escalation, OAuth, OIDC, SAML, JWT, RBAC, ABAC, Zero Trust, BeyondCorp, least privilege, microsegmentation, mTLS, TLS, encryption, AES, RSA, ECDSA, hashing, bcrypt, Argon2, WAF, IDS, IPS, DDoS, rate limiting, API security, secret management, Vault, KMS, SIEM, SBOM, supply chain, Cosign, SLSA, Dependabot, Snyk, MFA, WebAuthn, passwordless, PKCE, OPA, policy as code, threat modeling, STRIDE, incident response, breach, forensics, compliance, SOC2, GDPR, HIPAA, PCI-DSS
