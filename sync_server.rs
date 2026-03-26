use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Interaction {
    pub user_id: String,
    pub lemma: String,
    pub ts: i64,
    pub clicked: i32,
}

#[derive(Debug, Deserialize)]
pub struct PushRequest {
    pub user_id: String,
    pub interactions: Vec<Interaction>,
}

#[derive(Debug, Serialize)]
pub struct PullResponse {
    pub interactions: Vec<Interaction>,
}


fn init_db() -> Result<Connection, String> {
    let conn = Connection::open("sync_server.db").map_err(|e| e.to_string())?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS interactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            lemma TEXT NOT NULL,
            ts INTEGER NOT NULL,
            clicked INTEGER NOT NULL,
            UNIQUE(user_id, lemma, ts)
        )",
        [],
    )
        .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_user_ts ON interactions(user_id, ts)",
        [],
    )
        .ok();

    Ok(conn)
}

async fn push_data(
    State(db): State<Arc<Mutex<Connection>>>,
    Json(payload): Json<PushRequest>,
) -> Result<StatusCode, StatusCode> {
    info!("Pushing {} records for user {}", payload.interactions.len(), payload.user_id);

    let db = db.lock().await;
    let tx = db.unchecked_transaction().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for item in payload.interactions {
        let result = tx.execute(
            "INSERT OR IGNORE INTO interactions (user_id, lemma, ts, clicked) VALUES (?1, ?2, ?3, ?4)",
            params![item.user_id, item.lemma, item.ts, item.clicked],
        );

        if result.is_err() {
            eprintln!("Failed to insert record: {:?}", item);
        }
    }

    tx.commit().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

async fn pull_data(
    State(db): State<Arc<Mutex<Connection>>>,
    Json(payload): Json<Interaction>,
) -> Result<Json<PullResponse>, StatusCode> {
    info!("Pulling data for user {} after ts {}", payload.user_id, payload.ts);

    let db = db.lock().await;

    let mut stmt = db
        .prepare(
            "SELECT user_id, lemma, ts, clicked FROM interactions
             WHERE user_id = ?1 AND ts > ?2
             ORDER BY ts ASC"
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = stmt
        .query_map(params![payload.user_id, payload.ts], |row| {
            Ok(Interaction {
                user_id: row.get(0)?,
                lemma: row.get(1)?,
                ts: row.get(2)?,
                clicked: row.get(3)?,
            })
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut interactions = Vec::new();
    for row in rows.flatten() {
        interactions.push(row);
    }

    Ok(Json(PullResponse { interactions }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db = init_db().expect("Failed to init database");
    let db_state = Arc::new(Mutex::new(db));

    let app = Router::new()
        .route("/push", post(push_data))
        .route("/pull", post(pull_data))
        .with_state(db_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    info!("Sync server listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
