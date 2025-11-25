use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub player_id: String,
    pub player_name: String,
    pub stage: u32,
    pub last_update: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameUpdate {
    pub player_id: String,
    pub player_units: Vec<UnitData>,
    pub enemy_units: Vec<UnitData>,
    pub player_base_hp: f32,
    pub enemy_base_hp: f32,
    pub coins: u32,
    pub stage: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitData {
    pub id: u32,
    pub unit_type: String,
    pub position: f32,
    pub hp: f32,
}

type PlayerStore = Arc<Mutex<HashMap<String, PlayerState>>>;

// プレイヤー登録
async fn register_player(
    data: web::Json<serde_json::Value>,
    store: web::Data<PlayerStore>,
) -> impl Responder {
    let player_id = Uuid::new_v4().to_string();
    let player_name = data.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Anonymous")
        .to_string();
    
    let player_state = PlayerState {
        player_id: player_id.clone(),
        player_name,
        stage: 1,
        last_update: chrono::Utc::now().timestamp(),
    };

    let mut players = store.lock().unwrap();
    players.insert(player_id.clone(), player_state.clone());

    HttpResponse::Ok().json(player_state)
}

// プレイヤー状態更新
async fn update_player(
    player_id: web::Path<String>,
    data: web::Json<GameUpdate>,
    store: web::Data<PlayerStore>,
) -> impl Responder {
    let mut players = store.lock().unwrap();
    
    if let Some(player) = players.get_mut(player_id.as_str()) {
        player.stage = data.stage;
        player.last_update = chrono::Utc::now().timestamp();
        HttpResponse::Ok().json(player.clone())
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "error": "Player not found"
        }))
    }
}

// プレイヤー一覧取得
async fn list_players(store: web::Data<PlayerStore>) -> impl Responder {
    let players = store.lock().unwrap();
    let player_list: Vec<PlayerState> = players.values().cloned().collect();
    HttpResponse::Ok().json(player_list)
}

// プレイヤー情報取得
async fn get_player(
    player_id: web::Path<String>,
    store: web::Data<PlayerStore>,
) -> impl Responder {
    let players = store.lock().unwrap();
    
    if let Some(player) = players.get(player_id.as_str()) {
        HttpResponse::Ok().json(player.clone())
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "error": "Player not found"
        }))
    }
}

// ヘルスチェック
async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().timestamp()
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting Clicker Clicker Clicker Multiplayer Server...");
    println!("Server will listen on http://0.0.0.0:8080");
    
    let player_store = Arc::new(Mutex::new(HashMap::<String, PlayerState>::new()));

    HttpServer::new(move || {
        let cors = Cors::permissive();
        
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(player_store.clone()))
            .route("/health", web::get().to(health))
            .route("/api/player/register", web::post().to(register_player))
            .route("/api/player/{id}", web::get().to(get_player))
            .route("/api/player/{id}/update", web::post().to(update_player))
            .route("/api/players", web::get().to(list_players))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
