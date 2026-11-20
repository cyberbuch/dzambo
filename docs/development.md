# Разработка

## Изисквания

- Rust (stable, тествано 1.97), Node.js 20+, Docker.
- За desktop build: webkit2gtk-4.1-dev, libgtk-3-dev, libsoup-3.0-dev,
  librsvg2-bin, patchelf + `@tauri-apps/cli`.

## Стартиране (dev)

```bash
# PostgreSQL
docker run -d --name dzambo-postgres \
  -e POSTGRES_USER=dzambo -e POSTGRES_PASSWORD=dzambo -e POSTGRES_DB=dzambo \
  -p 5434:5432 postgres:17

# API (:8095) — миграции + seed автоматично
cargo run -p server

# Frontend (:3022)
cd frontend && npm install
YTBUCH_API_PROXY=http://127.0.0.1:8095 npx next dev -p 3022
```

`DATABASE_URL` default: `postgres://dzambo:dzambo@127.0.0.1:5434/dzambo`.
Login: `superadmin` / `dzambo-admin-dev` (env `DZAMBO_SUPERADMIN_PASSWORD`).

## Батерии

Преди всяко пушване — всички зелени:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

SAF-T се валидира: `xmllint --noout --schema BG_SAFT_Schema_V_1.0.1.xsd <file>`.

## Структура на работата

- Работи се **модул по модул**, пазейки паритета с ytbuch.
- Комуникация и коментари — на български; commit съобщения — български.
- Пушва се в `main` след всяка фаза („щом минат — качи").
- UI на живо в браузър при промяна на frontend.

## Конвенции

- **Файловете са ≤800 реда** — по-дълги се делят на модули (пример:
  `stockops` + `stock_replay`, `reports_data`/`reports_fs`/...).
- Rust без warnings (`clippy -D warnings`).
- Няма float за пари; BigDecimal / i128 стотинки.
- Миграциите са последователни, в `crates/storage/migrations/`.
- Diesel features: `64-column-tables`, `serde_json`; заявките през
  `AsyncPgConnection`, транзакции — ръчен `BEGIN/COMMIT/ROLLBACK`.

## Типове (ts-rs / tauri-specta)

DTO-тата се генерират в `frontend/src/bindings/` чрез `cargo test` (ts-rs).
Native командите — чрез tauri-specta (`src-tauri/src/lib.rs`), export в debug.

## Често срещани грешки

- Нов Pg контейнер/порт: увери се, че :5434 е свободен.
- CSP в dev: `middleware.ts` слага `'unsafe-eval'` само в development.
- Diesel 32-колонков лимит: companies(47 кол.) → `company_details` 1:1 +
  `64-column-tables`.
- LEFT JOIN селекти: `Option::<T>::as_select()`.
