# Informe de Deploy — Gestor Financiero en Fly.io

## Resumen

Despliegue manual de la aplicación **gestor-financiero-server** en Fly.io usando binario precompilado localmente.

---

## Errores Encontrados y Soluciones

### 1. Error de sintaxis en `fly.toml`

**Problema:**
Al modificar manualmente el `fly.toml` para apuntar al `Dockerfile.deploy`, se duplicó la sección `[build]`:

```toml
[build]
  [build]          ← duplicado
  dockerfile = 'Dockerfile.deploy'
```

Esto provocaba:
```
Error: failed loading app config: table build already exists
```

**Solución:**
Eliminar la línea duplicada `[build]`, dejando solo una sección:

```toml
[build]
  dockerfile = 'Dockerfile.deploy'
```

```
       ┌─────────────────────────────────────┐
       │          fly.toml (original)        │
       │  [build]                            │
       │    dockerfile = 'Dockerfile'        │
       └──────────┬──────────────────────────┘
                  │ Edición manual incorrecta
                  ▼
       ┌─────────────────────────────────────┐
       │     fly.toml (roto)                 │
       │  [build]                            │
       │    [build]          ← DUPLICADO     │
       │    dockerfile = 'Dockerfile.deploy' │
       └──────────┬──────────────────────────┘
                  │ Corrección
                  ▼
       ┌─────────────────────────────────────┐
       │    fly.toml (corregido)             │
       │  [build]                            │
       │    dockerfile = 'Dockerfile.deploy' │
       └─────────────────────────────────────┘
```

---

### 2. Binario no encontrado en el build de Docker

**Problema:**
El archivo `.dockerignore` contenía:

```
target/
```

Esta línea ignoraba **todo** el directorio `target/`, incluyendo el binario compilado en `target/release/gestor-financiero-server`. Al ejecutar `fly deploy`, Docker no encontraba el archivo:

```
COPY target/release/gestor-financiero-server /usr/local/bin/
→ failed to calculate checksum: "/target/release/gestor-financiero-server": not found
```

**Solución:**
Agregar una excepción en `.dockerignore` para incluir solo el binario:

```
target/
!target/release/gestor-financiero-server   ← excepción
```

```
┌─────────────────────────────────┐
│       .dockerignore            │
├─────────────────────────────────┤
│  target/        ← Ignora todo  │
│  !target/release/              │
│    gestor-financiero-server    │
│                  ← Excepción   │
│  .git/                         │
│  *.md                          │
│  .env                          │
└─────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│  Contexto de Docker (build)            │
├─────────────────────────────────────────┤
│  ✅ target/release/                     │
│      gestor-financiero-server  ← Incluido│
│  ❌ target/debug/              ← Ignorado│
│  ❌ target/.fingerprint/       ← Ignorado│
└─────────────────────────────────────────┘
```

---

### 3. Error TLS al verificar antes del deploy

**Problema:**
Se ejecutó `curl https://gestor-financiero.fly.dev/health` **antes** de hacer el `fly deploy`, cuando aún no existía ninguna aplicación corriendo:

```
curl: (35) TLS connect error: error:0A000126:SSL routines::unexpected eof while reading
```

**Solución:**
Ejecutar `curl` solo **después** de que `fly deploy` haya completado exitosamente.

```
Línea de tiempo correcta:
┌─────────┐    ┌──────────┐    ┌─────────┐
│ fly     │ →  │ fly      │ →  │ curl    │
│ secrets │    │ deploy   │    │ /health │
│ set     │    │          │    │         │
└─────────┘    └──────────┘    └─────────┘
                                    │
                          ❌ Acá NO funciona
                          ✅ Acá SÍ funciona
```

---

### 4. Timeout en health checks del deploy

**Problema:**
Durante el `fly deploy`, la máquina se creó pero los health checks nunca pasaron:

```
WARN failed to release lease for machine d8953d9c925798
✖ Failed: timeout reached waiting for health checks to pass
```

La máquina quedó en estado `stopped`:

```
PROCESS │ ID             │ VERSION │ REGION │ STATE   │ CHECKS
app     │ d8953d9c925798 │ 31      │ lax    │ stopped │ 1 warning
```

**Solución:**
Iniciar la máquina manualmente después del deploy:

```bash
fly machine start d8953d9c925798
```

