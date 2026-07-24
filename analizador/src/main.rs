//! Analizador CLI del Gestor Financiero
//!
//! Consulta la base de datos PostgreSQL y muestra estadísticas útiles.
//!
//! Uso:
//!   cargo run -- --top-conceptos
//!   cargo run -- --deuda-total
//!   cargo run -- --analisis --anio 2025
//!   cargo run -- --balance --anio 2025
//!   cargo run -- --periodos-morosos
//!   cargo run -- --todo

use clap::Parser;
use prettytable::{Cell, Row, Table};
use sqlx::postgres::PgPoolOptions;
use sqlx::FromRow;
use std::process;

// =============================================================================
// Modelos de datos locales (solo para las consultas del analizador)
// =============================================================================

#[derive(FromRow, Debug)]
struct ConceptoCount {
    concepto: String,
    cantidad: i64,
    monto_total: f64,
}

#[derive(FromRow, Debug)]
struct DeudaConcepto {
    concepto: String,
    total_monto: f64,
    cantidad: i32,
}

#[derive(FromRow, Debug)]
struct ResumenAnual {
    anio: i16,
    periodo: i16,
    ingresos: f64,
    egresos: f64,
    disponible: f64,
}

#[derive(FromRow, Debug)]
#[allow(dead_code)]
struct PeriodoMoroso {
    anio: i16,
    periodo: i16,
    concepto: String,
    monto_pendiente: f64,
    dias_vencido: i32,
}

#[derive(FromRow, Debug)]
struct StatsRow {
    total_ingresos: Option<f64>,
    total_egresos: Option<f64>,
    tirillas_pendientes: i64,
    devengados_pendientes: i64,
}

#[derive(Debug)]
#[allow(dead_code)]
struct StatsGlobales {
    total_ingresos: f64,
    total_egresos: f64,
    balance_general: f64,
    tirillas_pendientes: i64,
    devengados_pendientes: i64,
}

#[derive(FromRow, Debug)]
#[allow(dead_code)]
struct InfoConexion {
    version_bd: String,
    tamanio_mb: f64,
    conexiones_activas: i32,
}

// =============================================================================
// CLI
// =============================================================================

#[derive(Parser, Debug)]
#[command(name = "analizador-gestor")]
#[command(about = "Analizador CLI del Gestor Financiero", long_about = None)]
struct Args {
    /// Muestra los conceptos más usados en tirillas
    #[arg(long)]
    top_conceptos: bool,

    /// Muestra el total de deudas pendientes
    #[arg(long)]
    deuda_total: bool,

    /// Análisis detallado por año y período
    #[arg(long)]
    analisis: bool,

    /// Balance general entre ingresos y egresos
    #[arg(long)]
    balance: bool,

    /// Períodos con pagos vencidos
    #[arg(long)]
    periodos_morosos: bool,

    /// Muestra todo el análisis completo
    #[arg(long)]
    todo: bool,

    /// Filtro opcional de año
    #[arg(long)]
    anio: Option<i16>,
}

// =============================================================================
// Conexión a BD
// =============================================================================

async fn conectar() -> sqlx::PgPool {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| {
            eprintln!("❌ ERROR: DATABASE_URL no está definida.");
            eprintln!("   Crea un archivo .env en Gestor_Financiero/analizador/");
            eprintln!("   con: DATABASE_URL=postgres://usuario:pass@host:5432/bd");
            process::exit(1);
        });

    // Si no tiene sslmode, lo agregamos
    let url = if database_url.contains("sslmode=") {
        database_url
    } else if database_url.contains('?') {
        format!("{}&sslmode=require", database_url)
    } else {
        format!("{}?sslmode=require", database_url)
    };

    match PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
    {
        Ok(pool) => {
            println!("✅ Conectado a PostgreSQL\n");
            pool
        }
        Err(e) => {
            eprintln!("❌ ERROR al conectar: {}", e);
            process::exit(1);
        }
    }
}

// =============================================================================
// Funciones de análisis
// =============================================================================

