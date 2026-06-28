//! Servidor web para el Gestor Financiero.
//!
//! Sirve el frontend HTML y expone las APIs REST para manejar
//! tirillas, devengados, disponible y catálogos.

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

mod db;
use db::{
    ClasifEgreso, Concepto, DeudaPendiente, Devengado, DiferenciaResult, FormaPago, Tirilla,
};

/// Estado compartido: pool de conexiones + sesiones activas
struct AppState {
    db: PgPool,
    sesiones: Arc<Mutex<HashMap<String, String>>>,
    rate_limiter: RateLimitStore,
    admin_username: String,
    admin_password_hash: String,
}

// =============================================================================
// Autenticación
// =============================================================================

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
}

#[derive(Serialize)]
struct MsgError {
    error: String,
}

/// Extrae el token Bearer del header Authorization
fn extraer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .map(|s| s.to_string())
}

/// Valida el token. Retorna Ok(token) o Err(401)
async fn verificar_auth(
    headers: &HeaderMap,
    sesiones: &Arc<Mutex<HashMap<String, String>>>,
) -> Result<String, (StatusCode, Json<MsgError>)> {
    let token = extraer_token(headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(MsgError { error: "Token requerido".into() }),
        )
    })?;
    let map = sesiones.lock().await;
    if map.contains_key(&token) {
        Ok(token)
    } else {
        Err((
            StatusCode::UNAUTHORIZED,
            Json(MsgError { error: "Token inválido".into() }),
        ))
    }
}

// =============================================================================
// Rate Limiter para login
// =============================================================================

/// Entrada del rate limiter: (intentos, ventana_inicia_en)
type RateLimitStore = Arc<Mutex<HashMap<String, (u32, Instant)>>>;

/// Límites: 5 intentos por minuto por IP
const MAX_LOGIN_ATTEMPTS: u32 = 5;
const RATE_LIMIT_WINDOW_SECS: u64 = 60;

/// Verifica si la IP puede intentar login. Retorna Ok o 429.
async fn check_rate_limit(
    store: &RateLimitStore,
    ip: &str,
) -> Result<(), (StatusCode, Json<MsgError>)> {
    let mut map = store.lock().await;
    let now = Instant::now();
    
    if let Some((count, window_start)) = map.get(ip) {
        let elapsed = now.duration_since(*window_start).as_secs();
        if elapsed < RATE_LIMIT_WINDOW_SECS {
            // Dentro de la ventana de tiempo
            if *count >= MAX_LOGIN_ATTEMPTS {
                return Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(MsgError { error: format!("Demasiados intentos. Espera {} segundos.", RATE_LIMIT_WINDOW_SECS - elapsed) }),
                ));
            }
        } else {
            // Ventana expirada, reiniciar
            map.insert(ip.to_string(), (1, now));
            return Ok(());
        }
    }
    
    // Incrementar contador
    let entry = map.entry(ip.to_string()).or_insert((0, now));
    entry.0 += 1;
    if entry.0 == 1 {
        entry.1 = now; // primera vez en esta ventana
    }
    Ok(())
}

/// Extrae la IP del cliente desde los headers
fn extraer_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok().map(|s| s.to_string()))
        })
        .unwrap_or_else(|| "unknown".to_string())
}

// =============================================================================
// Tipos para peticiones POST (JSON body)
// =============================================================================

#[derive(Deserialize)]
struct InsertTirillaBody {
    anio: i16,
    periodo: i16,
    forma_id: i16,
    concepto_id: i32,
    monto_abs: f64,
    estatus_id: i16,
}

#[derive(Deserialize)]
struct UpdateTirillaBody {
    tir_id: i32,
    anio: i16,
    periodo: i16,
    forma_id: i16,
    concepto_id: i32,
    monto_abs: f64,
    estatus_id: i16,
}

#[derive(Deserialize)]
struct InsertDevengadoBody {
    anio: i16,
    periodo: i16,
    concepto: String,
    clasif_id: i16,
    forma_pago_id: i16,
    monto: f64,
    estatus_id: i16,
}

#[derive(Deserialize)]
struct UpdateDevengadoBody {
    dev_id: i32,
    anio: i16,
    periodo: i16,
    concepto: String,
    clasif_id: i16,
    forma_pago_id: i16,
    monto: f64,
    estatus_id: i16,
}

