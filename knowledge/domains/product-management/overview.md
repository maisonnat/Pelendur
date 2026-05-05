# Product Management - Domain Knowledge

## Overview
Product Management es un tema recurrente en entrevistas para roles senior y de liderazgo técnico (Staff+ Engineer, Engineering Manager, CTO). Aunque no seas PM, te van a preguntar cómo priorizás features, cómo trabajás con stakeholders, cómo medís el impacto de tu trabajo. Este dominio cubre los frameworks, métricas y procesos que todo leader técnico debe manejar.

## Key Concepts

### OKRs (Objectives & Key Results)
- **Estructura**: Objective (qué, aspiracional, inspirador) + 3-5 Key Results (cómo se mide, cuantitativo, alcanzable). Ej: "Ser la plataforma de pagos más confiable de LATAM" → "99.99% uptime", "NPS > 70", "< 1% transaction failure rate".
- **Good OKRs**: Objective motivador que da dirección. KRs específicos, medibles, con dueño claro. No son tasks — son outcomes.
- **Cascading**: Company OKRs → Team OKRs → Individual OKRs. Debe haber alineación, no copiar-pegar. Cada equipo deriva sus KRs de los company OKRs.
- **OKR vs KPI**: OKRs son aspiracionales por período (quarter). KPIs son métricas de salud continuas (uptime, retention, revenue). Un OKR puede influir en múltiples KPIs.
- **Práctica**: 3 OKRs por quarter máximo. KRs deben ser stretch goals (difícil pero posible). Evaluación no ligada a comp (Google), o ligada parcialmente (Microsoft). Revisión mid-quarter.

### Roadmap Prioritization
- **RICE Framework**: Reach (cuántos usuarios afecta), Impact (qué tan significativo), Confidence (qué tan seguros estamos), Effort (costo en engineer-weeks). Score = (Reach × Impact × Confidence) / Effort. Priorizar por score más alto.
- **MoSCoW**: Must-have (sin esto no se lanza), Should-have (importante pero no blocker), Could-have (nice to have), Won't-have (explicitamente fuera de scope). Útil cuando hay deadline fijo.
- **Eisenhower Matrix**: Urgent vs Important. Cuadrantes: Do First (urgente + importante), Schedule (importante no urgente), Delegate (urgente no importante), Eliminate (ni urgente ni importante).
- **Opportunity Scoring**: Encuestar usuarios: "qué tan importante es X" vs "qué tan satisfecho estás con X" → opportunity = importance - satisfaction. Priorizar features con mayor gap.
- **Kano Model**: Clasificar features en: Basic Needs (esperadas, no generan satisfacción extra), Performance (más es mejor), Delighters (sorpresas gratas, generan satisfacción). No invertir en Basic Needs más allá de lo necesario.
- **Práctica**: RICE para features nuevas. MoSCoW para releases con deadline. Kano para estrategia a largo plazo. Eisenhower para el día a día.

### Stakeholder Management
- **Identifying Stakeholders**: Quién tiene poder de decisión, quién será afectado, quién puede bloquear. Mapear en una matriz Poder-Interés.
- **Manage Up**: Entender las prioridades de tu manager/director. Comunicar en su lenguaje (business impact, no technical details). Anticipar preguntas antes de que las hagan.
- **Cross-team Alignment**: Dependencies entre equipos. RACI matrix (Responsible, Accountable, Consulted, Informed). Weekly syncs. Documentar decisiones y tradeoffs.
- **Expectation Setting**: Ser explícito sobre lo que NO se va a hacer (y por qué). Under-promise, over-deliver. Decir "no" es una skill — explicar tradeoffs, no solo rechazar.
- **Conflict Resolution**: Intereses vs posiciones (Harvard method). Separar la persona del problema. Buscar opciones de beneficio mutuo. Insistir en criterios objetivos.
- **Práctica**: Newsletter semanal de estado del proyecto. 1:1 regulares con stakeholders clave. Documentación de decisiones (ADRs — Architecture Decision Records).

### Product Discovery
- **User Research**: Entrevistas cualitativas (5 usuarios encuentran ~85% de problemas). Jobs to Be Done (JTBD): qué trabajo contrata el usuario a tu producto. Problem validation: asegurarse de que el problema existe antes de construir la solución.
- **Prototyping**: Low-fidelity (wireframes, paper prototypes) para validar flujo. High-fidelity (Figma, clickable prototypes) para usability testing. MVP: la versión más simple que resuelve el problema central.
- **Design Sprints**: Google Ventures: 5 días para resolver un problema crítico. Understand → Diverge → Decide → Prototype → Validate. Comprime meses de decisión en una semana.
- **User Testing**: Moderado (guión + facilitator) vs no moderado (grabación + análisis). A/B testing para validación cuantitativa. Alpha/beta testing con early adopters.
- **Práctica**: Discovery continuo, no solo al inicio. Cada quarter: research → prototype → test → build. Fracasar rápido y barato en discovery, no en delivery.

