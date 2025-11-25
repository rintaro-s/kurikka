# Clicker Clicker Clicker - Multiplayer Server

Actix-webベースのマルチプレイヤーサーバー

## 起動方法

```bash
cd multiplayer-server
cargo run
```

サーバーは `http://0.0.0.0:8080` で起動します。

## API エンドポイント

### ヘルスチェック
```
GET /health
```

### プレイヤー登録
```
POST /api/player/register
Content-Type: application/json

{
  "name": "PlayerName"
}
```

Response:
```json
{
  "player_id": "uuid",
  "player_name": "PlayerName",
  "stage": 1,
  "last_update": 1234567890
}
```

### プレイヤー情報取得
```
GET /api/player/{player_id}
```

### プレイヤー状態更新
```
POST /api/player/{player_id}/update
Content-Type: application/json

{
  "player_id": "uuid",
  "player_units": [...],
  "enemy_units": [...],
  "player_base_hp": 1000.0,
  "enemy_base_hp": 500.0,
  "coins": 100,
  "stage": 5
}
```

### プレイヤー一覧
```
GET /api/players
```

## 設定

Tauriアプリ側で接続先URLを設定可能。サーバーが起動していない場合は、通常のシングルプレイモードで動作します。
