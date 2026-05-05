# ML Fundamentals - Domain Knowledge

## Overview
Machine Learning se ha vuelto un tema obligatorio en entrevistas para senior roles, incluso para posiciones no-ML (porque ML está everywhere — recomendaciones, búsqueda, detección de anomalías, LLMs). Este dominio cubre los fundamentos que todo engineer debe conocer: cómo funciona el pipeline de ML, cómo evaluar modelos, cómo evitar overfitting, y cómo llevar modelos a producción con MLOps.

## Key Concepts

### ML Pipeline
- **Data Collection**: Fuentes: eventos de producto, DBs, third-party APIs, web scraping. Desafíos: data quality, bias en el muestreo, data drift en producción.
- **Feature Engineering**: Transformar raw data en features que el modelo entiende. Técnicas: one-hot encoding (categóricas), normalization/standardization (numéricas), binning, text vectorization (TF-IDF, embeddings), feature cross (interacciones).
- **Train/Validation/Test Split**: 70/15/15 típicamente. Hold-out validation para datasets grandes. Cross-validation (k-fold) para datasets chicos. Stratified split para datos desbalanceados.
- **Model Training**: Elegir algoritmo según el problema (regresión, clasificación, clustering). Entrenar en training set, validar en validation set, iterar.
- **Evaluation**: Métricas en test set (holdout final). NO evaluar en training set (overfitting). NO iterar basado en test set (data leakage).
- **Deployment**: Batch inference (datos en lotes, menor latency requirement) vs real-time (REST/gRPC endpoint, baja latencia).

### Model Training
- **Supervised Learning**: Labeled data. Regresión (predecir un valor continuo — precios, temperatura). Clasificación (predecir una categoría — spam detection, image classification).
- **Unsupervised Learning**: Sin labels. Clustering (K-means, DBSCAN, hierarchical), dimensionality reduction (PCA, t-SNE), anomaly detection.
- **Reinforcement Learning**: Agent aprende por trial-and-error con rewards. Usado en robotics, game AI, recommendation systems.
- **Loss Functions**: MSE (Mean Squared Error) para regresión. Cross-entropy para clasificación. MAE (Mean Absolute Error) menos sensible a outliers que MSE.
- **Gradient Descent**: Algoritmo que minimiza la loss function ajustando los pesos en la dirección del gradiente negativo. Variantes: SGD (stochastic — un sample por step), Mini-batch (batch de samples), Adam (adaptive learning rate, el más usado).
- **Práctica**: Para tabular data: XGBoost/LightGBM. Para imágenes: CNNs. Para texto: Transformers. Para datos no estructurados mixtos: modelos multimodales.

### Evaluation Metrics
- **Classification**: Accuracy (correctas/total — no sirve si desbalanceado), Precision (TP/(TP+FP) — qué tan selectivo), Recall (TP/(TP+FN) — qué tan completo), F1-Score (media armónica de precision + recall), AUC-ROC (tradeoff entre TPR y FPR).
- **Regression**: MSE (Mean Squared Error — penaliza más errores grandes), RMSE (square root de MSE — misma unidad que target), MAE (Mean Absolute Error), R² (proporción de varianza explicada).
- **Confusion Matrix**: TP (true positives), TN (true negatives), FP (false positives / Type I error), FN (false negatives / Type II error).
- **Precision-Recall Curve**: Mejor que ROC cuando hay desbalanceo de clases. AUC-PR es más informativo que AUC-ROC en casos extremos.
- **Business Metrics**: Lo que realmente importa — revenue impact, user retention, cost savings. El modelo es un medio, no un fin.

### Overfitting & Regularization
- **Overfitting**: El modelo memoriza el training data pero no generaliza. Síntomas: training loss baja, validation loss sube. Causas: modelo muy complejo, muy pocos datos, demasiadas features.
- **Underfitting**: El modelo no captura el patrón ni en training. Causas: modelo muy simple, features insuficientes, entrenamiento insuficiente.
- **L1 Regularization (Lasso)**: Agrega suma de valores absolutos de pesos a la loss. Tiende a cero pesos → feature selection. Útil cuando tenés muchas features irrelevantes.
- **L2 Regularization (Ridge)**: Agrega suma de cuadrados de pesos. Penaliza pesos grandes pero nunca los lleva a cero. Previene overfitting suavemente.
- **Elastic Net**: Combina L1 + L2. Lo mejor de ambos mundos.
- **Dropout**: En deep learning: apagar aleatoriamente un % de neuronas durante training. Fuerza al modelo a aprender representaciones redundantes y robustas.
- **Early Stopping**: Detener training cuando validation loss deja de mejorar. Previene overfitting automáticamente.
- **Cross-validation**: k-fold validation: evaluar en diferentes splits. Reduce la varianza de la estimación de performance.

