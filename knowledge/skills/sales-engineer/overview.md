# Sales Engineer - Skill de Conocimiento

## Visión General
Este skill contiene conocimientos, mejores prácticas y enfoques específicos para el rol de Sales Engineer (SE). Diseñado para ayudar en entrevistas técnicas y situaciones de preventa donde se combina conocimiento profundo del producto con habilidades de ventas y comunicación técnica.

## Competencias Clave
- Traducción de capacidades técnicas a valor de negocio
- Diseño y presentación de soluciones técnicas personalizadas
- Manejo de objeciones técnicas complejas
- Ejecución de PoCs (Proof of Concept) y demos efectivas
- Colaboración con equipos de ventas y producto
- Conocimiento profundo del stack tecnológico y arquitectura del producto
- Habilidad para realizar discovery técnico y entender pain points del cliente

## Metodologías y Frameworks
- Metodología MEDDIC/MEDDPICC para calificación de oportunidades
- Enfoque Challenger para ventas técnicas
- Marco de solución: Problema → Impacto → Solución → Valor
- Metodología de demo: contexto → dolor → solución → beneficio → próximo paso
- Análisis de ROI y TCO (Total Cost of Ownership)
- Arquitectura de referencia y patrones de integración

## Escenarios de Entrevista Comunes
### Preguntas Comportamentales
1. "Cuéntame de una vez que tuviste que explicar una arquitectura compleja a un stakeholder no técnico"
2. "Describe una situación donde el cliente tenía requerimientos contradictorios y cómo los reconciliaste"
3. "Cómo has manejado una demo fallida o problemas técnicos durante una presentación importante"
4. "Ejemplo de cómo identificaste una necesidad no expresada del cliente durante el discovery"
5. "Situación donde tuviste que aprender rápidamente una nueva tecnología para cumplir con un plazo de cliente"

### Preguntas Situacionales (Técnicas)
1. "Un cliente quiere integrar nuestro producto con su sistema legacy que usa tecnología X. ¿Cómo abordarías esto?"
2. "Durante una evaluación, el cliente menciona que nuestro rendimiento es un 20% menor que el competidor en su benchmark. ¿Qué haces?"
3. "El arquitecto técnico del cliente es escéptico sobre nuestra escalabilidad. ¿Cómo construireis su confianza?"
4. "Un cliente de enterprise quiere saber cómo garantizamos cumplimiento con regulaciones Y y Z. ¿Cuál es tu enfoque?"

### Preguntas de Discovery Técnico
1. "¿Qué métricas actuales mides y cuáles son tus mayores dolores de cabeza?"
2. "Describe tu arquitectura actual - qué tecnologías usas y cómo se comunican entre sí"
3. "¿Cuáles son los tres principales cambios que te gustaría hacer en tu stack actual?"
4. "¿Cómo manejan actualmente [problema específico que nuestro producto resuelve]?"
5. "¿Qué tan importante es [factor X] para ustedes y cómo lo miden actualmente?"

## Respuestas Efectivas - Plantillas STAR
### Para demostrar habilidad de traducción técnica-negocio
**S**ituación: Cliente financiero evaluaba nuestra plataforma de procesamiento de transacciones pero no veía cómo justificar la inversión frente a su sistema actual.
**T**area: Demostrar el valor de negocio de migrar a nuestra solución más allá de las especificaciones técnicas.
**A**ción:
- Realicé sesiones de discovery técnico para entender su volumen pico, patrones de fallo y costos ocultos
- Mapeamos sus SLAs actuales vs. lo que podían lograr con nuestra plataforma (menor latencia = menor abandono)
- Preparé un modelo de TCO mostrando ahorros en mantenimiento, escalabilidad y reducción de incidentes
- Diseñé un PoC enfocado en su caso de uso más crítico (liquidación en tiempo real)
- En la presentación, enfoqué 70% en impacto de negocio y 30% en detalles técnicos
**R**esultado:
- Cliente aprobó el proyecto con ROI proyectado en 8 meses
- El PoC logró procesar 5x su volumen actual con latencia <50ms
- Se convirtió en referencia para otros clientes del sector financiero

### Para manejar objeciones técnicas complejas
**S**ituación: Arquitecto de cliente expresó preocupaciones profundas sobre nuestra consistencia eventual en distribuciones globales durante una reunión de evaluación.
**T**area: Abordar la objeción técnica sin perder credibilidad ni prometer más de lo que podemos entregar.
**A**ción:
- Reconocí válidamente su preocupación y expliqué dónde consistencia fuerte sí es necesaria en nuestro producto
- Mostré con diagramas cómo manejamos la consistencia en diferentes tipos de datos (config vs. transacciones vs. analytics)
- Compartimos nuestras guías de consistencia y niveles de garantía por tipo de operación
- Proponí un enfoque de mitigación: usar nuestras transacciones distribuidas para operaciones críticas y consistencia eventual para analytics
- Ofrecí hacer un taller específico con su equipo de arquitectura para revisar sus flujos de datos
**R**esultado:
- El arquitecto se convirtió en defensor interno después del taller
- Cliente firmó contrato con cláusula de revisión técnica a los 3 meses
- Documentamos el enfoque como patrón para objeciones de consistencia futura

## Recursos Recomendados
- Libros: "The Challenger Sale: Taking Control of the Customer Conversation", "DemoMIND: A Framework for Winning More Demos"
- Frameworks: MEDDIC, SPICED, HEED Metodology
- Habilidades técnicas clave: conocimiento de APIs, comprensión de patrones de integración (sync/async, pub/sub), bases de datos (SQL/NoSQL), conceptos de cloud y distribuidos