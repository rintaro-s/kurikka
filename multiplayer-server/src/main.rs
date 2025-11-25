use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct UpgradesProgress {
    small_attack: u32,
    medium_attack: u32,
    large_attack: u32,
    small_hp: u32,
    medium_hp: u32,
    large_hp: u32,
    small_speed: u32,
    medium_speed: u32,
    large_speed: u32,
    coin_rate: u32,
    base_hp: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlayerProgress {
    stage: u32,
    coins: u32,
    upgrades: UpgradesProgress,
    max_player_base_hp: f32,
    max_enemy_base_hp: f32,
}

impl Default for PlayerProgress {
    fn default() -> Self {
        Self {
            stage: 1,
            coins: 0,
            upgrades: UpgradesProgress::default(),
            max_player_base_hp: 1000.0,
            max_enemy_base_hp: 500.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlayerProfile {
    player_id: String,
    player_name: String,
    progress: PlayerProgress,
    last_update: i64,
}

impl PlayerProfile {
    fn new(player_name: &str) -> Self {
        Self {
            player_id: Uuid::new_v4().to_string(),
            player_name: player_name.to_string(),
            progress: PlayerProgress::default(),
            last_update: Utc::now().timestamp(),
        }
    }
}

#[derive(Default)]
struct ServerState {
    players: HashMap<String, PlayerProfile>,
    name_index: HashMap<String, String>, // lower_name -> player_id
}

type PlayerStore = Arc<Mutex<ServerState>>;

fn data_dir() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    dir.push("data");
    dir.push("players");
    dir
}

fn load_profiles() -> ServerState {
    let mut state = ServerState::default();
    let dir = data_dir();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(contents) = fs::read_to_string(entry.path()) {
                if let Ok(profile) = serde_json::from_str::<PlayerProfile>(&contents) {
                    let lower = profile.player_name.to_lowercase();
                    state.name_index.insert(lower, profile.player_id.clone());
                    state.players.insert(profile.player_id.clone(), profile);
                }
            }
        }
    }
    state
}

fn save_profile(profile: &PlayerProfile) -> std::io::Result<()> {
    let dir = data_dir();
    fs::create_dir_all(&dir)?;
    let mut path = dir;
    path.push(format!("{}.json", profile.player_id));
    let json = serde_json::to_string_pretty(profile).unwrap_or_default();
    fs::write(path, json)
}

#[derive(Debug, Deserialize)]
struct RegisterRequest {
    player_name: String,
}

#[derive(Debug, Serialize)]
struct RegisterResponse {
    player_id: String,
    player_name: String,
    message: String,
    progress: PlayerProgress,
    last_update: i64,
}

fn normalize_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn build_register_response(profile: &PlayerProfile, message: &str) -> RegisterResponse {
    RegisterResponse {
        player_id: profile.player_id.clone(),
        player_name: profile.player_name.clone(),
        message: message.to_string(),
        progress: profile.progress.clone(),
        last_update: profile.last_update,
    }
}

async fn register_player(
    data: web::Json<RegisterRequest>,
    store: web::Data<PlayerStore>,
) -> impl Responder {
    let requested_name = data.player_name.trim();
    if requested_name.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Player name is required",
        }));
    }

    let lower_name = normalize_name(requested_name);
    let mut state = store.lock().unwrap();

    if let Some(existing_id) = state.name_index.get(&lower_name).cloned() {
        if let Some(profile) = state.players.get(&existing_id).cloned() {
            return HttpResponse::Ok().json(build_register_response(
                &profile,
                "Welcome back! Progress loaded.",
            ));
        }
    }

    let profile = PlayerProfile::new(requested_name);
    state
        .name_index
        .insert(lower_name, profile.player_id.clone());
    state
        .players
        .insert(profile.player_id.clone(), profile.clone());
    let profile_clone = profile.clone();
    drop(state);

    if let Err(err) = save_profile(&profile_clone) {
        eprintln!("Failed to save profile: {}", err);
    }

    HttpResponse::Ok().json(build_register_response(&profile_clone, "Account created!"))
}

#[derive(Debug, Deserialize)]
struct SyncRequest {
    progress: PlayerProgress,
}

async fn sync_player(
    player_id: web::Path<String>,
    data: web::Json<SyncRequest>,
    store: web::Data<PlayerStore>,
) -> impl Responder {
    let mut state = store.lock().unwrap();
    if let Some(profile) = state.players.get_mut(player_id.as_str()) {
        profile.progress = data.progress.clone();
        profile.last_update = Utc::now().timestamp();
        let profile_clone = profile.clone();
        drop(state);

        if let Err(err) = save_profile(&profile_clone) {
            eprintln!("Failed to save profile: {}", err);
        }

        return HttpResponse::Ok().json(profile_clone);
    }

    HttpResponse::NotFound().json(serde_json::json!({ "error": "Player not found" }))
}

async fn get_player(player_id: web::Path<String>, store: web::Data<PlayerStore>) -> impl Responder {
    let state = store.lock().unwrap();
    if let Some(profile) = state.players.get(player_id.as_str()) {
        HttpResponse::Ok().json(profile.clone())
    } else {
        HttpResponse::NotFound().json(serde_json::json!({ "error": "Player not found" }))
    }
}

#[derive(Serialize)]
struct PlayerSummary {
    player_id: String,
    player_name: String,
    stage: u32,
    last_update: i64,
}

async fn list_players(store: web::Data<PlayerStore>) -> impl Responder {
    let state = store.lock().unwrap();
    let players: Vec<PlayerSummary> = state
        .players
        .values()
        .map(|profile| PlayerSummary {
            player_id: profile.player_id.clone(),
            player_name: profile.player_name.clone(),
            stage: profile.progress.stage,
            last_update: profile.last_update,
        })
        .collect();
    HttpResponse::Ok().json(players)
}

async fn health(store: web::Data<PlayerStore>) -> impl Responder {
    let state = store.lock().unwrap();
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "timestamp": Utc::now().timestamp(),
        "player_count": state.players.len(),
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting Clicker Clicker Clicker Multiplayer Server...");
    println!("Server will listen on http://0.0.0.0:8080");

    let initial_state = load_profiles();
    println!("Loaded {} player profiles", initial_state.players.len());
    let player_store = Arc::new(Mutex::new(initial_state));

    HttpServer::new(move || {
        let cors = Cors::permissive();

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(player_store.clone()))
            .route("/health", web::get().to(health))
            .route("/api/player/register", web::post().to(register_player))
            .route("/api/player/{id}", web::get().to(get_player))
            .route("/api/player/{id}/sync", web::post().to(sync_player))
            .route("/api/players", web::get().to(list_players))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
