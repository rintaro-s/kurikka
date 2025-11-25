use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, WindowEvent};

mod config;
mod game;
mod input_hook;
mod multiplayer;

use config::AppConfig;
use game::{AutoBuyConfig, GameState, Unit, UnitType};
use input_hook::InputCounter;
use multiplayer::MultiplayerClient;

#[derive(Clone, Serialize)]
struct GameStateUpdate {
    player_units: Vec<Unit>,
    enemy_units: Vec<Unit>,
    player_base_hp: f32,
    enemy_base_hp: f32,
    coins: u32,
    stage: u32,
    click_count: u32,
    type_count: u32,
    upgrades: game::Upgrades,
}

#[derive(Clone, Serialize)]
struct RegisterCommandResponse {
    player_id: String,
    player_name: String,
    message: String,
    last_update: i64,
    stage: u32,
    coins: u32,
}

#[tauri::command]
fn get_game_state(state: tauri::State<Arc<Mutex<GameState>>>) -> GameStateUpdate {
    let game = state.lock();
    GameStateUpdate {
        player_units: game.player_units.clone(),
        enemy_units: game.enemy_units.clone(),
        player_base_hp: game.player_base_hp,
        enemy_base_hp: game.enemy_base_hp,
        coins: game.coins,
        stage: game.stage,
        click_count: game.click_count,
        type_count: game.type_count,
        upgrades: game.upgrades.clone(),
    }
}

#[tauri::command]
fn purchase_upgrade(
    state: tauri::State<Arc<Mutex<GameState>>>,
    upgrade_type: String,
    unit_type: String,
) -> Result<bool, String> {
    let mut game = state.lock();
    game.purchase_upgrade(&upgrade_type, &unit_type)
}

#[tauri::command]
fn reset_stage(state: tauri::State<Arc<Mutex<GameState>>>) {
    let mut game = state.lock();
    game.reset_current_stage();
}

#[tauri::command]
fn get_config() -> AppConfig {
    AppConfig::load()
}

#[tauri::command]
fn save_config(
    config: AppConfig,
    mp_client: tauri::State<'_, Arc<MultiplayerClient>>,
) -> Result<(), String> {
    // サーバーURLを更新
    mp_client.set_server_url(config.multiplayer_server_url.clone());
    config.save()
}

#[tauri::command]
fn apply_widget_config(app: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    if let Some(widget_window) = app.get_webview_window("widget") {
        if let Ok(Some(monitor)) = widget_window.current_monitor() {
            let monitor_pos = monitor.position();
            let monitor_size = monitor.size();
            let width = monitor_size.width;
            let height = 80_u32;
            let x = monitor_pos.x;
            let mut y = monitor_pos.y + monitor_size.height as i32 - height as i32;
            y -= config.widget_y_offset;

            let _ = widget_window.set_size(Size::Physical(PhysicalSize::new(width, height)));
            let _ = widget_window.set_position(Position::Physical(PhysicalPosition::new(x, y)));

            Ok(())
        } else {
            Err("Failed to get monitor info".to_string())
        }
    } else {
        Err("Widget window not found".to_string())
    }
}

#[tauri::command]
async fn mp_register_player(
    mp_client: tauri::State<'_, Arc<MultiplayerClient>>,
    game_state: tauri::State<'_, Arc<Mutex<GameState>>>,
    player_name: String,
) -> Result<RegisterCommandResponse, String> {
    let register_result = mp_client.register_player(player_name).await?;

    {
        let mut game = game_state.lock();
        game.import_progress(&register_result.progress);
    }

    let mut config = AppConfig::load();
    config.multiplayer_player_name = register_result.player_name.clone();
    config.multiplayer_player_id = register_result.player_id.clone();
    let _ = config.save();

    Ok(RegisterCommandResponse {
        player_id: register_result.player_id,
        player_name: register_result.player_name,
        message: register_result.message,
        last_update: register_result.last_update,
        stage: register_result.progress.stage,
        coins: register_result.progress.coins,
    })
}

#[tauri::command]
async fn mp_update_state(
    mp_client: tauri::State<'_, Arc<MultiplayerClient>>,
    game_state: tauri::State<'_, Arc<Mutex<GameState>>>,
) -> Result<(), String> {
    let progress = {
        let game = game_state.lock();
        game.export_progress()
    };

    let _ = mp_client.sync_progress(&progress).await?;
    Ok(())
}

#[tauri::command]
async fn mp_get_players(
    mp_client: tauri::State<'_, Arc<MultiplayerClient>>,
) -> Result<Vec<serde_json::Value>, String> {
    mp_client.get_all_players().await
}

