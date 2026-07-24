//! Examen interactivo de Rust — 5 niveles basados en el proyecto Gestor_Financiero
//!
//! Compilar:  rustc examen_rust.rs -o examen_rust
//! Ejecutar:  ./examen_rust

use std::io::{self, Write};

// =============================================================================
// ESTRUCTURA DE PREGUNTAS
// =============================================================================

struct Pregunta {
    enunciado: &'static str,
    opciones: [&'static str; 4],
    correcta: usize, // 0..3
}

// =============================================================================
// NIVEL 1 — Sintaxis básica, tipos, macros, traits derivados
// =============================================================================

const NIVEL_1: &[Pregunta] = &[
    Pregunta {
        enunciado: "¿Qué tipo de dato usa Rust para representar el campo `anio` en la struct `Tirilla`?",
        opciones: ["a) u16", "b) i16", "c) i32", "d) u32"],
        correcta: 1,
    },
    Pregunta {
        enunciado: "¿Qué crate se usa en Cargo.toml como framework web para este proyecto?",
        opciones: ["a) actix-web", "b) axum", "c) rocket", "d) warp"],
        correcta: 1,
    },
    Pregunta {
        enunciado: "¿Qué trait deriva `Tirilla` en db.rs que permite mapear filas SQL directamente?",
        opciones: ["a) Serialize", "b) FromRow", "c) Deserialize", "d) Debug"],
        correcta: 1,
    },
    Pregunta {
        enunciado: "¿Cómo se declara una función asíncrona en Rust?",
        opciones: ["a) fn async mi_func()", "b) async fn mi_func()", "c) fn mi_func() -> async", "d) async fn mi_func"],
        correcta: 1,
    },
    Pregunta {
        enunciado: "¿Qué keyword de Rust se usa para 'pedir prestado' un valor sin tomar ownership?",
        opciones: ["a) ref", "b) &", "c) borrow", "d) let mut"],
        correcta: 1,
    },
];

// =============================================================================
// NIVEL 2 — Option, Result, serde, manejo de errores, HashMap
// =============================================================================

const NIVEL_2: &[Pregunta] = &[
    Pregunta {
        enunciado: "En `api_login`, ¿qué wrapper envuelve a `AppState` para permitir acceso compartido entre hilos?",
        opciones: ["a) Box<AppState>", "b) Arc<AppState>", "c) Rc<AppState>", "d) Mutex<AppState>"],
        correcta: 1,
    },
    Pregunta {
        enunciado: "¿Qué devuelve `verificar_auth` si el token no existe en el HashMap de sesiones?",
        opciones: [
            "a) Ok(false)",
            "b) Err con StatusCode::UNAUTHORIZED",
            "c) Una String vacía",
            "d) Un panic!()",
        ],
        correcta: 1,
    },
    Pregunta {
        enunciado: "En `get_tirillas_filtradas`, ¿qué struct de sqlx construye consultas dinámicas sin inyección SQL?",
        opciones: ["a) SqlBuilder", "b) QueryBuilder", "c) DynQuery", "d) PreparedQuery"],
        correcta: 1,
    },
    Pregunta {
        enunciado: "¿Qué combinator de Result se usa en `extraer_ip` para encadenar varias operaciones que pueden fallar?",
        opciones: ["a) .map().and_then()", "b) .or_else()", "c) .unwrap_or_else()", "d) .ok_or()"],
        correcta: 2,
    },
    Pregunta {
        enunciado: "¿Qué atributo de serde permite dar un valor por defecto cuando un campo no viene en el JSON?",
        opciones: ["a) #[serde(default)]", "b) #[serde(optional)]", "c) #[serde(skip)]", "d) #[serde(ignore)]"],
        correcta: 0,
    },
];

// =============================================================================
// NIVEL 3 — Concurrencia, Mutex, Arc, async/await, lifetime, traits
// =============================================================================

const NIVEL_3: &[Pregunta] = &[
    Pregunta {
        enunciado: "¿Por qué en `AppState` se usa `Arc<Mutex<HashMap<...>>>` en lugar de solo `Mutex<HashMap<...>>`?",
        opciones: [
            "a) Para poder clonar el estado y compartirlo entre múltiples hilos de tokio",
            "b) Para hacer el HashMap más rápido",
            "c) Porque Mutex requiere Arc para funcionar",
            "d) Para evitar el borrow checker",
        ],
        correcta: 0,
    },
    Pregunta {
        enunciado: "¿Qué diferencia clave hay entre `unwrap()` y `?` en el manejo de `Result`?",
        opciones: [
            "a) Ninguna, son equivalentes",
            "b) `unwrap()` causa panic! si hay error; `?` propaga el error al caller",
            "c) `?` solo funciona en funciones asíncronas",
            "d) `unwrap()` convierte Err en Ok",
        ],
        correcta: 1,
    },
    Pregunta {
        enunciado: "En `update_tirilla`, los parámetros se pasan con `.bind()`. ¿Qué ventaja tiene sobre interpolación de strings?",
        opciones: [
            "a) Es más rápido de escribir",
            "b) Previene inyección SQL al escapar los valores automáticamente",
            "c) Permite usar cualquier tipo de dato",
            "d) b y c son correctas",
        ],
        correcta: 3,
    },
    Pregunta {
        enunciado: "¿Qué hace `COALESCE(e.estatus, '')` en la consulta SQL de `get_tirillas`?",
        opciones: [
            "a) Retorna NULL si e.estatus es vacío",
            "b) Retorna '' (string vacío) si e.estatus es NULL",
            "c) Concatena e.estatus con ''",
            "d) Lanza un error si e.estatus es NULL",
        ],
        correcta: 1,
    },
    Pregunta {
        enunciado: "¿Qué representa el patrón `Result<T, E>` en las funciones de db.rs?",
        opciones: [
            "a) Un valor que siempre es exitoso",
            "b) Un valor opcional que puede ser Some o None",
            "c) Un valor que puede ser Ok(T) si éxito o Err(E) si falla",
            "d) Un valor que se repite hasta ser exitoso",
        ],
        correcta: 2,
    },
];

// =============================================================================
// NIVEL 4 — SQL dinámico, funciones avanzadas, clausuras, genéricos
// =============================================================================

const NIVEL_4: &[Pregunta] = &[
    Pregunta {
        enunciado: "En `get_diferencia`, se usa `FULL OUTER JOIN`. ¿Qué garantiza esta cláusula sobre los resultados?",
        opciones: [
            "a) Solo muestra períodos que existen en ambas tablas",
            "b) Incluye períodos aunque solo existan en tirillas O solo en devengados",
            "c) Excluye períodos sin datos en ninguna tabla",
            "d) Duplica los registros cuando hay coincidencias",
        ],
        correcta: 1,
    },
    Pregunta {
        enunciado: "¿Qué función de ventana (window function) usa `get_cascada` para calcular el acumulado?",
        opciones: ["a) ROW_NUMBER()", "b) SUM() OVER (ORDER BY ...)", "c) LAG()", "d) RANK()"],
        correcta: 1,
    },
    Pregunta {
        enunciado: "¿Qué hace `generate_series($2::int, $3::int)::smallint` en `insert_tirilla_multi`?",
        opciones: [
            "a) Genera una serie de números aleatorios",
            "b) Genera una secuencia de enteros desde periodo_inicio hasta periodo_fin",
            "c) Genera una serie de fechas",
            "d) Crea una tabla temporal",
        ],
        correcta: 1,
    },
    Pregunta {
        enunciado: "¿Qué macro de Rust permite escribir strings multi-línea con contenido SQL de forma legible?",
        opciones: [
            "a) println!()",
            "b) format!()",
            "c) El lenguaje no necesita macro, se usa un string raw r#\"...\"#",
            "d) sql!()",
        ],
        correcta: 2,
    },
    Pregunta {
        enunciado: "En `recalcular_monto_total`, ¿qué significa `IS DISTINCT FROM` en la cláusula WHERE?",
        opciones: [
            "a) Es igual a `!=` pero trata NULL como un valor distinguible",
            "b) Es lo mismo que `<>`",
            "c) Compara si dos valores son exactamente iguales",
            "d) Ignora los valores nulos",
        ],
        correcta: 0,
    },
];

// =============================================================================
// NIVEL 5 — Arquitectura, diseño, Caso concreto, depuración
// =============================================================================

const NIVEL_5: &[Pregunta] = &[
    Pregunta {
        enunciado: "Si un endpoint no requiere autenticación pero por error se le aplica `verificar_auth`, ¿qué código HTTP retornará si no hay token?",
        opciones: [
            "a) 200 OK",
            "b) 401 Unauthorized",
            "c) 403 Forbidden",
            "d) 500 Internal Server Error",
        ],
        correcta: 1,
    },
    Pregunta {
        enunciado: "En `api_query_tirillas`, se usa `any()` en la ruta. ¿Por qué es necesario en lugar de `get()` o `post()`?",
        opciones: [
            "a) Porque axum 0.7 no soporta el método HTTP QUERY en MethodFilter",
            "b) Porque es más rápido",
            "c) Porque la ruta necesita aceptar cualquier método",
            "d) Para evitar el CORS",
        ],
        correcta: 0,
    },
    Pregunta {
        enunciado: "¿Qué problema de concurrencia resuelve `Arc<Mutex<HashMap>>` en el rate limiter?",
        opciones: [
            "a) Que múltiples hilos lean y escriban el HashMap sin condiciones de carrera",
            "b) Que el HashMap sea inmutable",
            "c) Que el HashMap sea más rápido en un solo hilo",
            "d) Que los datos persistan entre reinicios del servidor",
        ],
        correcta: 0,
    },
    Pregunta {
        enunciado: "¿Por qué `extraer_ip` revisa primero `x-forwarded-for` y luego `x-real-ip`?",
        opciones: [
            "a) Porque x-forwarded-for es el estándar cuando hay proxy/reverse proxy",
            "b) Porque x-real-ip es más confiable",
            "c) Porque x-forwarded-for siempre existe",
            "d) Es una decisión arbitraria sin razón técnica",
        ],
        correcta: 0,
    },
    Pregunta {
        enunciado: "En `check_rate_limit`, ¿qué pasaría si dos peticiones simultáneas intentan actualizar el mismo contador?",
        opciones: [
            "a) Se perdería una actualización por race condition — por eso está dentro de Mutex",
            "b) Funcionaría perfectamente porque Rust lo maneja automáticamente",
            "c) El servidor crashearía con panic!",
            "d) Solo la primera petición se procesaría",
        ],
        correcta: 0,
    },
];

// =============================================================================
// LÓGICA DEL EXAMEN
// =============================================================================

const NIVELES: &[(&str, &[Pregunta])] = &[
    ("NIVEL 1 — Conceptos básicos de Rust (tipos, macros, sintaxis)", NIVEL_1),
    ("NIVEL 2 — Option, Result, serde, manejo de errores", NIVEL_2),
    ("NIVEL 3 — Concurrencia, async/await, Mutex, Arc", NIVEL_3),
    ("NIVEL 4 — SQL dinámico, window functions, generics", NIVEL_4),
    ("NIVEL 5 — Arquitectura, diseño, casos prácticos", NIVEL_5),
];

fn leer_opcion() -> usize {
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Error al leer");
        match input.trim() {
            "1" => return 0,
            "2" => return 1,
            "3" => return 2,
            "4" => return 3,
            _ => {
                print!("  Opción inválida. Elige 1-4: ");
                io::stdout().flush().unwrap();
            }
        }
    }
}

