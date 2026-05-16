use axum::{
    extract::{Query, State},
    http::{HeaderValue, Method, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use std::env;
use std::net::SocketAddr;

#[derive(Deserialize)]
struct Params {
    action: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
struct Producto {
    id: i32,
    nombre: Option<String>,
    precio: Option<f64>, // Ajusta según tus columnas reales
}

#[derive(Deserialize)]
struct LoginPayload {
    email: String,
    password: String,
    nombre: Option<String>,
}

#[derive(Deserialize)]
struct PagoPayload {
    nombre: String,
    direccion: String,
    pago: String,
    referencia: String,
    productos_json: String,
    total_num: f64,
}

#[derive(Serialize)]
struct ResponseGeneric {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<serde_json::Value>,
}

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[tokio::main]
async fn main() {
    let database_url = env::var("DATABASE_URL").expect("Falta DATABASE_URL");
    let pool = PgPool::connect(&database_url).await.expect("Fallo de conexión");
    let state = AppState { pool };

    let cors = CorsLayer::new()
        .allow_origin("*".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/", get(api_handler).post(api_handler_post))
        .layer(cors)
        .with_state(state);

    let port = env::var("PORT").unwrap_or_else(|_| "10000".to_string()).parse::<u16>().unwrap();
    let addr = SocketAddr::from((, port));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn api_handler(Query(params): Query<Params>, State(state): State<AppState>) -> Result<Json<serde_json::Value>, StatusCode> {
    if params.action.as_deref() == Some("productos") {
        let productos = sqlx::query_as::<_, Producto>("SELECT * FROM productos ORDER BY id ASC")
            .fetch_all(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok(Json(serde_json::to_value(productos).unwrap()));
    }
    Err(StatusCode::BAD_REQUEST)
}

async fn api_handler_post(
    Query(params): Query<Params>,
    State(state): State<AppState>,
    body: String,
) -> Result<Json<ResponseGeneric>, StatusCode> {
    let action = params.action.as_deref().unwrap_or("");

    if action == "login" {
        let payload: LoginPayload = serde_urlencoded::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
        let user_row = sqlx::query("SELECT id, nombre, email, password FROM clientes WHERE email = $1")
            .bind(&payload.email)
            .fetch_optional(&state.pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if let Some(row) = user_row {
            use sqlx::Row;
            let db_pass: String = row.get("password");
            if payload.password == db_pass {
                let user_json = serde_json::json!({ "email": payload.email, "nombre": row.get::<Option<String>, _>("nombre") });
                return Ok(Json(ResponseGeneric { status: "success".to_string(), message: None, user: Some(user_json) }));
            } else {
                return Ok(Json(ResponseGeneric { status: "error".to_string(), message: Some("Contraseña incorrecta".to_string()), user: None }));
            }
        } else {
            let nom = payload.nombre.unwrap_or_else(|| "Cliente Nuevo".to_string());
            let insert_row = sqlx::query("INSERT INTO clientes (nombre, email, password) VALUES ($1, $2, $3) RETURNING id, nombre, email")
                .bind(&nom).bind(&payload.email).bind(&payload.password)
                .fetch_one(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            use sqlx::Row;
            let user_json = serde_json::json!({ "email": payload.email, "nombre": insert_row.get::<Option<String>, _>("nombre") });
            return Ok(Json(ResponseGeneric { status: "success".to_string(), message: None, user: Some(user_json) }));
        }
    }

    if action == "pago" {
        let p: PagoPayload = serde_urlencoded::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;
        sqlx::query("INSERT INTO historial_crm (nombre, direccion, metodo_pago, referencia, productos_comprados, total_pagado) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(&p.nombre).bind(&p.direccion).bind(&p.pago).bind(&p.referencia).bind(&p.productos_json).bind(p.total_num)
            .execute(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok(Json(ResponseGeneric { status: "success".to_string(), message: None, user: None }));
    }

    Err(StatusCode::BAD_REQUEST)
                                            }