### Feature Engineering
- **Encoding**: One-hot encoding (categorías sin orden), label encoding (categorías ordinales), target encoding (reemplazar categoría por media del target — cuidado con leakage).
- **Scaling**: Standardization (z-score: media 0, std 1) para datos con distribución normal. Normalization (min-max: [0,1]) para datos con bounds conocidos. Robust scaling (usando median + IQR) para datos con outliers.
- **Feature Selection**: Filter methods (correlación, chi-square), Wrapper methods (forward/backward selection), Embedded methods (L1 regularization, tree importance).
- **Dimensionality Reduction**: PCA (componentes principales — transformación lineal), t-SNE (visualización de alta dimensión), UMAP (más rápido que t-SNE, preserva mejor estructura global).
- **Date/Time Features**: Extraer día de semana, mes, hora, is_weekend, is_holiday, rolling windows, lag features.
- **Text Features**: Bag of Words, TF-IDF, word embeddings (Word2Vec, GloVe), contextual embeddings (BERT, sentence-transformers).
- **Práctica**: Feature engineering es donde más valor se agrega en ML aplicado. Un buen feature engineered con un modelo simple gana a features raw con un modelo complejo.

### Model Serving
- **Batch Inference**: Procesar lotes de datos periódicamente (cada hora/día). Simple, eficiente, barato. Usos: recomendaciones diarias, reportes, scoring de leads.
- **Real-time Inference**: REST API o gRPC endpoint. Latencia crítica (<100ms). Usos: detección de fraude, búsqueda, recomendaciones en tiempo real.
- **Model Format**: ONNX (formato portable, soportado por múltiples frameworks), TensorRT (optimizado para NVIDIA), TorchScript (PyTorch), SavedModel (TensorFlow), PMML (para modelos tabulares).
- **Model Versioning**: Cada versión del modelo tiene un ID único. A/B testing entre versiones. Canary deployments. Rollback inmediato.
- **Feature Store**: Repositorio centralizado de features (Feast, Tecton). Reusar features entre modelos, consistencia training/serving.
- **Práctica**: Usar ONNX como formato universal. Docker + Kubernetes para serving (Seldon Core, KServe). Feature store evita training-serving skew.

### MLOps
- **Experiment Tracking**: MLflow, Weights & Biases, Neptune. Log de parámetros, métricas, artifacts, código. Reproducibilidad de experimentos.
- **Model Registry**: MLflow Model Registry, DVC, HuggingFace Hub. Versiones, stages (staging/production), approval workflow, metadata.
- **CI/CD for ML**: DVC (data versioning), CML (Continuous ML — GitHub Actions para ML). Data validation al commit, training automático, evaluation gates, deploy si pasa thresholds.
- **Monitoring**: Data drift (distribución de features cambia), model drift (performance degrada), prediction drift (distribución de predicciones cambia). Herramientas: Evidently, WhyLabs, Sagemaker Model Monitor.
- **Retraining**: Programado (semanal/mensual), o trigger por drift detection (automático cuando performance cae debajo de threshold).
- **Reproducibility**: Fijar random seeds, versionar datos + código + hiperparámetros, containerizar el ambiente de entrenamiento.

### Deep Learning Basics
- **Neural Network**: Capas: input → hidden (con activaciones no lineales como ReLU, tanh, sigmoid) → output. Weights aprendidos por backpropagation.
- **CNN (Convolutional Neural Networks)**: Para datos con estructura espacial (imágenes, audio spectrograms). Capas: convolution (detecta patrones locales), pooling (reduce dimensionalidad), fully connected.
- **RNN (Recurrent Neural Networks)**: Para datos secuenciales (texto, time series). Problema: vanishing gradient. LSTM/GRU resuelven con gates.
- **Transformers**: Attention mechanism (cada token atiende a todos los tokens). La arquitectura dominante para NLP (BERT, GPT) y visión (ViT). Escala cuadráticamente con seq length.
- **Transfer Learning**: Usar un modelo pre-entrenado y fine-tunearlo para tu tarea. Mucho más eficiente que entrenar desde cero. Ej: BERT fine-tuned para clasificación de texto.
- **Práctica**: No entrenar CNNs/Transformers desde cero. Usar modelos pre-entrenados (HuggingFace, PyTorch Hub) y fine-tunear.

## Common Interview Questions

1. **"Explicá la diferencia entre overfitting y underfitting. ¿Cómo diagnosticás y solucionás cada uno?"**
   Overfitting: training loss baja, validation loss alta. Fix: más datos, regularización (L1/L2/dropout), early stopping, reducir complejidad del modelo. Underfitting: training loss alta también. Fix: modelo más complejo, mejores features, más epochs, reducir regularización.