#[tauri::command]
async fn mp_pull_state(
    mp_client: tauri::State<'_, Arc<MultiplayerClient>>,
    game_state: tauri::State<'_, Arc<Mutex<GameState>>>,
) -> Result<bool, String> {
    let profile = mp_client.fetch_profile().await?;
    if mp_client.mark_remote_update(profile.last_update) {
        let mut game = game_state.lock();
        game.import_progress(&profile.progress);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
async fn mp_health_check(
    mp_client: tauri::State<'_, Arc<MultiplayerClient>>,
) -> Result<String, String> {
    mp_client.health_check().await
}

#[tauri::command]
fn mp_is_connected(mp_client: tauri::State<'_, Arc<MultiplayerClient>>) -> bool {
    mp_client.is_connected()
}

#[tauri::command]
fn start_auto_buy(
    state: tauri::State<Arc<Mutex<GameState>>>,
    upgrade_type: String,
    unit_type: String,
    duration_seconds: f32,
) -> Result<(), String> {
    let mut game = state.lock();

    // 自動購入のコスト: 5000コイン
    let auto_buy_cost = 5000;
    if game.coins < auto_buy_cost {
        return Err("Not enough coins for auto-buy".to_string());
    }

    game.coins -= auto_buy_cost;
    game.auto_buy = AutoBuyConfig {
        enabled: true,
        upgrade_type,
        unit_type,
        remaining_time: duration_seconds,
    };

    Ok(())
}

#[tauri::command]
fn get_auto_buy(state: tauri::State<Arc<Mutex<GameState>>>) -> AutoBuyConfig {
    let game = state.lock();
    game.auto_buy.clone()
}

#[tauri::command]
fn stop_auto_buy(state: tauri::State<Arc<Mutex<GameState>>>) -> Result<(), String> {
    let mut game = state.lock();
    game.auto_buy.enabled = false;
    game.auto_buy.remaining_time = 0.0;
    Ok(())
}

#[tauri::command]
fn exit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let game_state = Arc::new(Mutex::new(GameState::new()));
    let input_counter = Arc::new(Mutex::new(InputCounter::new()));
    let mp_client = Arc::new(MultiplayerClient::new());

    // 設定からサーバーURLをロード
    let config = AppConfig::load();
    if !config.multiplayer_server_url.is_empty() {
        mp_client.set_server_url(config.multiplayer_server_url);
    }

    // ゲームループ用のステート
    let game_state_loop = Arc::clone(&game_state);
    let input_counter_clone = Arc::clone(&input_counter);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                if window.label() == "main" {
                    window.app_handle().exit(0);
                }
            }
        })
        .manage(game_state)
        .manage(input_counter)
        .manage(mp_client)
        .invoke_handler(tauri::generate_handler![
            get_game_state,
            purchase_upgrade,
            reset_stage,
            get_config,
            save_config,
            apply_widget_config,
            mp_register_player,
            mp_update_state,
            mp_get_players,
            mp_pull_state,
            mp_health_check,
            mp_is_connected,
            start_auto_buy,
            get_auto_buy,
            stop_auto_buy,
            exit_app
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let config = AppConfig::load();

            if let Some(widget_window) = app_handle.get_webview_window("widget") {
                let _ = widget_window.set_always_on_top(true);
                let _ = widget_window.set_skip_taskbar(true);
                let _ = widget_window.set_ignore_cursor_events(true);
                let _ = widget_window.set_decorations(false);

                if let Ok(Some(monitor)) = widget_window.current_monitor() {
                    let monitor_pos = monitor.position();
                    let monitor_size = monitor.size();
                    let width = monitor_size.width;
                    let height = 80_u32;
                    let x = monitor_pos.x;
                    // Try to place the widget above the bottom edge; subtract a bit to avoid overlapping a taskbar.
                    let mut y = monitor_pos.y + monitor_size.height as i32 - height as i32;
                    // Position using config offset
                    let fallback_offset = config.widget_y_offset;
                    y -= fallback_offset;

                    println!(
                        "[widget] monitor pos={:?} size={:?} x={} y={} width={} height={}",
                        monitor_pos, monitor_size, x, y, width, height
                    );

                    let _ =
                        widget_window.set_size(Size::Physical(PhysicalSize::new(width, height)));
                    let _ =
                        widget_window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
                } else {
                    println!("[widget] current_monitor not found; fallback to primary_monitor");
                    if let Ok(Some(monitor)) = app_handle.primary_monitor() {
                        let monitor_pos = monitor.position();
                        let monitor_size = monitor.size();
                        let width = monitor_size.width;
                        let height = 80_u32;
                        let x = monitor_pos.x;
                        let mut y = monitor_pos.y + monitor_size.height as i32 - height as i32;
                        y -= config.widget_y_offset;
                        let _ = widget_window
                            .set_size(Size::Physical(PhysicalSize::new(width, height)));
                        let _ = widget_window
                            .set_position(Position::Physical(PhysicalPosition::new(x, y)));
                    }
                }
            }

            // グローバル入力フックの開始
            let input_counter_hook = Arc::clone(&input_counter_clone);
            std::thread::spawn(move || {
                input_hook::start_input_hook(input_counter_hook);
            });

            // ゲームループ
            std::thread::spawn(move || {
                let mut last_update = Instant::now();
                let mut last_time_unit_spawn = Instant::now();

                loop {
                    std::thread::sleep(Duration::from_millis(16)); // 約60 FPS

                    let delta = last_update.elapsed().as_secs_f32();
                    last_update = Instant::now();

                    // 入力カウントの取得とユニット生成
                    let (clicks, types) = {
                        let mut counter = input_counter_clone.lock();
                        counter.consume_inputs()
                    };

                    let mut game = game_state_loop.lock();

                    // ユニット生成
                    for _ in 0..types {
                        game.spawn_unit(UnitType::Small);
                    }
                    for _ in 0..clicks {
                        game.spawn_unit(UnitType::Medium);
                    }

                    // 1分ごとの強力ユニット生成
                    if last_time_unit_spawn.elapsed().as_secs() >= 60 {
                        game.spawn_unit(UnitType::Large);
                        last_time_unit_spawn = Instant::now();
                    }

                    // ゲーム更新
                    game.update(delta);

                    // フロントエンドに状態を送信
                    let _ = app_handle.emit(
                        "game-update",
                        GameStateUpdate {
                            player_units: game.player_units.clone(),
                            enemy_units: game.enemy_units.clone(),
                            player_base_hp: game.player_base_hp,
                            enemy_base_hp: game.enemy_base_hp,
                            coins: game.coins,
                            stage: game.stage,
                            click_count: game.click_count,
                            type_count: game.type_count,
                            upgrades: game.upgrades.clone(),
                        },
                    );
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