fn ejecutar_nivel(nivel: &[Pregunta]) -> (usize, usize) {
    let mut correctas = 0;
    let total = nivel.len();

    for (i, p) in nivel.iter().enumerate() {
        println!("\n  Pregunta {} de {}", i + 1, total);
        println!("  ─────────────────────────────────────────────");
        println!("  {}", p.enunciado);
        for op in &p.opciones {
            println!("    {}", op);
        }
        print!("\n  Tu respuesta (1-4): ");
        io::stdout().flush().unwrap();

        let respuesta = leer_opcion();
        if respuesta == p.correcta {
            println!("  ✅ ¡Correcto!\n");
            correctas += 1;
        } else {
            let correcta_letra = match p.correcta {
                0 => "1",
                1 => "2",
                2 => "3",
                _ => "4",
            };
            println!("  ❌ Incorrecto. La respuesta correcta era opción {}", correcta_letra);
            let txt_correcto = p.opciones[p.correcta];
            println!("     {}", txt_correcto);
            println!();
        }
    }

    (correctas, total)
}

fn mostrar_resultado_nivel(nombre: &str, correctas: usize, total: usize) {
    let pct = (correctas as f64 / total as f64) * 100.0;
    println!("  ─────────────────────────────────────────────");
    println!("  {}: {}/{} correctas ({:.0}%)", nombre, correctas, total, pct);
    if pct >= 80.0 {
        println!("  🏆 ¡Excelente! Dominas este nivel.\n");
    } else if pct >= 60.0 {
        println!("  👍 Bien, pero puedes repasar algunos conceptos.\n");
    } else {
        println!("  📚 Te sugiero repasar este tema con más calma.\n");
    }
}

