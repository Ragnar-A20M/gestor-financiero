//! Módulo de acceso a base de datos PostgreSQL para el Gestor Financiero.
//!
//! Contiene los modelos de datos (structs) y funciones asíncronas para consultar
//! y modificar las tablas de ingresos (tirillas), egresos (devengado) y catálogos.

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::PgPool;
use thiserror::Error;

/// Error personalizado para operaciones de base de datos.
///
/// Encapsula errores de sqlx y proporciona mensajes descriptivos.
#[derive(Debug, Error)]
pub enum AppError {
    /// Error de base de datos (sqlx)
    #[error("Error de base de datos: {0}")]
    Database(#[from] sqlx::Error),

    /// Operación no permitida
    #[error("Operación no permitida: {0}")]
    Forbidden(String),
}

impl From<AppError> for String {
    fn from(e: AppError) -> String {
        e.to_string()
    }
}

// =============================================================================
// Modelos de datos
// =============================================================================

/// Representa un registro de la tabla `ingresos.tirillas`.
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Tirilla {
    pub tir_id: i32,
    pub anio: i16,
    pub periodo: i16,
    pub forma_id: i16,
    pub concepto_id: i32,
    pub concepto: String,
    pub monto_abs: f64,
    pub estatus_id: i16,
    pub monto_total: Option<f64>,
}

/// Representa un registro de la tabla `egresos.devengado` con joins a catálogos.
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Devengado {
    pub dev_id: i32,
    pub anio: i16,
    pub periodo: i16,
    pub concepto: String,
    pub clasif_id: i16,
    pub desc_clas: String,
    pub forma_pago_id: i16,
    pub desc_fp: String,
    pub monto: f64,
    pub estatus_id: i16,
}

/// Resultado del cálculo de diferencia entre ingresos y egresos por período.
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DiferenciaResult {
    pub anio: i16,
    pub periodo: i16,
    pub diferencia: f64,
}

/// Resultado de la consulta de deudas pendientes (clasif_id = 7, estatus_id = 0).
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DeudaPendiente {
    pub desc_clas: String,
    pub concepto: String,
    pub total_monto: f64,
}

/// Catálogo de conceptos de nómina.
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Concepto {
    pub concept_id: i32,
    pub concepto: String,
    pub acr_concepto: String,
}

/// Catálogo de formas de pago.
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct FormaPago {
    pub fp_id: i16,
    pub desc_fp: String,
}

/// Catálogo de clasificaciones de egresos.
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ClasifEgreso {
    pub clas_id: i16,
    pub desc_clas: String,
}

// =============================================================================
// Funciones: TIRILLAS (ingresos)
// =============================================================================

/// Obtiene todas las tirillas ordenadas por año y período.
pub async fn get_tirillas(pool: &PgPool) -> Result<Vec<Tirilla>, sqlx::Error> {
    let rows = sqlx::query_as::<_, Tirilla>(
        "SELECT tir.tir_id, tir.anio, tir.periodo, tir.forma_id, tir.concepto_id, \
                c.concepto, tir.monto_abs::float8, tir.estatus_id, tir.monto_total::float8 \
         FROM ingresos.tirillas tir \
         JOIN catalogos.conceptos c ON tir.concepto_id = c.concept_id \
         ORDER BY c.tipo_id IN (1,3) DESC, tir.anio DESC, tir.periodo DESC"
    )
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Obtiene tirillas filtradas opcionalmente por año y/o período.
///
/// Usa `sqlx::QueryBuilder` con binds parametrizados para evitar inyección SQL.
pub async fn get_tirillas_filtradas(
    pool: &PgPool,
    anio: Option<i16>,
    periodo: Option<i16>,
) -> Result<Vec<Tirilla>, sqlx::Error> {
    use sqlx::QueryBuilder;

    let mut builder = QueryBuilder::new(
        "SELECT tir.tir_id, tir.anio, tir.periodo, tir.forma_id, tir.concepto_id, \
                c.concepto, tir.monto_abs::float8, tir.estatus_id, tir.monto_total::float8 \
         FROM ingresos.tirillas tir \
         JOIN catalogos.conceptos c ON tir.concepto_id = c.concept_id \
         WHERE 1=1"
    );

    if let Some(a) = anio {
        builder.push(" AND tir.anio = ");
        builder.push_bind(a);
    }
    if let Some(p) = periodo {
        builder.push(" AND tir.periodo = ");
        builder.push_bind(p);
    }
    builder.push(" ORDER BY c.tipo_id IN (1,3) DESC, tir.anio DESC, tir.periodo DESC");

    let rows = builder
        .build_query_as::<Tirilla>()
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Calcula la diferencia entre ingresos netos y egresos netos por período.
///
/// Usa `FULL OUTER JOIN` para incluir períodos que solo existen en una tabla
/// y columnas en minúsculas para compatibilidad con el esquema.
pub async fn get_diferencia(pool: &PgPool, anio: Option<i16>) -> Result<Vec<DiferenciaResult>, sqlx::Error> {
    use sqlx::QueryBuilder;

    let mut builder = QueryBuilder::new(
        "WITH \
         Neto_Tirillas AS ( \
             SELECT tir.anio, tir.periodo, COALESCE(SUM(tir.monto_total), 0::numeric) AS ingreso_neto \
             FROM ingresos.tirillas AS tir \
             WHERE tir.estatus_id = 0"
    );

    if let Some(a) = anio {
        builder.push(" AND tir.anio = ");
        builder.push_bind(a);
    }

    builder.push(
        " GROUP BY tir.anio, tir.periodo \
         ), \
         Neto_Devengado AS ( \
             SELECT dev.anio, dev.periodo, COALESCE(SUM(dev.monto), 0::numeric) AS devengado_neto \
             FROM egresos.devengado AS dev \
             WHERE dev.estatus_id = 0
             AND dev.forma_pago_id != 4"
    );

    if let Some(a) = anio {
        builder.push(" AND dev.anio = ");
        builder.push_bind(a);
    }

    builder.push(
        " GROUP BY dev.anio, dev.periodo \
         ) \
         SELECT \
             COALESCE(nt.anio, nd.anio) AS anio, \
             COALESCE(nt.periodo, nd.periodo) AS periodo, \
             (COALESCE(nt.ingreso_neto, 0::numeric) - COALESCE(nd.devengado_neto, 0::numeric))::float8 AS diferencia \
         FROM Neto_Tirillas AS nt \
         FULL OUTER JOIN Neto_Devengado AS nd \
             ON nt.anio = nd.anio AND nt.periodo = nd.periodo \
         ORDER BY anio ASC, periodo ASC"
    );

    let rows = builder
        .build_query_as::<DiferenciaResult>()
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Actualiza una tirilla existente (todos los campos editables).
pub async fn update_tirilla(
    pool: &PgPool,
    tir_id: i32,
    anio: i16,
    periodo: i16,
    forma_id: i16,
    concepto_id: i32,
    monto_abs: f64,
    estatus_id: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE ingresos.tirillas \
         SET anio = $1, periodo = $2, forma_id = $3, concepto_id = $4, \
             monto_abs = $5, estatus_id = $6 \
         WHERE tir_id = $7",
    )
    .bind(anio)
    .bind(periodo)
    .bind(forma_id)
    .bind(concepto_id)
    .bind(monto_abs)
    .bind(estatus_id)
    .bind(tir_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Inserta una nueva tirilla.
pub async fn insert_tirilla(
    pool: &PgPool,
    anio: i16,
    periodo: i16,
    forma_id: i16,
    concepto_id: i32,
    monto_abs: f64,
    estatus_id: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ingresos.tirillas (anio, periodo, forma_id, concepto_id, monto_abs, estatus_id) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(anio)
    .bind(periodo)
    .bind(forma_id)
    .bind(concepto_id)
    .bind(monto_abs)
    .bind(estatus_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Inserta una tirilla en múltiples periodos consecutivos.
///
/// Usa `generate_series` para insertar un registro con los mismos datos
/// en cada periodo desde `periodo_inicio` hasta `periodo_fin`.
pub async fn insert_tirilla_multi(
    pool: &PgPool,
    anio: i16,
    periodo_inicio: i16,
    periodo_fin: i16,
    forma_id: i16,
    concepto_id: i32,
    monto_abs: f64,
    estatus_id: i16,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO ingresos.tirillas (anio, periodo, forma_id, concepto_id, monto_abs, estatus_id) \
         SELECT $1, generate_series($2::int, $3::int)::smallint, $4, $5, $6, $7"
    )
    .bind(anio)
    .bind(periodo_inicio)
    .bind(periodo_fin)
    .bind(forma_id)
    .bind(concepto_id)
    .bind(monto_abs)
    .bind(estatus_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Recalcula `monto_total` según el tipo de concepto (positivo para tipo 1,3; negativo para los demás).
pub async fn recalcular_monto_total(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"UPDATE ingresos.tirillas tir
           SET monto_total = 
               CASE WHEN c.tipo_id IN (1,3) THEN tir.monto_abs
                    ELSE tir.monto_abs * (-1)
               END
           FROM catalogos.conceptos c
           WHERE tir.concepto_id = c.concept_id
             AND (tir.monto_total IS DISTINCT FROM 
               CASE WHEN c.tipo_id IN (1,3) THEN tir.monto_abs
                    ELSE tir.monto_abs * (-1)
               END)"#
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

// =============================================================================
// Funciones: DEVENGADO (egresos)
// =============================================================================

/// Obtiene devengados filtrados opcionalmente por año, período y/o estatus.
pub async fn get_devengados(
    pool: &PgPool,
    anio: Option<i16>,
    periodo: Option<i16>,
    estatus_id: Option<i16>,
) -> Result<Vec<Devengado>, sqlx::Error> {
    use sqlx::QueryBuilder;

    let mut builder = QueryBuilder::new(
        "SELECT d.dev_id, d.anio, d.periodo, d.concepto, \
                d.clasif_id, ce.desc_clas, \
                d.forma_pago_id, fp.desc_fp, \
                d.monto::float8, d.estatus_id \
         FROM egresos.devengado d \
         JOIN catalogos.clasif_egreso ce ON d.clasif_id = ce.clas_id \
         JOIN segmento.forma_pago fp ON d.forma_pago_id = fp.fp_id \
         WHERE 1=1"
    );

    if let Some(a) = anio {
        builder.push(" AND d.anio = ");
        builder.push_bind(a);
    }
    if let Some(p) = periodo {
        builder.push(" AND d.periodo = ");
        builder.push_bind(p);
    }
    if let Some(e) = estatus_id {
        builder.push(" AND d.estatus_id = ");
        builder.push_bind(e);
    }
    builder.push(" ORDER BY d.anio, d.periodo, d.concepto");

    let rows = builder
        .build_query_as::<Devengado>()
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Actualiza un devengado existente.
pub async fn update_devengado(
    pool: &PgPool,
    dev_id: i32,
    anio: i16,
    periodo: i16,
    concepto: String,
    clasif_id: i16,
    forma_pago_id: i16,
    monto: f64,
    estatus_id: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE egresos.devengado \
         SET anio=$1, periodo=$2, concepto=$3, clasif_id=$4, \
             forma_pago_id=$5, monto=$6, estatus_id=$7 \
         WHERE dev_id=$8"
    )
    .bind(anio)
    .bind(periodo)
    .bind(&concepto)
    .bind(clasif_id)
    .bind(forma_pago_id)
    .bind(monto)
    .bind(estatus_id)
    .bind(dev_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Inserta un nuevo devengado.
pub async fn insert_devengado(
    pool: &PgPool,
    anio: i16,
    periodo: i16,
    concepto: String,
    clasif_id: i16,
    forma_pago_id: i16,
    monto: f64,
    estatus_id: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO egresos.devengado (anio, periodo, concepto, clasif_id, forma_pago_id, monto, estatus_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(anio)
    .bind(periodo)
    .bind(&concepto)
    .bind(clasif_id)
    .bind(forma_pago_id)
    .bind(monto)
    .bind(estatus_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Inserta un devengado en múltiples periodos consecutivos.
///
/// Usa `generate_series` para insertar un registro con los mismos datos
/// en cada periodo desde `periodo_inicio` hasta `periodo_fin`.
pub async fn insert_devengado_multi(
    pool: &PgPool,
    anio: i16,
    periodo_inicio: i16,
    periodo_fin: i16,
    concepto: &str,
    clasif_id: i16,
    forma_pago_id: i16,
    monto: f64,
    estatus_id: i16,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO egresos.devengado (anio, periodo, concepto, clasif_id, forma_pago_id, monto, estatus_id) \
         SELECT $1, generate_series($2::int, $3::int)::smallint, $4, $5, $6, $7, $8"
    )
    .bind(anio)
    .bind(periodo_inicio)
    .bind(periodo_fin)
    .bind(concepto)
    .bind(clasif_id)
    .bind(forma_pago_id)
    .bind(monto)
    .bind(estatus_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

// =============================================================================
// Funciones: CATÁLOGOS
// =============================================================================

/// Obtiene las deudas pendientes (clasif_id = 7, estatus_id = 0) agrupadas por clasificación y concepto.
pub async fn get_deudas_pendientes(pool: &PgPool) -> Result<Vec<DeudaPendiente>, sqlx::Error> {
    let rows = sqlx::query_as::<_, DeudaPendiente>(
        "SELECT ce.desc_clas, dev.concepto, SUM(dev.monto)::float8 AS total_monto \
         FROM egresos.devengado dev \
         INNER JOIN catalogos.clasif_egreso ce ON dev.clasif_id = ce.clas_id \
         WHERE dev.clasif_id = 7 AND dev.estatus_id = 0 \
         GROUP BY ce.desc_clas, dev.concepto \
         ORDER BY dev.concepto"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Obtiene todos los conceptos del catálogo.
pub async fn get_conceptos(pool: &PgPool) -> Result<Vec<Concepto>, sqlx::Error> {
    let rows = sqlx::query_as::<_, Concepto>(
        "SELECT concept_id, concepto, acr_concepto FROM catalogos.conceptos ORDER BY concepto"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Obtiene todas las formas de pago.
pub async fn get_formas_pago(pool: &PgPool) -> Result<Vec<FormaPago>, sqlx::Error> {
    let rows = sqlx::query_as::<_, FormaPago>(
        "SELECT fp_id, desc_fp FROM segmento.forma_pago ORDER BY desc_fp"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Obtiene todas las clasificaciones de egresos.
pub async fn get_clasif_egresos(pool: &PgPool) -> Result<Vec<ClasifEgreso>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ClasifEgreso>(
        "SELECT clas_id, desc_clas FROM catalogos.clasif_egreso ORDER BY desc_clas"
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}