#[derive(Serialize)]
struct FilasAfectadas {
    filas: u64,
}

// =============================================================================
// Query params para filtros GET
// =============================================================================

/// Deserializa un `Option<T>` tratando los strings vacíos como `None`.
/// Útil para query params donde el frontend envía valores vacíos.
fn deserialize_opt_i16<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<Option<i16>, D::Error> {
    use serde::de::Error;
    let s = Option::<String>::deserialize(d)?;
    match s {
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => s.parse::<i16>().map(Some).map_err(D::Error::custom),
        None => Ok(None),
    }
}

#[derive(Deserialize)]
struct TirillasFiltro {
    #[serde(default, deserialize_with = "deserialize_opt_i16")]
    anio: Option<i16>,
    #[serde(default, deserialize_with = "deserialize_opt_i16")]
    periodo: Option<i16>,
}

#[derive(Deserialize)]
struct AnioFiltro {
    #[serde(default, deserialize_with = "deserialize_opt_i16")]
    anio: Option<i16>,
}

#[derive(Deserialize)]
struct DevengadosFiltro {
    #[serde(default, deserialize_with = "deserialize_opt_i16")]
    anio: Option<i16>,
    #[serde(default, deserialize_with = "deserialize_opt_i16")]
    periodo: Option<i16>,
    #[serde(default, deserialize_with = "deserialize_opt_i16")]
    estatus_id: Option<i16>,
}

// =============================================================================
// Handler: LOGIN
// =============================================================================

async fn api_login(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<LoginBody>,
) -> Result<Json<LoginResponse>, (StatusCode, Json<MsgError>)> {
    // Rate limiting
    let ip = extraer_ip(&headers);
    check_rate_limit(&state.rate_limiter, &ip).await?;
    
    if body.username != state.admin_username {
        return Err((StatusCode::UNAUTHORIZED, Json(MsgError { error: "Credenciales inválidas".into() })));
    }
    let input_hash = hex::encode(Sha256::digest(body.password.as_bytes()));
    if input_hash != state.admin_password_hash {
        return Err((StatusCode::UNAUTHORIZED, Json(MsgError { error: "Credenciales inválidas".into() })));
    }
    let token = Uuid::new_v4().to_string();
    state.sesiones.lock().await.insert(token.clone(), body.username);
    Ok(Json(LoginResponse { token }))
}

// =============================================================================
// Handlers: TIRILLAS
// =============================================================================

