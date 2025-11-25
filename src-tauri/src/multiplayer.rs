use crate::game::PlayerProgressData;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub player_id: String,
    pub player_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub player_id: String,
    pub player_name: String,
    pub progress: PlayerProgressData,
    pub last_update: i64,
}

pub struct MultiplayerClient {
    server_url: Arc<Mutex<String>>,
    player_info: Arc<Mutex<Option<PlayerInfo>>>,
    last_remote_update: Arc<Mutex<Option<i64>>>,
    http_client: reqwest::Client,
}

impl MultiplayerClient {
    pub fn new() -> Self {
        Self {
            server_url: Arc::new(Mutex::new(String::new())),
            player_info: Arc::new(Mutex::new(None)),
            last_remote_update: Arc::new(Mutex::new(None)),
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

    pub async fn register_player(&self, player_name: String) -> Result<RegisterResult, String> {
        let server_url = self.get_server_url();
        if server_url.is_empty() {
            return Err("No server URL configured".to_string());
        }

        #[derive(Serialize)]
        struct RegisterRequest {
            player_name: String,
        }

        let url = format!("{}/api/player/register", server_url);
        let response = self
            .http_client
            .post(&url)
            .json(&RegisterRequest {
                player_name: player_name.clone(),
            })
            .send()
            .await
            .map_err(|e| format!("Failed to register player: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Server returned error: {}", response.status()));
        }

        let register_response: RegisterResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        *self.player_info.lock() = Some(PlayerInfo {
            player_id: register_response.player_id.clone(),
            player_name,
        });
        *self.last_remote_update.lock() = Some(register_response.last_update);

        Ok(RegisterResult {
            player_id: register_response.player_id,
            player_name: register_response.player_name,
            message: register_response.message,
            progress: register_response.progress,
            last_update: register_response.last_update,
        })
    }

    pub async fn sync_progress(
        &self,
        progress: &PlayerProgressData,
    ) -> Result<PlayerProfile, String> {
        let info = self
            .player_info
            .lock()
            .clone()
            .ok_or("Not registered to server")?;
        let server_url = self.get_server_url();
        if server_url.is_empty() {
            return Err("No server URL configured".to_string());
        }

        #[derive(Serialize)]
        struct SyncRequest<'a> {
            progress: &'a PlayerProgressData,
        }

        let url = format!("{}/api/player/{}/sync", server_url, info.player_id);
        let response = self
            .http_client
            .post(&url)
            .json(&SyncRequest { progress })
            .send()
            .await
            .map_err(|e| format!("Failed to sync state: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Server returned error: {}", response.status()));
        }

        let profile: PlayerProfile = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        *self.last_remote_update.lock() = Some(profile.last_update);
        Ok(profile)
    }

    pub async fn get_all_players(&self) -> Result<Vec<serde_json::Value>, String> {
        let server_url = self.get_server_url();
        if server_url.is_empty() {
            return Err("No server URL configured".to_string());
        }

        let url = format!("{}/api/players", server_url);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to get players: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Server returned error: {}", response.status()));
        }

        let players: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(players)
    }

    pub async fn fetch_profile(&self) -> Result<PlayerProfile, String> {
        let info = self
            .player_info
            .lock()
            .clone()
            .ok_or("Not registered to server")?;
        let server_url = self.get_server_url();
        if server_url.is_empty() {
            return Err("No server URL configured".to_string());
        }

        let url = format!("{}/api/player/{}", server_url, info.player_id);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch profile: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Server returned error: {}", response.status()));
        }

        let profile: PlayerProfile = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        Ok(profile)
    }

    pub async fn health_check(&self) -> Result<String, String> {
        let server_url = self.get_server_url();
        if server_url.is_empty() {
            return Err("No server URL configured".to_string());
        }

        let url = format!("{}/health", server_url);
        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to reach server: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Server returned error: {}", response.status()));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(json.to_string())
    }

    pub fn mark_remote_update(&self, timestamp: i64) -> bool {
        let mut guard = self.last_remote_update.lock();
        if guard.map_or(true, |current| timestamp > current) {
            *guard = Some(timestamp);
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Deserialize)]
struct RegisterResponse {
    player_id: String,
    player_name: String,
    message: String,
    progress: PlayerProgressData,
    last_update: i64,
}

#[derive(Debug, Clone)]
pub struct RegisterResult {
    pub player_id: String,
    pub player_name: String,
    pub message: String,
    pub progress: PlayerProgressData,
    pub last_update: i64,
}