```
┌──────────────┐
│ fly deploy   │
│              │
│  ┌────────┐  │
│  │ Build  │  │
│  │ Image  │  │  ✅
│  └────────┘  │
│  ┌────────┐  │
│  │ Create │  │
│  │Machine │  │  ✅
│  └────────┘  │
│  ┌────────┐  │
│  │Health  │  │
│  │Checks  │  │  ❌ Timeout
│  └────────┘  │
│      ↓       │
│   Machine    │
│   STOPPED    │
└──────┬───────┘
       │ fly machine start
       ▼
┌──────────────┐
│  Machine     │
│  RUNNING     │  ✅
└──────────────┘
```

---

### 5. Error 502 — Secrets placeholder

**Problema:**
La aplicación se iniciaba pero devolvía `502 Bad Gateway` porque los secrets contenían valores de ejemplo:

```bash
DATABASE_URL="postgres://usuario:password@host:5432/tu_bd?sslmode=require"
ADMIN_PASSWORD="tu_password_segura"
```

La app fallaba al conectar a PostgreSQL con credenciales inválidas, por lo que el servidor se caía inmediatamente.

**Solución:**
Configurar los secrets con valores reales de Supabase:

```bash
fly secrets set \
  DATABASE_URL="postgres://postgres.qrxjboazrbttgssimbqp:HpTKFtZElGNYZelc@aws-1-us-east-2.pooler.supabase.com:5432/postgres?sslmode=require" \
  ADMIN_USERNAME="gf_main" \
  ADMIN_PASSWORD="SakuraLS3518"
```

```
┌──────────────┐     ┌──────────────────┐
│  fly secrets │     │  Aplicación      │
│              │     │                  │
│  DATABASE_URL├────►│  main.rs         │
│  (real)      │     │                  │
│              │     │  PgPool::connect │
│  ADMIN_*     │     │  ───────────────►│
│  (real)      │     │  ✅ Conexión OK  │
└──────────────┘     └──────────────────┘
```

---

## Estados Finales

| Componente | Estado |
|------------|--------|
| Compilación local | ✅ Exitoso (0.14s) |
| Docker image | ✅ Construida |
| Máquina Fly.io | ✅ Running (versión 31, región `lax`) |
| Health check | ✅ `{"status":"ok", "timestamp":"..."}` |
| Secrets | ✅ Desplegados |
| fly.toml | ✅ Restaurado a `Dockerfile` original |

## Archivos Modificados

| Archivo | Cambio |
|---------|--------|
| `Dockerfile.deploy` | ✅ Creado (nuevo, para deploy con binario precompilado) |
| `.dockerignore` | ✅ Agregada excepción `!target/release/gestor-financiero-server` |
| `fly.toml` | ✅ Corregido (sección `[build]` duplicada) y luego restaurado |

## URL de la aplicación

```
https://gestor-financiero.fly.dev

---

## 📝 Modificaciones — 21/Jul/2026

### Cambios realizados

#### 1. Nueva consulta de cascada acumulada en Dashboard

Se agregó un endpoint y gráfica que muestra el neto vs acumulado de tirillas con conceptos 6, 18, 24.

**Archivos modificados:**

| Archivo | Cambio |
|---------|--------|
| `src/db.rs` | ✅ Nuevo struct `CascadaResult` (anio, periodo, total, neto, acumulado) |
| `src/db.rs` | ✅ Nueva función `get_cascada()` con consulta SQL que agrupa por año/período y calcula `SUM(neto) OVER (ORDER BY anio, periodo)` |
| `src/main.rs` | ✅ Nuevo handler `api_get_cascada()` |
| `src/main.rs` | ✅ Nueva ruta `GET /api/cascada` |
| `static/index.html` | ✅ Nueva tarjeta en Dashboard con canvas `#cascadaGrafico` |
| `static/index.html` | ✅ Gráfico combinado: barras neto (verde/rojo) + línea acumulado (púrpura con área) |
| `static/index.html` | ✅ Filtro visual solo para año 2026 (acumulado real arrastrado desde años anteriores) |

#### 2. Corrección de esquema BD

Se reemplazaron las referencias incorrectas al esquema `segmento.forma_pago` por `segmentos.forma_pago`:

```diff
- JOIN segmento.forma_pago fp ON d.forma_pago_id = fp.fp_id
+ JOIN segmentos.forma_pago fp ON d.forma_pago_id = fp.fp_id

- SELECT fp_id, desc_fp FROM segmento.forma_pago
+ SELECT fp_id, desc_fp FROM segmentos.forma_pago
```