async fn api_get_tirillas(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<Tirilla>>, (StatusCode, String)> {
    verificar_auth(&headers, &state.sesiones).await
        .map_err(|(s, _)| (s, "No autorizado".into()))?;
    db::get_tirillas(&state.db)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn api_get_tirillas_filtradas(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(filtro): Query<TirillasFiltro>,
) -> Result<Json<Vec<Tirilla>>, (StatusCode, String)> {
    verificar_auth(&headers, &state.sesiones).await
        .map_err(|(s, _)| (s, "No autorizado".into()))?;
    db::get_tirillas_filtradas(&state.db, filtro.anio, filtro.periodo)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn api_insert_tirilla(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<InsertTirillaBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    verificar_auth(&headers, &state.sesiones).await
        .map_err(|(s, _)| (s, "No autorizado".into()))?;
    db::insert_tirilla(
        &state.db,
        body.anio,
        body.periodo,
        body.forma_id,
        body.concepto_id,
        body.monto_abs,
        body.estatus_id,
    )
    .await
    .map(|_| StatusCode::CREATED)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn api_update_tirilla(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateTirillaBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    db::update_tirilla(
        &state.db,
        body.tir_id,
        body.anio,
        body.periodo,
        body.forma_id,
        body.concepto_id,
        body.monto_abs,
        body.estatus_id,
    )
    .await
    .map(|_| StatusCode::OK)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn api_recalcular_total(
    State(state): State<Arc<AppState>>,
) -> Result<Json<FilasAfectadas>, (StatusCode, String)> {
    db::recalcular_monto_total(&state.db)
        .await
        .map(|f| Json(FilasAfectadas { filas: f }))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// =============================================================================
// Handlers: DISPONIBLE
// =============================================================================

async fn api_get_disponible(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(filtro): Query<AnioFiltro>,
) -> Result<Json<Vec<DiferenciaResult>>, (StatusCode, String)> {
    verificar_auth(&headers, &state.sesiones).await
        .map_err(|(s, _)| (s, "No autorizado".into()))?;
    db::get_diferencia(&state.db, filtro.anio)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// =============================================================================
// Handlers: DEVENGADOS
// =============================================================================

async fn api_get_devengados(
    State(state): State<Arc<AppState>>,
    Query(filtro): Query<DevengadosFiltro>,
) -> Result<Json<Vec<Devengado>>, (StatusCode, String)> {
    db::get_devengados(&state.db, filtro.anio, filtro.periodo, filtro.estatus_id)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn api_insert_devengado(
    State(state): State<Arc<AppState>>,
    Json(body): Json<InsertDevengadoBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    db::insert_devengado(
        &state.db,
        body.anio,
        body.periodo,
        body.concepto,
        body.clasif_id,
        body.forma_pago_id,
        body.monto,
        body.estatus_id,
    )
    .await
    .map(|_| StatusCode::CREATED)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn api_update_devengado(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UpdateDevengadoBody>,
) -> Result<StatusCode, (StatusCode, String)> {
    db::update_devengado(
        &state.db,
        body.dev_id,
        body.anio,
        body.periodo,
        body.concepto,
        body.clasif_id,
        body.forma_pago_id,
        body.monto,
        body.estatus_id,
    )
    .await
    .map(|_| StatusCode::OK)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// =============================================================================
// Handlers: CATÁLOGOS
// =============================================================================

async fn api_get_conceptos(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Concepto>>, (StatusCode, String)> {
    db::get_conceptos(&state.db)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn api_get_formas_pago(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<FormaPago>>, (StatusCode, String)> {
    db::get_formas_pago(&state.db)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn api_get_clasif_egresos(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ClasifEgreso>>, (StatusCode, String)> {
    db::get_clasif_egresos(&state.db)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn api_get_deudas(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<DeudaPendiente>>, (StatusCode, String)> {
    db::get_deudas_pendientes(&state.db)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// =============================================================================
// Inicio del servidor
// =============================================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;

    let sesiones: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    let rate_limiter: RateLimitStore = Arc::new(Mutex::new(HashMap::new()));
    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_password = std::env::var("ADMIN_PASSWORD").expect("ADMIN_PASSWORD must be set");
    let admin_password_hash = hex::encode(Sha256::digest(admin_password.as_bytes()));

    let state = Arc::new(AppState { db: pool, sesiones, rate_limiter, admin_username, admin_password_hash });

    let cors = CorsLayer::permissive();

    let app = Router::new()
        // Servir frontend
        .route("/", get(|| async { Html(include_str!("../static/index.html")) }))
        // Login
        .route("/api/login", post(api_login))
        // Tirillas
        .route("/api/tirillas", get(api_get_tirillas).post(api_insert_tirilla))
        .route("/api/tirillas/filtradas", get(api_get_tirillas_filtradas))
        .route("/api/tirillas/actualizar", post(api_update_tirilla))
        .route("/api/tirillas/recalcular", post(api_recalcular_total))
        // Disponible
        .route("/api/disponible", get(api_get_disponible))
        // Devengados
        .route("/api/devengados", get(api_get_devengados).post(api_insert_devengado))
        .route("/api/devengados/actualizar", post(api_update_devengado))
        // Catálogos
        .route("/api/conceptos", get(api_get_conceptos))
        .route("/api/formas-pago", get(api_get_formas_pago))
        .route("/api/clasif-egresos", get(api_get_clasif_egresos))
        .route("/api/deudas", get(api_get_deudas))
        // Middleware
        .layer(cors)
        .with_state(state);

    let addr = "0.0.0.0:3000";
    println!("🚀 Servidor corriendo en http://{}", addr);
    println!("🌐 Frontend: http://localhost:3000/");
    println!("📋 API endpoints:");
    println!("   GET|POST /api/tirillas");
    println!("   GET      /api/tirillas/filtradas?anio=&periodo=");
    println!("   POST     /api/tirillas/actualizar");
    println!("   POST     /api/tirillas/recalcular");
    println!("   GET      /api/disponible?anio=");
    println!("   GET|POST /api/devengados");
    println!("   POST     /api/devengados/actualizar");
    println!("   GET      /api/conceptos");
    println!("   GET      /api/formas-pago");
    println!("   GET      /api/clasif-egresos");
    println!("   GET      /api/deudas");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}