fn main() {
    println!();
    println!("  ╔══════════════════════════════════════════════════╗");
    println!("  ║   EXAMEN INTERACTIVO DE RUST                    ║");
    println!("  ║   Basado en tu proyecto Gestor_Financiero       ║");
    println!("  ║   5 niveles de dificultad progresiva            ║");
    println!("  ╚══════════════════════════════════════════════════╝");
    println!();
    println!("  Responde cada pregunta eligiendo 1, 2, 3 o 4.");
    println!("  Al final de cada nivel verás tu puntuación.");
    println!();

    let mut total_correctas = 0;
    let mut total_preguntas = 0;

    for (nombre, preguntas) in NIVELES {
        println!("  █████████████████████████████████████████████████████████");
        println!("  █  {}", nombre);
        println!("  █████████████████████████████████████████████████████████");
        println!();

        let (corr, tot) = ejecutar_nivel(preguntas);
        mostrar_resultado_nivel(nombre, corr, tot);

        total_correctas += corr;
        total_preguntas += tot;

        // Pausa entre niveles
        if nombre != &NIVELES[NIVELES.len() - 1].0 {
            print!("  Presiona Enter para continuar al siguiente nivel...");
            io::stdout().flush().unwrap();
            let mut pausa = String::new();
            io::stdin().read_line(&mut pausa).unwrap();
            println!();
        }
    }

    // Resultado final
    let pct_final = (total_correctas as f64 / total_preguntas as f64) * 100.0;
    println!();
    println!("  ╔══════════════════════════════════════════════════╗");
    println!("  ║            RESULTADO FINAL                      ║");
    println!("  ╠══════════════════════════════════════════════════╣");
    println!("  ║  Total: {}/{} preguntas correctas ({:.0}%)       ║", total_correctas, total_preguntas, pct_final);
    println!("  ╚══════════════════════════════════════════════════╝");
    println!();

    if pct_final >= 90.0 {
        println!("  🌟 ¡Impresionante! Tienes un dominio excepcional de Rust aplicado a tu proyecto.");
    } else if pct_final >= 75.0 {
        println!("  🎉 ¡Muy bien! Tienes un buen entendimiento de Rust en tu proyecto.");
    } else if pct_final >= 60.0 {
        println!("  💪 Bien, pero hay áreas que puedes reforzar. ¡Sigue practicando!");
    } else {
        println!("  📖 Vale la pena repasar los fundamentos. ¡No te rindas!");
    }
    println!();
}