import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Unit {
  id: number;
  unit_type: "Small" | "Medium" | "Large";
  position: number;
  hp: number;
  max_hp: number;
  attack: number;
  speed: number;
  is_player: boolean;
  target_id: number | null;
  knockback_velocity?: number;
  knockback_time?: number; // remaining
  knockback_total?: number; // total duration
}

interface Upgrades {
  small_attack: number;
  medium_attack: number;
  large_attack: number;
  small_hp: number;
  medium_hp: number;
  large_hp: number;
  small_speed: number;
  medium_speed: number;
  large_speed: number;
  coin_rate: number;
  base_hp: number;
}

interface GameState {
  player_units: Unit[];
  enemy_units: Unit[];
  player_base_hp: number;
  enemy_base_hp: number;
  coins: number;
  stage: number;
  click_count: number;
  type_count: number;
  upgrades: Upgrades;
}

const UPGRADE_BASE_COST = 1000;
const UPGRADE_LEVEL_STEP = 500;

const canvas = document.getElementById("game-canvas") as HTMLCanvasElement;
const ctx = canvas.getContext("2d")!;

let currentGameState: GameState | null = null;
let maxPlayerBaseHp = 1000;
let maxEnemyBaseHp = 500;

// ドット絵描画関数
function drawPixelUnit(x: number, y: number, type: "Small" | "Medium" | "Large", isPlayer: boolean, hpPercent: number) {
  const color = isPlayer ? "#00ff00" : "#ff0000";
  const hpColor = isPlayer ? "#00aa00" : "#aa0000";

  let size = 8;
  if (type === "Medium") size = 12;
  if (type === "Large") size = 16;

  // ユニット本体
  ctx.fillStyle = color;
  ctx.fillRect(x - size / 2, y - size / 2, size, size);

  // 目
  ctx.fillStyle = "#000";
  ctx.fillRect(x - size / 4, y - size / 4, 2, 2);
  ctx.fillRect(x + size / 4 - 2, y - size / 4, 2, 2);

  // HPバー
  const barWidth = size + 4;
  const barHeight = 2;
  ctx.fillStyle = "#000";
  ctx.fillRect(x - barWidth / 2, y - size / 2 - 6, barWidth, barHeight);
  ctx.fillStyle = hpColor;
  ctx.fillRect(x - barWidth / 2, y - size / 2 - 6, barWidth * hpPercent, barHeight);
}

function drawBase(x: number, y: number, isPlayer: boolean, hpPercent: number) {
  const color = isPlayer ? "#0088ff" : "#ff8800";
  const size = 40;

  // 基地の本体
  ctx.fillStyle = color;
  ctx.fillRect(x - size / 2, y - size / 2, size, size / 2);
  ctx.fillRect(x - size / 4, y - size / 2, size / 2, size);

  // 窓
  ctx.fillStyle = "#000";
  ctx.fillRect(x - 8, y - 4, 4, 4);
  ctx.fillRect(x + 4, y - 4, 4, 4);
  ctx.fillRect(x - 8, y + 8, 4, 4);
  ctx.fillRect(x + 4, y + 8, 4, 4);

  // HPバー
  const barWidth = size + 10;
  const barHeight = 4;
  ctx.fillStyle = "#000";
  ctx.fillRect(x - barWidth / 2, y - size / 2 - 12, barWidth, barHeight);
  ctx.fillStyle = isPlayer ? "#00ff00" : "#ff0000";
  ctx.fillRect(x - barWidth / 2, y - size / 2 - 12, barWidth * hpPercent, barHeight);
}