### Metrics & Analytics
- **North Star Metric**: La métrica única que captura el valor del producto a largo plazo. Ej: Airbnb = "nights booked", Spotify = "time spent listening", Facebook = "daily active users". Guía todas las decisiones.
- **AARRR (Pirate Metrics)**: Acquisition (cómo te encuentran), Activation (primera experiencia wow), Retention (vuelven?), Revenue (monetización), Referral (invitan a otros?).
- **Retention**: La métrica más importante para SaaS. Cohort analysis: % de usuarios que vuelven después de N días/semanas. Retention curve que se aplane = product-market fit. Good: >40% retention a 6 meses.
- **NPS (Net Promoter Score)**: "¿Qué tan probable es que recomiendes este producto?" 0-10. Promoters (9-10), Passives (7-8), Detractors (0-6). NPS = %Promoters - %Detractors. Rango: -100 a +100.
- **Cohort Analysis**: Agrupar usuarios por período de signup y trackear su comportamiento a lo largo del tiempo. Muestra si las mejoras del producto realmente mejoran retention.
- **Activation**: El momento "aha" donde el usuario experimenta el valor del producto. Ej: Slack = "team envía 10 mensajes", Dropbox = "primer file synced". Optimizar el time-to-activation.
- **Práctica**: Dashboard con North Star + 3-5 input metrics. Cohort reports semanales. Event tracking con Amplitude/Mixpanel/PostHog.

### A/B Testing
- **Diseño**: Control (A) vs Treatment (B). Asignación aleatoria de usuarios. Mínimo: 2 semanas para capturar day-of-week effects.
- **Statistical Significance**: p-value < 0.05 (95% confianza). Pero cuidado con el peeking problem — no mirar resultados cada día y decidir temprano (false positives).
- **Sample Size**: Calculadora de sample size antes de empezar. Depende de: baseline conversion rate, minimum detectable effect, desired power (típicamente 80%).
- **Métrica Primaria vs Secundarias**: Una métrica primaria (la decisión depende de ella). Múltiples secundarias para monitorear efectos colaterales. Segmentar por cohorts para entender impacto diferencial.
- **Práctica**: Google's "20% experiment failure rate" es normal (fracasar indica que estás testeando cosas importantes). No lanzar sin significancia estadística. Documentar todos los experiments, incluso los fracasos.

### Agile & Scrum
- **Sprint Planning**: Team capacity (velocity histórica). El equipo se compromete a un sprint goal. Desglosar stories en tasks de <2 días.
- **Daily Standup**: 3 preguntas: qué hice ayer, qué haré hoy, qué me bloquea. No más de 15 min. No es status meeting — es coordinación.
- **Retrospective**: Lo que funcionó, lo que no, acciones concretas para mejorar. Cambiar una cosa por sprint. Técnicas: Start/Stop/Continue, Sailboat, 4Ls (Liked, Learned, Lacked, Longed For).
- **Velocity**: Puntos de historia por sprint. Usar para capacity planning, no para medir productividad. La velocidad es única por equipo — no comparar entre equipos.
- **Estimation**: Planning poker. Fibonacci (1,2,3,5,8,13). Relative estimation: comparar con stories anteriores, no con horas. <8 puntos = demasiado grande, dividir.
- **Scrum Master vs Product Owner**: SM facilita el proceso, remueve blockers, protege al equipo. PO define el backlog, prioriza, es la voz del usuario.
- **Práctica**: Scrum no es religión. Adaptar al contexto: kanban para equipos de soporte/ops, scrum para equipos de producto con roadmap claro.

## Common Interview Questions

1. **"How do you decide what to build when everything is a priority?"**
   Data-driven: RICE framework (Reach, Impact, Confidence, Effort). Score = (R×I×C)/E. Siempre hay una cola de prioridades — las prioridades se negocian con stakeholders explicitando tradeoffs. Si dos features tienen score similar, priorizar la que desbloquea más valor futuro (optionality).

