use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub player_id: String,
    pub player_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameUpdate {
    pub stage: i32,
    pub coins: i32,
    pub player_units_count: usize,
    pub enemy_units_count: usize,
}

pub struct MultiplayerClient {
    server_url: Arc<Mutex<String>>,
    player_info: Arc<Mutex<Option<PlayerInfo>>>,
    http_client: reqwest::Client,
}

impl MultiplayerClient {
    pub fn new() -> Self {
        Self {
            server_url: Arc::new(Mutex::new(String::new())),
            player_info: Arc::new(Mutex::new(None)),
            http_client: reqwest::Client::new(),
        }
    }

    pub fn set_server_url(&self, url: String) {
        *self.server_url.lock() = url;
    }

    pub fn get_server_url(&self) -> String {
        self.server_url.lock().clone()
    }

    pub fn is_connected(&self) -> bool {
        !self.get_server_url().is_empty() && self.player_info.lock().is_some()
    }

    pub async fn register_player(&self, player_name: String) -> Result<String, String> {
        let server_url = self.get_server_url();
        if server_url.is_empty() {
            return Err("No server URL configured".to_string());
        }

        #[derive(Serialize)]
        struct RegisterRequest {
            player_name: String,
        }

        #[derive(Deserialize)]
        struct RegisterResponse {
            player_id: String,
            message: String,
        }

        let url = format!("{}/api/player/register", server_url);
        let response = self.http_client
            .post(&url)
            .json(&RegisterRequest { player_name: player_name.clone() })
            .send()
            .await
            .map_err(|e| format!("Failed to register player: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Server returned error: {}", response.status()));
        }

        let register_response: RegisterResponse = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        *self.player_info.lock() = Some(PlayerInfo {
            player_id: register_response.player_id.clone(),
            player_name,
        });

        Ok(register_response.player_id)
    }

    pub async fn update_game_state(&self, update: GameUpdate) -> Result<(), String> {
        let player_info = self.player_info.lock().clone();
        let player_info = player_info.ok_or("Not registered to server")?;

        let server_url = self.get_server_url();
        if server_url.is_empty() {
            return Err("No server URL configured".to_string());
        }

        let url = format!("{}/api/player/{}/update", server_url, player_info.player_id);
        let response = self.http_client
            .post(&url)
            .json(&update)
            .send()
            .await
            .map_err(|e| format!("Failed to update state: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Server returned error: {}", response.status()));
        }

        Ok(())
    }

    pub async fn get_all_players(&self) -> Result<Vec<serde_json::Value>, String> {
        let server_url = self.get_server_url();
        if server_url.is_empty() {
            return Err("No server URL configured".to_string());
        }

        let url = format!("{}/api/players", server_url);
        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to get players: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Server returned error: {}", response.status()));
        }

        let players: Vec<serde_json::Value> = response.json().await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(players)
    }
}