function render() {
  if (!currentGameState) return;

  // 画面クリア
  ctx.fillStyle = "#0a0a0a";
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  // 地面
  ctx.fillStyle = "#222";
  ctx.fillRect(0, canvas.height - 100, canvas.width, 100);

  // 中央線
  ctx.strokeStyle = "#444";
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(canvas.width / 2, 0);
  ctx.lineTo(canvas.width / 2, canvas.height);
  ctx.stroke();

  // プレイヤー基地
  const playerBaseX = 50;
  const playerBaseY = canvas.height - 150;
  const playerBaseHpPercent = currentGameState.player_base_hp / maxPlayerBaseHp;
  drawBase(playerBaseX, playerBaseY, true, playerBaseHpPercent);

  // 敵基地
  const enemyBaseX = canvas.width - 50;
  const enemyBaseY = canvas.height - 150;
  const enemyBaseHpPercent = currentGameState.enemy_base_hp / maxEnemyBaseHp;
  drawBase(enemyBaseX, enemyBaseY, false, enemyBaseHpPercent);

  // ユニット描画
  const unitY = canvas.height - 150;

  currentGameState.player_units.forEach((unit) => {
    const x = 100 + (unit.position / 1000) * (canvas.width - 200);
    const hpPercent = unit.hp / unit.max_hp;
    // 追加入力でノックバックアニメを表現
    let yOffset = 0;
    if (unit.knockback_time && unit.knockback_total && unit.knockback_total > 0) {
      const progress = 1 - unit.knockback_time / unit.knockback_total;
      const amplitude = 12 + (unit.unit_type === "Small" ? 0 : unit.unit_type === "Medium" ? 4 : 8);
      yOffset = Math.sin(progress * Math.PI) * amplitude;
    }
    drawPixelUnit(x, unitY - yOffset, unit.unit_type, true, hpPercent);
  });

  currentGameState.enemy_units.forEach((unit) => {
    const x = 100 + (unit.position / 1000) * (canvas.width - 200);
    const hpPercent = unit.hp / unit.max_hp;
    let yOffset = 0;
    if (unit.knockback_time && unit.knockback_total && unit.knockback_total > 0) {
      const progress = 1 - unit.knockback_time / unit.knockback_total;
      const amplitude = 12 + (unit.unit_type === "Small" ? 0 : unit.unit_type === "Medium" ? 4 : 8);
      yOffset = Math.sin(progress * Math.PI) * amplitude;
    }
    drawPixelUnit(x, unitY - yOffset, unit.unit_type, false, hpPercent);
  });
}

function updateUI(state: GameState) {
  document.getElementById("stage")!.textContent = state.stage.toString();
  document.getElementById("coins")!.textContent = state.coins.toString();
  document.getElementById("type-count")!.textContent = state.type_count.toString();
  document.getElementById("click-count")!.textContent = state.click_count.toString();

  const playerHpBar = document.getElementById("player-hp-bar") as HTMLDivElement;
  const playerHpText = document.getElementById("player-hp-text")!;
  const playerHpPercent = (state.player_base_hp / maxPlayerBaseHp) * 100;
  playerHpBar.style.width = `${playerHpPercent}%`;
  playerHpText.textContent = `${Math.round(state.player_base_hp)}/${maxPlayerBaseHp}`;

  const enemyHpBar = document.getElementById("enemy-hp-bar") as HTMLDivElement;
  const enemyHpText = document.getElementById("enemy-hp-text")!;
  const enemyHpPercent = (state.enemy_base_hp / maxEnemyBaseHp) * 100;
  enemyHpBar.style.width = `${enemyHpPercent}%`;
  enemyHpText.textContent = `${Math.round(state.enemy_base_hp)}/${maxEnemyBaseHp}`;

  // アップグレードボタンのコスト更新
  updateUpgradeCosts(state.upgrades, state.coins);
}

function updateUpgradeCosts(upgrades: Upgrades, coins: number) {
  const upgradeButtons = document.querySelectorAll(".upgrade-btn");
  upgradeButtons.forEach((btn) => {
    const button = btn as HTMLButtonElement;
    const type = button.dataset.type!;
    const unit = button.dataset.unit || "";

    let level = 0;
    switch (type) {
      case "attack":
        if (unit === "small") level = upgrades.small_attack;
        else if (unit === "medium") level = upgrades.medium_attack;
        else if (unit === "large") level = upgrades.large_attack;
        break;
      case "hp":
        if (unit === "small") level = upgrades.small_hp;
        else if (unit === "medium") level = upgrades.medium_hp;
        else if (unit === "large") level = upgrades.large_hp;
        break;
      case "speed":
        if (unit === "small") level = upgrades.small_speed;
        else if (unit === "medium") level = upgrades.medium_speed;
        else if (unit === "large") level = upgrades.large_speed;
        break;
      case "coin_rate":
        level = upgrades.coin_rate;
        break;
      case "base_hp":
        level = upgrades.base_hp;
        break;
    }

    const cost = UPGRADE_BASE_COST + level * UPGRADE_LEVEL_STEP;
    const costSpan = button.querySelector(".cost")!;
    costSpan.textContent = cost.toString();

    button.disabled = coins < cost;
  });
}

