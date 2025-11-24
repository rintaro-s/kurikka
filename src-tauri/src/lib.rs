use parking_lot::Mutex;
use serde::Serialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size};

mod game;
mod input_hook;

use game::{GameState, Unit, UnitType};
use input_hook::InputCounter;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let game_state = Arc::new(Mutex::new(GameState::new()));
    let input_counter = Arc::new(Mutex::new(InputCounter::new()));

    // ゲームループ用のステート
    let game_state_loop = Arc::clone(&game_state);
    let input_counter_clone = Arc::clone(&input_counter);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(game_state)
        .manage(input_counter)
        .invoke_handler(tauri::generate_handler![
            get_game_state,
            purchase_upgrade,
            reset_stage
        ])
        .setup(move |app| {
            let app_handle = app.handle().clone();

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
                    // Position lower (closer to taskbar)
                    let fallback_offset = 100_i32;
                    y -= fallback_offset;

                    println!("[widget] monitor pos={:?} size={:?} x={} y={} width={} height={}", monitor_pos, monitor_size, x, y, width, height);

                    let _ = widget_window.set_size(Size::Physical(PhysicalSize::new(width, height)));
                    let _ = widget_window
                        .set_position(Position::Physical(PhysicalPosition::new(x, y)));
                } else {
                    println!("[widget] current_monitor not found; fallback to primary_monitor");
                    if let Ok(Some(monitor)) = app_handle.primary_monitor() {
                        let monitor_pos = monitor.position();
                        let monitor_size = monitor.size();
                        let width = monitor_size.width;
                        let height = 80_u32;
                        let x = monitor_pos.x;
                        let mut y = monitor_pos.y + monitor_size.height as i32 - height as i32;
                        y -= 100_i32;
                        let _ = widget_window.set_size(Size::Physical(PhysicalSize::new(width, height)));
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
