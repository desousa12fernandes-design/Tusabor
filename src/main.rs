// 1. Creamos un Enum para capturar dinámicamente el JSON correcto
#[derive(Deserialize)]
#[serde(untagged)] // Intenta parsear uno por uno sin exigir etiquetas extras
enum PostPayload {
    Login(LoginPayload),
    Pago(PagoPayload),
}

// 2. Modificamos el manejador para usar Axum Json directamente
async fn api_handler_post(
    Query(params): Query<Params>,
    State(state): State<AppState>,
    Json(body): Json<PostPayload>, // <-- CORRECCIÓN: Axum ahora valida el JSON automáticamente
) -> Result<Json<ResponseGeneric>, StatusCode> {
    let action = params.action.as_deref().unwrap_or("");

    if action == "login" {
        // Extraemos los datos del enum de forma segura
        let payload = match body {
            PostPayload::Login(p) => p,
            _ => return Err(StatusCode::BAD_REQUEST), // Si mandaron datos de pago a login
        };

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
        let p = match body {
            PostPayload::Pago(p) => p,
            _ => return Err(StatusCode::BAD_REQUEST),
        };

        sqlx::query("INSERT INTO historial_crm (nombre, direccion, metodo_pago, referencia, productos_comprados, total_pagado) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(&p.nombre).bind(&p.direccion).bind(&p.pago).bind(&p.referencia).bind(&p.productos_json).bind(p.total_num)
            .execute(&state.pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok(Json(ResponseGeneric { status: "success".to_string(), message: None, user: None }));
    }

    Err(StatusCode::BAD_REQUEST)
}