async function purchaseUpgrade(upgradeType: string, unitType: string) {
  try {
    await invoke("purchase_upgrade", {
      upgradeType,
      unitType,
    });
  } catch (error) {
    console.error("Failed to purchase upgrade:", error);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  // ウィジェットか通常モードかを判定
  const isWidget = window.location.hash === "#widget" || window.location.href.includes("#widget");
  console.log("[main] DOMContentLoaded: isWidget=", isWidget, "hash=", window.location.hash, "href=", window.location.href);

  if (isWidget) {
    // ウィジェットモード
    console.log("[main] Entering widget mode");
    setupWidget();
  } else {
    // 通常のゲームモード
    console.log("[main] Entering game mode");
    setupGame();
  }
});

function setupWidget() {
  const widgetContainer = document.getElementById("widget-container")!;
  console.log("[setupWidget] widget-container element:", widgetContainer);
  widgetContainer.classList.remove("hidden");

  // メインUI非表示
  const gameContainer = document.getElementById("game-container")!;
  gameContainer.style.display = "none";

  document.body.style.background = "transparent";
  document.documentElement.style.background = "transparent";
  document.body.style.margin = "0";
  document.body.style.overflow = "hidden";
  document.body.style.pointerEvents = "none";

  // ウィジェット用キャンバス
  const widgetCanvas = document.getElementById("widget-canvas") as HTMLCanvasElement;
  const widgetCtx = widgetCanvas.getContext("2d")!;
  console.log("[setupWidget] initial canvas size:", { width: widgetCanvas.width, height: widgetCanvas.height });

  const syncWidgetCanvas = () => {
    const oldWidth = widgetCanvas.width;
    const oldHeight = widgetCanvas.height;
    widgetCanvas.width = window.innerWidth;
    widgetCanvas.height = window.innerHeight;
    console.log("[setupWidget] canvas resized:", { from: { width: oldWidth, height: oldHeight }, to: { width: widgetCanvas.width, height: widgetCanvas.height } });
  };

  syncWidgetCanvas();
  window.addEventListener("resize", syncWidgetCanvas);

  let currentState: GameState | null = null;

  // リアルタイム更新をセットアップ
  const setupRealtimeUpdates = async () => {
    try {
      // 初期状態取得
      console.log("[setupWidget] Fetching initial game state...");
      const initialState = await invoke("get_game_state");
      currentState = initialState as GameState;
      console.log("[setupWidget] Initial state received:", { playerUnits: currentState.player_units.length, enemyUnits: currentState.enemy_units.length });
      renderWidget(widgetCtx, widgetCanvas, currentState);
    } catch (e) {
      console.error("[setupWidget] Failed to get initial state:", e);
    }

    // game-update イベントリスナー
    try {
      console.log("[widget] Setting up game-update listener...");
      await listen("game-update", (event: any) => {
        currentState = event.payload as GameState;
        console.log("[widget] game-update received:", {
          playerUnits: currentState.player_units.length,
          enemyUnits: currentState.enemy_units.length,
          stage: currentState.stage,
        });
        renderWidget(widgetCtx, widgetCanvas, currentState);
      });
      console.log("[widget] game-update listener established");
    } catch (e) {
      console.error("[widget] Failed to setup listener:", e);
    }

    // ポーリングによる定期更新（フォールバック）
    setInterval(async () => {
      try {
        const state = await invoke("get_game_state");
        currentState = state as GameState;
        renderWidget(widgetCtx, widgetCanvas, currentState);
      } catch (e) {
        console.error("[widget] Polling failed:", e);
      }
    }, 100); // 100ms ごとにポーリング
  };

  setupRealtimeUpdates();
}

