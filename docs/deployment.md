# Разгръщане (deployment)

## Web

```bash
docker compose up --build
# UI: http://localhost:3022   API: http://localhost:8095   PostgreSQL: :5434
```

`docker-compose.yml` вдига три контейнера:

- `dzambo-postgres` — `postgres:17-alpine`, healthcheck, volume `dzambo_pgdata`.
- `dzambo-server` — Rust API (:8095); миграциите са embedded, seed при старт;
  `DZAMBO_BIND=0.0.0.0`, `DZAMBO_SUPERADMIN_PASSWORD`.
- `dzambo-frontend` — Next.js standalone (:3022); `YTBUCH_API_PROXY=http://server:8095`
  сочи към API; proxy-ва `/v1`.

Променливи: `DATABASE_URL`, `DZAMBO_BIND` (default 127.0.0.1), `DZAMBO_PORT`,
`DZAMBO_SUPERADMIN_PASSWORD`, `FRONTEND_DIST`.

## Desktop (AppImage)

Desktop-ът е **тънък клиент** към сървъра (винаги online). Launcher
(`src-tauri/ui/index.html`) въвежда адрес на сървъра и зарежда remote Next app.

### Build

Необходими системни пакети (Debian/Ubuntu):

```bash
sudo apt install librsvg2-bin patchelf webkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev
npm install -g @tauri-apps/cli@^2
tauri build --bundles appimage
```

Резултат: `target/release/bundle/appimage/dzambo_0.1.0_amd64.AppImage`.

### Стартиране

```bash
./target/release/bundle/appimage/dzambo_0.1.0_amd64.AppImage
# въведи адрес на сървъра (default http://127.0.0.1:3022) → Свързи
```

### Dev режим

```bash
# 1. API + фронт чрез compose (или native)
# 2. Tauri dev (devUrl = http://127.0.0.1:3022)
tauri dev
```

## Конфигурация

| Env | Слой | Смисъл |
|-----|------|--------|
| `DATABASE_URL` | server | Pg връзка |
| `DZAMBO_BIND` | server | bind адрес (default 127.0.0.1; Docker 0.0.0.0) |
| `DZAMBO_PORT` | server | default 8095 |
| `DZAMBO_SUPERADMIN_PASSWORD` | server | парола на супер администратора |
| `MISTRAL_API_KEY` | server | AI OCR/класификация (по желание) |
| `YTBUCH_API_PROXY` | frontend | адрес на API (bake-ва се при build) |

Mistral AI е **опционално**: без ключ cn8-suggest работи от локалния каталог,
scans/extract връща 422 с ясна дирекция.
