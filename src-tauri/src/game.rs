use directories::ProjectDirs;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub enum UnitType {
    Small,
    Medium,
    Large,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id: u32,
    pub unit_type: UnitType,
    pub position: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub attack: f32,
    pub speed: f32,
    pub is_player: bool,
    pub target_id: Option<u32>,
    #[serde(default)]
    pub knockback_velocity: f32,
    #[serde(default)]
    pub knockback_time: f32,
    #[serde(default)]
    pub knockback_total: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Upgrades {
    // 攻撃力アップグレード（％）
    pub small_attack: u32,
    pub medium_attack: u32,
    pub large_attack: u32,
    // 体力アップグレード（％）
    pub small_hp: u32,
    pub medium_hp: u32,
    pub large_hp: u32,
    // 速度アップグレード（％）
    pub small_speed: u32,
    pub medium_speed: u32,
    pub large_speed: u32,
    // コイン獲得率（％）
    pub coin_rate: u32,
    // 基地体力
    pub base_hp: u32,
}

impl Upgrades {
    pub fn new() -> Self {
        Self {
            small_attack: 0,
            medium_attack: 0,
            large_attack: 0,
            small_hp: 0,
            medium_hp: 0,
            large_hp: 0,
            small_speed: 0,
            medium_speed: 0,
            large_speed: 0,
            coin_rate: 0,
            base_hp: 0,
        }
    }

    pub fn get_cost(&self, upgrade_type: &str, unit_type: &str) -> u32 {
        let level = match (upgrade_type, unit_type) {
            ("attack", "small") => self.small_attack,
            ("attack", "medium") => self.medium_attack,
            ("attack", "large") => self.large_attack,
            ("hp", "small") => self.small_hp,
            ("hp", "medium") => self.medium_hp,
            ("hp", "large") => self.large_hp,
            ("speed", "small") => self.small_speed,
            ("speed", "medium") => self.medium_speed,
            ("speed", "large") => self.large_speed,
            ("coin_rate", _) => self.coin_rate,
            ("base_hp", _) => self.base_hp,
            _ => 0,
        };
        // 初期値3000、1.2倍ずつ増加
        (3000.0 * 1.2_f32.powi(level as i32)) as u32
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AutoBuyConfig {
    pub enabled: bool,
    pub upgrade_type: String, // "attack", "hp", "speed", "coin_rate", "base_hp"
    pub unit_type: String,    // "small", "medium", "large", ""
    #[serde(default)]
    pub remaining_time: f32, // 残り時間（秒）
}

impl Default for AutoBuyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            upgrade_type: String::new(),
            unit_type: String::new(),
            remaining_time: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GameState {
    pub player_units: Vec<Unit>,
    pub enemy_units: Vec<Unit>,
    pub player_base_hp: f32,
    pub enemy_base_hp: f32,
    pub max_player_base_hp: f32,
    pub max_enemy_base_hp: f32,
    pub coins: u32,
    pub stage: u32,
    pub click_count: u32,
    pub type_count: u32,
    pub upgrades: Upgrades,
    #[serde(default)]
    pub auto_buy: AutoBuyConfig,
    next_unit_id: u32,
    enemy_spawn_timer: f32,
    stage_clear: bool,
    #[serde(skip)]
    save_timer: f32,
}

impl GameState {
    pub fn new() -> Self {
        if let Some(mut loaded) = Self::load_from_disk() {
            loaded.save_timer = 0.0;
            loaded.next_unit_id = loaded
                .player_units
                .iter()
                .chain(loaded.enemy_units.iter())
                .map(|u| u.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            return loaded;
        }

        let state = Self::fresh();
        state.persist_state();
        state
    }

    fn fresh() -> Self {
        Self {
            player_units: Vec::new(),
            enemy_units: Vec::new(),
            player_base_hp: 1000.0,
            enemy_base_hp: 500.0,
            max_player_base_hp: 1000.0,
            max_enemy_base_hp: 500.0,
            coins: 0,
            stage: 1,
            click_count: 0,
            type_count: 0,
            upgrades: Upgrades::new(),
            auto_buy: AutoBuyConfig::default(),
            next_unit_id: 0,
            enemy_spawn_timer: 0.0,
            stage_clear: false,
            save_timer: 0.0,
        }
    }

    fn data_file_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "ClickerClicker", "ClickerClickerClicker")
            .map(|dirs| dirs.data_dir().join("game_state.json"))
    }

    fn load_from_disk() -> Option<Self> {
        let path = Self::data_file_path()?;
        let contents = fs::read_to_string(path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    fn persist_state(&self) {
        if let Some(path) = Self::data_file_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(json) = serde_json::to_string(self) {
                let _ = fs::write(path, json);
            }
        }
    }

    pub fn spawn_unit(&mut self, unit_type: UnitType) {
        let (base_hp, base_attack, base_speed) = match unit_type {
            UnitType::Small => (10.0, 5.0, 100.0),
            UnitType::Medium => (30.0, 15.0, 80.0),
            UnitType::Large => (100.0, 50.0, 60.0),
        };

        // アップグレード適用
        let (hp_bonus, attack_bonus, speed_bonus) = match unit_type {
            UnitType::Small => (
                self.upgrades.small_hp,
                self.upgrades.small_attack,
                self.upgrades.small_speed,
            ),
            UnitType::Medium => (
                self.upgrades.medium_hp,
                self.upgrades.medium_attack,
                self.upgrades.medium_speed,
            ),
            UnitType::Large => (
                self.upgrades.large_hp,
                self.upgrades.large_attack,
                self.upgrades.large_speed,
            ),
        };

        let hp = base_hp * (1.0 + hp_bonus as f32 / 100.0);
        let attack = base_attack * (1.0 + attack_bonus as f32 / 100.0);
        let speed = base_speed * (1.0 + speed_bonus as f32 / 100.0);

        self.player_units.push(Unit {
            id: self.next_unit_id,
            unit_type,
            position: 0.0,
            hp,
            max_hp: hp,
            attack,
            speed,
            is_player: true,
            target_id: None,
            knockback_velocity: 0.0,
            knockback_time: 0.0,
            knockback_total: 0.0,
        });

        self.next_unit_id += 1;

        match unit_type {
            UnitType::Small => self.type_count += 1,
            UnitType::Medium => self.click_count += 1,
            _ => {}
        }
    }

    fn spawn_enemy(&mut self) {
        let mut rng = rand::thread_rng();
        // 1000ステージ想定でなだらかに難易度上昇（対数的スケーリング）
        let stage_multiplier = 1.0 + (self.stage as f32 - 1.0) * 0.05 + ((self.stage as f32).ln() / 10.0) * 0.3;

        let unit_type = if rng.gen_bool(0.7) {
            UnitType::Small
        } else if rng.gen_bool(0.5) {
            UnitType::Medium
        } else {
            UnitType::Large
        };

        let (base_hp, base_attack, base_speed) = match unit_type {
            UnitType::Small => (15.0, 4.0, 90.0),
            UnitType::Medium => (40.0, 12.0, 70.0),
            UnitType::Large => (120.0, 40.0, 50.0),
        };

        self.enemy_units.push(Unit {
            id: self.next_unit_id,
            unit_type,
            position: 1000.0,
            hp: base_hp * stage_multiplier,
            max_hp: base_hp * stage_multiplier,
            attack: base_attack * stage_multiplier,
            speed: base_speed,
            is_player: false,
            target_id: None,
            knockback_velocity: 0.0,
            knockback_time: 0.0,
            knockback_total: 0.0,
        });

        self.next_unit_id += 1;
    }

    pub fn update(&mut self, delta: f32) {
        // 敵のスポーン（なだらかに速度上昇、1000ステージ想定）
        self.enemy_spawn_timer += delta;
        let spawn_interval = (3.0 - (self.stage as f32 * 0.002).min(2.0)).max(1.0);
        if self.enemy_spawn_timer >= spawn_interval {
            self.spawn_enemy();
            self.enemy_spawn_timer = 0.0;
        }

            // ユニットの移動と戦闘
        let mut units_to_remove: Vec<u32> = Vec::new();

        // ターゲット検出とユニット移動
        for i in 0..self.player_units.len() {
            let unit = &mut self.player_units[i];

            // ノックバック処理（吹き飛ばし）
            if unit.knockback_time > 0.0 {
                unit.position += unit.knockback_velocity * delta;
                unit.knockback_time = (unit.knockback_time - delta).max(0.0);
                if unit.knockback_time == 0.0 {
                    unit.knockback_velocity = 0.0;
                    unit.knockback_total = 0.0;
                }
            }

            // ターゲットが有効かチェック
            if let Some(target_id) = unit.target_id {
                if !self.enemy_units.iter().any(|e| e.id == target_id) {
                    unit.target_id = None;
                }
            }

                // ターゲットを探す
            if unit.target_id.is_none() {
                if let Some(enemy) = self
                    .enemy_units
                    .iter()
                    .min_by(|a, b| {
                        (a.position - unit.position)
                            .abs()
                            .partial_cmp(&(b.position - unit.position).abs())
                            .unwrap()
                    })
                {
                    unit.target_id = Some(enemy.id);
                }
            }

            // 移動または攻撃
            if let Some(target_id) = unit.target_id {
                if let Some(enemy) = self.enemy_units.iter_mut().find(|e| e.id == target_id) {
                    let distance = (enemy.position - unit.position).abs();
                    if distance <= 10.0 {
                        // 攻撃範囲内
                        enemy.hp -= unit.attack * delta;
                        if enemy.hp <= 0.0 {
                            units_to_remove.push(enemy.id);
                            let coin_bonus = 1.0 + self.upgrades.coin_rate as f32 / 100.0;
                            // 敵撃破報酬を1～3コインに削減
                            self.coins += (1.0 * coin_bonus).max(1.0) as u32;
                        }
                    } else {
                        // 移動
                        let direction = if enemy.position > unit.position {
                            1.0
                        } else {
                            -1.0
                        };
                        unit.position += direction * unit.speed * delta;
                    }
                }
            } else {
                // ターゲットがいない場合は敵基地へ移動
                if unit.position < 1000.0 {
                    unit.position += unit.speed * delta;
                } else {
                    // 敵基地を攻撃
                    self.enemy_base_hp -= unit.attack * delta;
                }
            }
        }

        // 敵ユニットの移動と戦闘
        for i in 0..self.enemy_units.len() {
            let unit = &mut self.enemy_units[i];

            // 敵のノックバック処理
            if unit.knockback_time > 0.0 {
                unit.position += unit.knockback_velocity * delta;
                unit.knockback_time = (unit.knockback_time - delta).max(0.0);
                if unit.knockback_time == 0.0 {
                    unit.knockback_velocity = 0.0;
                    unit.knockback_total = 0.0;
                }
            }

            if let Some(target_id) = unit.target_id {
                if !self.player_units.iter().any(|e| e.id == target_id) {
                    unit.target_id = None;
                }
            }

            if unit.target_id.is_none() {
                if let Some(player) = self
                    .player_units
                    .iter()
                    .min_by(|a, b| {
                        (a.position - unit.position)
                            .abs()
                            .partial_cmp(&(b.position - unit.position).abs())
                            .unwrap()
                    })
                {
                    unit.target_id = Some(player.id);
                }
            }

            if let Some(target_id) = unit.target_id {
                if let Some(player) = self.player_units.iter_mut().find(|e| e.id == target_id) {
                    let distance = (player.position - unit.position).abs();
                    if distance <= 10.0 {
                        player.hp -= unit.attack * delta;
                        if player.hp <= 0.0 {
                            units_to_remove.push(player.id);
                        }
                    } else {
                        let direction = if player.position > unit.position {
                            1.0
                        } else {
                            -1.0
                        };
                        unit.position += direction * unit.speed * delta;
                    }
                }
            } else {
                if unit.position > 0.0 {
                    unit.position -= unit.speed * delta;
                } else {
                    self.player_base_hp -= unit.attack * delta;
                }
            }
        }

        // 位置の範囲をクランプ
        for unit in &mut self.player_units {
            unit.position = unit.position.max(0.0).min(1000.0);
        }
        for unit in &mut self.enemy_units {
            unit.position = unit.position.max(0.0).min(1000.0);
        }

        // 死亡したユニットを削除
        self.player_units.retain(|u| !units_to_remove.contains(&u.id));
        self.enemy_units.retain(|u| !units_to_remove.contains(&u.id));

        // 勝敗判定
        if self.enemy_base_hp <= 0.0 && !self.stage_clear {
            self.stage_clear = true;
            let _coin_bonus = 1.0 + self.upgrades.coin_rate as f32 / 100.0;
            // ステージクリア報酬を大幅に削減
            self.coins += (20 * (self.stage as u32) / 2).max(10) as u32;
            self.next_stage();
        }

        if self.player_base_hp <= 0.0 {
            self.reset_current_stage();
        }

        // 自動購入処理（時間ベース）
        if self.auto_buy.remaining_time > 0.0 {
            self.auto_buy.remaining_time -= delta;
            if self.auto_buy.remaining_time <= 0.0 {
                self.auto_buy.remaining_time = 0.0;
                self.auto_buy.enabled = false;
            }
            
            if self.auto_buy.enabled && !self.auto_buy.upgrade_type.is_empty() {
                let upgrade_type = self.auto_buy.upgrade_type.clone();
                let unit_type = self.auto_buy.unit_type.clone();
                let cost = self.upgrades.get_cost(&upgrade_type, &unit_type);
                if self.coins >= cost {
                    let _ = self.purchase_upgrade(&upgrade_type, &unit_type);
                }
            }
        } else {
            self.auto_buy.enabled = false;
        }

        // 定期セーブ
        self.save_timer += delta;
        if self.save_timer >= 5.0 {
            self.save_timer = 0.0;
            self.persist_state();
        }
    }

    fn next_stage(&mut self) {
        self.stage += 1;
        self.enemy_base_hp = 500.0 * (1.0 + (self.stage as f32 - 1.0) * 0.5);
        self.max_enemy_base_hp = self.enemy_base_hp;
        self.enemy_units.clear();
        self.enemy_spawn_timer = 0.0;
        self.stage_clear = false;
        self.reposition_player_units();
        self.persist_state();
    }

    pub fn reset_current_stage(&mut self) {
        self.player_units.clear();
        self.enemy_units.clear();
        self.player_base_hp = self.max_player_base_hp;
        self.enemy_base_hp = self.max_enemy_base_hp;
        self.enemy_spawn_timer = 0.0;
        self.stage_clear = false;
        self.persist_state();
    }

    pub fn purchase_upgrade(&mut self, upgrade_type: &str, unit_type: &str) -> Result<bool, String> {
        let cost = self.upgrades.get_cost(upgrade_type, unit_type);

        if self.coins < cost {
            return Err("Not enough coins".to_string());
        }

        self.coins -= cost;

        match (upgrade_type, unit_type) {
            ("attack", "small") => self.upgrades.small_attack += 10,
            ("attack", "medium") => self.upgrades.medium_attack += 10,
            ("attack", "large") => self.upgrades.large_attack += 10,
            ("hp", "small") => self.upgrades.small_hp += 10,
            ("hp", "medium") => self.upgrades.medium_hp += 10,
            ("hp", "large") => self.upgrades.large_hp += 10,
            ("speed", "small") => self.upgrades.small_speed += 10,
            ("speed", "medium") => self.upgrades.medium_speed += 10,
            ("speed", "large") => self.upgrades.large_speed += 10,
            ("coin_rate", _) => self.upgrades.coin_rate += 10,
            ("base_hp", _) => {
                self.upgrades.base_hp += 10;
                self.max_player_base_hp *= 1.1;
                self.player_base_hp = self.max_player_base_hp;
            }
            _ => return Err("Invalid upgrade type".to_string()),
        }

        self.persist_state();
        Ok(true)
    }

    fn reposition_player_units(&mut self) {
        let mut rng = rand::thread_rng();
        // 小ユニットの最大 HP を基準にダメージを計算（アップグレードを考慮）
        let small_base_hp = 10.0 * (1.0 + self.upgrades.small_hp as f32 / 100.0);
        let damage = small_base_hp * 0.35; // 小ユニット HP の 35% 固定ダメージ

        for unit in &mut self.player_units {
            // ハーフフィールドへ移動
            unit.position = unit.position.min(400.0);
            // ダメージ適用（HPが0以下になったら死亡させるため、max(1.0)を削除）
            unit.hp = unit.hp - damage;

            // 吹き飛ばし（ランダム距離と時間）
            let distance = rng.gen_range(30.0..200.0);
            let duration = rng.gen_range(0.35..0.85);
            // 左方向(自陣側)へ移動させる
            unit.knockback_velocity = -(distance / duration);
            unit.knockback_time = duration;
            unit.knockback_total = duration;
        }

        // HP が 0 以下のユニットを削除
        self.player_units.retain(|u| u.hp > 0.0);
    }
}