function renderWidget(ctx: CanvasRenderingContext2D, canvas: HTMLCanvasElement, state: GameState) {
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  const baseline = canvas.height / 2;
  const convertX = (position: number) => (position / 1000) * canvas.width;

  // ユニット数表示（背景あり）
  ctx.fillStyle = "#000000";
  ctx.fillRect(0, 0, 200, 30);
  ctx.fillStyle = "#ffffff";
  ctx.font = "16px monospace";
  ctx.fillText(`P:${state.player_units.length} E:${state.enemy_units.length}`, 5, 20);

  state.player_units.forEach((unit) => {
    const x = convertX(unit.position);
    const size = unit.unit_type === "Small" ? 4 : unit.unit_type === "Medium" ? 6 : 8;
    // Knockback vertical animation: sinusで少し上がるようにする
    let yOffset = 0;
    if (unit.knockback_time && unit.knockback_total && unit.knockback_total > 0) {
      const progress = 1 - unit.knockback_time / unit.knockback_total;
      const amplitude = 6 + (size / 2);
      yOffset = Math.sin(progress * Math.PI) * amplitude;
    }
    ctx.fillStyle = "#0088ff"; // 青色（味方）
    ctx.fillRect(x - size / 2, baseline - size / 2 - yOffset, size, size);
  });

  state.enemy_units.forEach((unit) => {
    const x = convertX(unit.position);
    const size = unit.unit_type === "Small" ? 4 : unit.unit_type === "Medium" ? 6 : 8;
    let yOffset = 0;
    if (unit.knockback_time && unit.knockback_total && unit.knockback_total > 0) {
      const progress = 1 - unit.knockback_time / unit.knockback_total;
      const amplitude = 6 + (size / 2);
      yOffset = Math.sin(progress * Math.PI) * amplitude;
    }
    ctx.fillStyle = "#ff0000";
    ctx.fillRect(x - size / 2, baseline - size / 2 - yOffset, size, size);
  });
}

function setupGame() {
  // キャンバスのリサイズ
  function resizeCanvas() {
    const container = document.getElementById("game-container")!;
    const topBar = document.getElementById("top-bar")!;
    const baseStats = document.getElementById("base-stats")!;

    const height = container.clientHeight - topBar.clientHeight - baseStats.clientHeight;
    canvas.width = container.clientWidth;
    canvas.height = height;
  }

  resizeCanvas();
  window.addEventListener("resize", resizeCanvas);

  // メニュー表示/非表示
  const menuBtn = document.getElementById("menu-btn")!;
  const closeMenuBtn = document.getElementById("close-menu-btn")!;
  const menuOverlay = document.getElementById("menu-overlay")!;

  menuBtn.addEventListener("click", () => {
    menuOverlay.classList.remove("hidden");
  });

  closeMenuBtn.addEventListener("click", () => {
    menuOverlay.classList.add("hidden");
  });

  // アップグレードボタン
  const upgradeButtons = document.querySelectorAll(".upgrade-btn");
  upgradeButtons.forEach((btn) => {
    btn.addEventListener("click", async () => {
      const button = btn as HTMLButtonElement;
      const type = button.dataset.type!;
      const unit = button.dataset.unit || "";
      await purchaseUpgrade(type, unit);
    });
  });

  // ゲーム状態の更新を受信（非同期で処理）
  listen("game-update", (event: any) => {
    currentGameState = event.payload as GameState;
    
    // 最大HP更新の検知
    if (currentGameState.player_base_hp > maxPlayerBaseHp * 0.99) {
      maxPlayerBaseHp = currentGameState.player_base_hp;
    }
    if (currentGameState.enemy_base_hp > maxEnemyBaseHp * 0.99) {
      maxEnemyBaseHp = currentGameState.enemy_base_hp;
    }

    updateUI(currentGameState);
    render();
  }).catch((e) => console.error("Failed to listen to game-update:", e));

  // 初期状態取得（非同期で処理）
  invoke("get_game_state").then((state: any) => {
    currentGameState = state as GameState;
    if (currentGameState) {
      maxPlayerBaseHp = currentGameState.player_base_hp;
      maxEnemyBaseHp = currentGameState.enemy_base_hp;
      updateUI(currentGameState);
      render();
    }
  }).catch((e) => console.error("Failed to get game state:", e));

  // レンダリングループ
  setInterval(render, 1000 / 60);
}