2. **"How do you handle a stakeholder who wants to change scope mid-sprint?"**
   No decir "no" automáticamente — entender el porqué. Evaluar impacto: ¿qué feature existente se sacrifica? El PO decide, no el stakeholder directo. Si es crítico: swap (una story sale, la nueva entra). Documentar el tradeoff. Después de 3 mid-sprint changes: revisar el proceso de discovery.

3. **"How do you measure the success of a product feature?"**
   Antes de construir: definir métrica primaria + hipótesis clara ("si hacemos X, entonces Y métrica mejora en Z%"). Después del launch: A/B test con significancia estadística. Monitorear métricas secundarias (efectos colaterales). North star: ¿esto mueve la métrica que realmente importa?

4. **"Walk me through how you'd launch a new feature from idea to ship."**
   1) Discovery: user research + problem validation. 2) Prototype: wireframes → usability testing. 3) Spec: PRD con requirements, success metrics, edge cases. 4) Build: sprints con refinamiento continuo. 5) QA: test plan, UAT con early adopters. 6) Launch: phased rollout (5% → 25% → 100%) con feature flags. 7) Measure: A/B test, cohort analysis. 8) Iterate: aprender, mejorar, expandir.

5. **"What's the difference between a good OKR and a bad one? Give examples."**
   Bad: "Improve performance" (no es medible, no es accionable). Good: "Make the app feel instant" → KR1: "P95 API latency < 200ms", KR2: "Page load < 1.5s en mobile", KR3: "User satisfaction score > 4.5/5". Good OKRs son outcomes, no outputs.

6. **"How do you align multiple engineering teams toward a common goal?"**
   Company/team OKRs en cascada. Cada equipo deriva sus KRs de los company OKRs — no copian, traducen. Quarterly all-hands con demos cross-team. Shared roadmap visible para todos. Cross-team DRI (Directly Responsible Individual) para initiatives grandes. Ritual de sync semanal entre leads.

7. **"Design a system to reduce churn for a B2B SaaS product."**
   1) Diagnostic: cohort analysis para identificar cuándo churnean (day 7, day 30). 2) Survey: churn survey + exit interviews. 3) Root causes: onboarding failure? falta de value realization? pricing? 4) Interventions: improved onboarding (time-to-value reducido), health score (detectar cuentas en riesgo), customer success outreach proactivo, feature adoption emails. 5) Measure: churn rate, expansion revenue, NPS.

8. **"How do you make data-driven decisions when you don't have enough data?"**
   Triangulación: qualitative signals (user interviews, support tickets) + quantitative (lo que hay, aunque sea poco) + competitive analysis + principled reasoning (first principles). Frame the decision as: "what's the cost of being wrong?" Si es reversible → decide rápido. Si es irreversible → más investigación. Usar pre-mortem: "asumamos que esta feature fracasa — ¿por qué?"

9. **"How do you prioritize technical debt vs new features?"**
   No es binario. Technical debt es un costo de oportunidad: la deuda te frena eventualmente. Frameworks: (1) Cuantificar el costo actual de la deuda (developer hours perdidos, incident rate, time-to-market). (2) Asignar un bucket fijo (20-30% de capacity para tech debt). (3) User-visible: si la deuda causa bugs visibles, priorizar. (4) Threshold: si la deuda supera X incidentes por mes, se paga automáticamente.

10. **"Tell me about a time you had to kill a project you invested heavily in."**
    (STAR method) Situation: proyecto de 6 meses con 5 engineers. Task: feature que no ganó tracción esperada. Action: analizar datos — usage flat después del launch, NPS negativo, churn de usuarios power. Decidir kill con datos, no con ego. Communicar transparentemente, celebrar el aprendizaje, reasignar equipo. Result: equipo enfocado en features de mayor impacto, revenue +20% el próximo quarter.

## STAR Story Triggers
- OKR, key result, objective, north star metric, KPI, RICE, MoSCoW, Eisenhower matrix, opportunity scoring, Kano model, prioritization, roadmap, product discovery, user research, JTBD, jobs to be done, prototyping, MVP, design sprint, A/B testing, statistical significance, p-value, sample size, peeking problem, cohort analysis, retention, activation, NPS, AARRR, pirate metrics, acquisition, referral, product-market fit, stakeholder management, manage up, RACI, tradeoff, decision framework, PRD, feature spec, sprint planning, scrum, kanban, velocity, estimation, planning poker, retrospective, daily standup, agile, scrum master, product owner, technical debt, build vs buy, phased rollout, feature flag, canary deploy, churn, expansion revenue, LTV, CAC, unit economics, product strategy, vision, mission, roadmap prioritization, data-driven decision, first principles, pre-mortem, post-mortem, incident review, blameless culture
