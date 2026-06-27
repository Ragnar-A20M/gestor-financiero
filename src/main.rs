//! Servidor web para el Gestor Financiero.
//!
//! Sirve el frontend HTML y expone las APIs REST para manejar
//! tirillas, devengados, disponible y catálogos.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

mod db;
use db::{
    ClasifEgreso, Concepto, DeudaPendiente, Devengado, DiferenciaResult, FormaPago, Tirilla,
};

/// Estado compartido: pool de conexiones a PostgreSQL.
struct AppState {
    db: PgPool,
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

#[derive(Deserialize)]
struct TirillasFiltro {
    anio: Option<i16>,
    periodo: Option<i16>,
}

#[derive(Deserialize)]
struct AnioFiltro {
    anio: Option<i16>,
}

#[derive(Deserialize)]
struct DevengadosFiltro {
    anio: Option<i16>,
    periodo: Option<i16>,
    estatus_id: Option<i16>,
}

// =============================================================================
// Handlers: TIRILLAS
// =============================================================================

async fn api_get_tirillas(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Tirilla>>, (StatusCode, String)> {
    db::get_tirillas(&state.db)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn api_get_tirillas_filtradas(
    State(state): State<Arc<AppState>>,
    Query(filtro): Query<TirillasFiltro>,
) -> Result<Json<Vec<Tirilla>>, (StatusCode, String)> {
    db::get_tirillas_filtradas(&state.db, filtro.anio, filtro.periodo)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn api_insert_tirilla(
    State(state): State<Arc<AppState>>,
    Json(body): Json<InsertTirillaBody>,
) -> Result<StatusCode, (StatusCode, String)> {
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
    Query(filtro): Query<AnioFiltro>,
) -> Result<Json<Vec<DiferenciaResult>>, (StatusCode, String)> {
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

    let state = Arc::new(AppState { db: pool });

    let cors = CorsLayer::permissive();

    let app = Router::new()
        // Servir frontend
        .route("/", get(|| async { Html(include_str!("../static/index.html")) }))
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