async fn top_conceptos(pool: &sqlx::PgPool) -> Result<Vec<ConceptoCount>, sqlx::Error> {
    sqlx::query_as::<_, ConceptoCount>(
        r#"
        SELECT 
            c.concepto,
            COUNT(*)::bigint AS cantidad,
            SUM(tir.monto_abs)::float8 AS monto_total
        FROM ingresos.tirillas tir
        JOIN catalogos.conceptos c ON tir.concepto_id = c.concept_id
        GROUP BY c.concepto
        ORDER BY cantidad DESC
        LIMIT 10
        "#,
    )
    .fetch_all(pool)
    .await
}

async fn deuda_total(pool: &sqlx::PgPool) -> Result<(Vec<DeudaConcepto>, Vec<DeudaConcepto>), sqlx::Error> {
    let tirillas = sqlx::query_as::<_, DeudaConcepto>(
        r#"
        SELECT 
            cc.concepto,
            SUM(tir.monto_abs)::float8 AS total_monto,
            COUNT(*)::int AS cantidad
        FROM ingresos.tirillas tir
        JOIN catalogos.conceptos cc ON tir.concepto_id = cc.concept_id
        WHERE tir.estatus_id = 0
          AND tir.concepto_id IN (13, 31)
        GROUP BY cc.concepto
        ORDER BY cc.concepto
        "#,
    )
    .fetch_all(pool)
    .await?;

    let egresos = sqlx::query_as::<_, DeudaConcepto>(
        r#"
        SELECT 
            dev.concepto,
            SUM(dev.monto)::float8 AS total_monto,
            COUNT(*)::int AS cantidad
        FROM egresos.devengado dev
        WHERE dev.clasif_id = 7 AND dev.estatus_id = 0
        GROUP BY dev.concepto
        ORDER BY dev.concepto
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok((tirillas, egresos))
}

async fn analisis_anual(pool: &sqlx::PgPool, anio: Option<i16>) -> Result<Vec<ResumenAnual>, sqlx::Error> {
    use sqlx::QueryBuilder;

    let mut builder = QueryBuilder::new(
        "WITH \
         Ingresos AS ( \
             SELECT tir.anio, tir.periodo, \
                    COALESCE(SUM(tir.monto_total), 0.0)::float8 AS ingresos \
             FROM ingresos.tirillas tir \
             WHERE tir.estatus_id = 0"
    );

    if let Some(a) = anio {
        builder.push(" AND tir.anio = ");
        builder.push_bind(a);
    }

    builder.push(
        " GROUP BY tir.anio, tir.periodo \
         ), \
         Egresos AS ( \
             SELECT dev.anio, dev.periodo, \
                    COALESCE(SUM(dev.monto), 0.0)::float8 AS egresos \
             FROM egresos.devengado dev \
             WHERE dev.estatus_id = 0 \
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
             COALESCE(i.anio, e.anio) AS anio, \
             COALESCE(i.periodo, e.periodo) AS periodo, \
             COALESCE(i.ingresos, 0.0) AS ingresos, \
             COALESCE(e.egresos, 0.0) AS egresos, \
             (COALESCE(i.ingresos, 0.0) - COALESCE(e.egresos, 0.0)) AS disponible \
         FROM Ingresos i \
         FULL OUTER JOIN Egresos e ON i.anio = e.anio AND i.periodo = e.periodo \
         ORDER BY anio ASC, periodo ASC"
    );

    let rows = builder
        .build_query_as::<ResumenAnual>()
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

async fn periodos_morosos(pool: &sqlx::PgPool) -> Result<Vec<PeriodoMoroso>, sqlx::Error> {
    // Asumiendo que existe una tabla catalogos.periodos con fecha_cobro
    // Si no, mostramos los registros con estatus pendiente
    let resultado_tirillas = sqlx::query_as::<_, PeriodoMoroso>(
        r#"
        SELECT 
            tir.anio,
            tir.periodo,
            c.concepto,
            tir.monto_abs::float8 AS monto_pendiente,
            0 AS dias_vencido
        FROM ingresos.tirillas tir
        JOIN catalogos.conceptos c ON tir.concepto_id = c.concept_id
        WHERE tir.estatus_id = 0
        ORDER BY tir.anio DESC, tir.periodo DESC
        LIMIT 20
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(resultado_tirillas)
}

async fn stats_globales(pool: &sqlx::PgPool) -> Result<StatsGlobales, sqlx::Error> {
    let row = sqlx::query_as::<_, StatsRow>(
        "SELECT \
           (SELECT SUM(tir.monto_total)::float8 \
            FROM ingresos.tirillas tir WHERE tir.estatus_id = 0) AS total_ingresos, \
           (SELECT SUM(dev.monto)::float8 \
            FROM egresos.devengado dev \
            WHERE dev.estatus_id = 0 AND dev.forma_pago_id != 4) AS total_egresos, \
           (SELECT COUNT(*)::bigint FROM ingresos.tirillas WHERE estatus_id = 0) AS tirillas_pendientes, \
           (SELECT COUNT(*)::bigint FROM egresos.devengado WHERE estatus_id = 0) AS devengados_pendientes"
    )
    .fetch_one(pool)
    .await?;

    Ok(StatsGlobales {
        total_ingresos: row.total_ingresos.unwrap_or(0.0),
        total_egresos: row.total_egresos.unwrap_or(0.0),
        balance_general: row.total_ingresos.unwrap_or(0.0) - row.total_egresos.unwrap_or(0.0),
        tirillas_pendientes: row.tirillas_pendientes,
        devengados_pendientes: row.devengados_pendientes,
    })
}

async fn info_bd(pool: &sqlx::PgPool) -> Result<InfoConexion, sqlx::Error> {
    sqlx::query_as::<_, InfoConexion>(
        r#"
        SELECT 
            version() AS version_bd,
            0.0 AS tamanio_mb,
            0 AS conexiones_activas
        "#,
    )
    .fetch_one(pool)
    .await
}

// =============================================================================
// Formateo de resultados
// =============================================================================

fn mostrar_tabla(title: &str, headers: &[&str], rows: &[Vec<String>]) {
    println!("\n  📊 {}\n", title);
    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_BOX_CHARS);

    let header_row = Row::new(headers.iter().map(|h| Cell::new(h)).collect());
    table.set_titles(header_row);

    for row_data in rows {
        let row = Row::new(row_data.iter().map(|c| Cell::new(c)).collect());
        table.add_row(row);
    }

    table.printstd();
    println!();
}

fn mostrar_stats_globales(stats: &StatsGlobales) {
    println!("\n  📈 BALANCE GENERAL\n");
    println!("  ┌────────────────────────────────────────────┐");
    println!("  │ {:44} │", format!("Ingresos totales:    ${:>10.2}", stats.total_ingresos));
    println!("  │ {:44} │", format!("Egresos totales:     ${:>10.2}", stats.total_egresos));
    let balance = stats.total_ingresos - stats.total_egresos;
    let signo = if balance >= 0.0 { "+" } else { "" };
    println!("  │ {:44} │", format!("Balance:             ${}{:>9.2}", signo, balance));
    println!("  │ {:44} │", "".to_string());
    println!("  │ {:44} │", format!("Tirillas pendientes:  {}", stats.tirillas_pendientes));
    println!("  │ {:44} │", format!("Devengados pendientes:{}", stats.devengados_pendientes));
    println!("  └────────────────────────────────────────────┘\n");
}

// =============================================================================
// MAIN
// =============================================================================

#[tokio::main]
async fn main() {
    println!();
    println!("  ╔═══════════════════════════════════════════════╗");
    println!("  ║   ANALIZADOR DEL GESTOR FINANCIERO           ║");
    println!("  ║   CLI interactivo para consultar la BD       ║");
    println!("  ╚═══════════════════════════════════════════════╝");
    println!();

    let args = Args::parse();
    let pool = conectar().await;

    // Si no se especifica nada o se usa --todo, ejecuta todo
    let ejecutar_todo = args.todo || (!args.top_conceptos && !args.deuda_total && !args.analisis && !args.balance && !args.periodos_morosos);

    // ---- INFO BD ----
    if let Ok(info) = info_bd(&pool).await {
        println!("  🖥  PostgreSQL: {}", info.version_bd.split(',').next().unwrap_or("desconocido"));
        println!();
    }

    // ---- TOP CONCEPTOS ----
    if ejecutar_todo || args.top_conceptos {
        match top_conceptos(&pool).await {
            Ok(conceptos) => {
                let rows: Vec<Vec<String>> = conceptos
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        vec![
                            (i + 1).to_string(),
                            c.concepto.clone(),
                            c.cantidad.to_string(),
                            format!("${:.2}", c.monto_total),
                        ]
                    })
                    .collect();
                mostrar_tabla(
                    "TOP 10 CONCEPTOS MÁS USADOS EN TIRILLAS",
                    &["#", "Concepto", "Cantidad", "Monto Total"],
                    &rows,
                );
            }
            Err(e) => eprintln!("  ⚠️  Error al obtener top conceptos: {}", e),
        }
    }

    // ---- DEUDA TOTAL ----
    if ejecutar_todo || args.deuda_total {
        match deuda_total(&pool).await {
            Ok((tirillas, egresos)) => {
                let mut rows: Vec<Vec<String>> = tirillas
                    .iter()
                    .map(|d| {
                        vec![
                            "Tirilla".to_string(),
                            d.concepto.clone(),
                            d.cantidad.to_string(),
                            format!("${:.2}", d.total_monto),
                        ]
                    })
                    .collect();

                for d in &egresos {
                    rows.push(vec![
                        "Devengado".to_string(),
                        d.concepto.clone(),
                        d.cantidad.to_string(),
                        format!("${:.2}", d.total_monto),
                    ]);
                }

                let total_deuda: f64 = tirillas.iter().map(|d| d.total_monto).sum::<f64>()
                    + egresos.iter().map(|d| d.total_monto).sum::<f64>();

                mostrar_tabla(
                    &format!("DEUDAS PENDIENTES (Total: ${:.2})", total_deuda),
                    &["Origen", "Concepto", "Cantidad", "Monto"],
                    &rows,
                );
            }
            Err(e) => eprintln!("  ⚠️  Error al obtener deudas: {}", e),
        }
    }

    // ---- ANÁLISIS ANUAL ----
    if ejecutar_todo || args.analisis {
        match analisis_anual(&pool, args.anio).await {
            Ok(resumen) => {
                let rows: Vec<Vec<String>> = resumen
                    .iter()
                    .map(|r| {
                        vec![
                            r.anio.to_string(),
                            format!("{:02}", r.periodo),
                            format!("${:.2}", r.ingresos),
                            format!("${:.2}", r.egresos),
                            if r.disponible >= 0.0 {
                                format!(" ${:.2}", r.disponible)
                            } else {
                                format!("-${:.2}", r.disponible.abs())
                            },
                        ]
                    })
                    .collect();

                let titulo = match args.anio {
                    Some(a) => format!("ANÁLISIS POR PERÍODO - Año {}", a),
                    None => "ANÁLISIS POR PERÍODO - Todos los años".to_string(),
                };

                mostrar_tabla(&titulo, &["Año", "Per", "Ingresos", "Egresos", "Disponible"], &rows);
            }
            Err(e) => eprintln!("  ⚠️  Error en análisis anual: {}", e),
        }
    }

    // ---- BALANCE GENERAL ----
    if ejecutar_todo || args.balance {
        match stats_globales(&pool).await {
            Ok(stats) => {
                mostrar_stats_globales(&stats);
            }
            Err(e) => eprintln!("  ⚠️  Error al obtener stats globales: {}", e),
        }
    }

    // ---- PERÍODOS MOROSOS ----
    if ejecutar_todo || args.periodos_morosos {
        match periodos_morosos(&pool).await {
            Ok(morosos) => {
                let rows: Vec<Vec<String>> = morosos
                    .iter()
                    .map(|m| {
                        vec![
                            format!("{}-{:02}", m.anio, m.periodo),
                            m.concepto.clone(),
                            format!("${:.2}", m.monto_pendiente),
                        ]
                    })
                    .collect();

                mostrar_tabla(
                    "REGISTROS PENDIENTES DE PAGO (estatus_id = 0)",
                    &["Período", "Concepto", "Monto Pendiente"],
                    &rows,
                );
            }
            Err(e) => eprintln!("  ⚠️  Error al obtener períodos morosos: {}", e),
        }
    }

    println!("  ✅ Análisis completado.\n");
}