2. **"¿Qué métricas usás para evaluar un modelo de clasificación con clases desbalanceadas?"**
   Accuracy no sirve (95% sí, 5% no → clasificar todo como sí da 95% accuracy). Usar: Precision, Recall, F1-Score, AUC-PR (no AUC-ROC). Matriz de confusión. Estratificar el split. Técnicas: SMOTE para oversampling, class weights, focal loss.

3. **"Explicá la diferencia entre bagging y boosting. ¿Cuándo usarías cada uno?"**
   Bagging (Random Forest): múltiples modelos entrenados en paralelo, cada uno en un bootstrap sample. Promedio/voto. Reduce varianza. Bueno para datos con mucho ruido. Boosting (XGBoost, LightGBM): modelos secuenciales, cada uno corrige errores del anterior. Reduce bias. Más propenso a overfitting. LightGBM es más rápido con datos grandes.

4. **"¿Cómo detectás y manejás data drift en producción?"**
   Monitorear distribución de features (PSI — Population Stability Index, KS test). Si drift detectado: (1) Alert. (2) Investigar causa raíz (cambio en comportamiento de usuarios, bug en pipeline de datos). (3) Retrain con datos nuevos. (4) Validar en test set. (5) Deploy nueva versión. Herramientas: Evidently, WhyLabs.

5. **"Diseñá un sistema de recomendación para un e-commerce."**
   Dos fases: (1) Retrieval — candidate generation (collaborative filtering, content-based, embeddings de usuario/producto). (2) Ranking — modelo más complejo (GBDT o red neuronal) que rankea candidates por CTR estimado. Cold start: usar content-based para nuevos items. A/B testing online.

6. **"Explicá cómo funciona el transformer attention mechanism."**
   Cada token genera Q (query), K (key), V (value) matrices. Attention scores = softmax(Q·K^T / sqrt(d_k)). Multi-head attention: múltiples attention heads capturan distintas relaciones. Self-attention: cada token atiende a todos los tokens. Positional encoding inyecta orden secuencial.

7. **"¿Qué son los embeddings y cómo se generan?"**
   Embeddings son representaciones vectoriales densas de datos discretos (palabras, items, usuarios). Capturan significado semántico: vectores similares → significados similares. Word2Vec (CBOW, Skip-gram), GloVe, BERT embeddings. Para items: usar modelos two-tower (user embedding + item embedding). Distancia coseno entre embeddings mide similitud.

8. **"Explicá cómo funciona gradient descent y la diferencia entre batch, stochastic y mini-batch."**
   Gradient descent: calcular gradiente de la loss respecto a los pesos, mover pesos en dirección opuesta. Batch GD: todo el dataset por step (preciso pero lento y memory-heavy). SGD: un sample por step (ruidoso pero rápido). Mini-batch: batch de 32-512 samples — mejor tradeoff. Adam: learning rate adaptativo por parámetro (el más usado en deep learning).

9. **"¿Cómo construirías un ML pipeline reproducible?"**
   Versionar: datos (DVC), código (git), ambiente (Docker + requirements.lock/pyproject.toml). MLflow para experiment tracking + model registry. CI/CD con CML (GitHub Actions): data validation → training → evaluation gates → deploy. Fijar seeds. Data pipeline inmutable (no modificar raw data). Feature store para training-serving consistency.

10. **"Diseñá un sistema de detección de anomalías para transacciones financieras en tiempo real."**
    Pipeline: streaming de transactions → feature engineering en tiempo real (amount, frequency, location, device fingerprint, historical patterns) → modelo de ML (Isolation Forest o LSTM-Autoencoder). Threshold ajustable (tradeoff entre false positives y detección). Si anomalía score > threshold → bloquear transacción + alert compliance team. Online learning para adaptarse a nuevos patrones. Monitoreo de drift + retraining periódico.

## STAR Story Triggers
- ML pipeline, feature engineering, data preprocessing, train/validation/test split, cross-validation, supervised learning, unsupervised learning, reinforcement learning, loss function, gradient descent, SGD, Adam, optimizer, overfitting, underfitting, regularization, L1, L2, Lasso, Ridge, dropout, early stopping, classification, regression, clustering, precision, recall, F1, AUC-ROC, AUC-PR, confusion matrix, RMSE, MAE, R², feature selection, PCA, t-SNE, UMAP, one-hot encoding, normalization, standardization, embeddings, Word2Vec, BERT, Transformer, attention, CNN, RNN, LSTM, batch inference, real-time inference, ONNX, model serving, KServe, Seldon, MLOps, MLflow, Weights & Biases, experiment tracking, model registry, data drift, model drift, concept drift, Evidently, retraining, A/B testing, feature store, Feast, Tecton, reproducibility, DVC, CML
