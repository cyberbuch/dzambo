# dzambo

Счетоводна програма — миграция на [ytbuch](https://github.com/cyberbuch) към нов
стек: **Rust + Axum + Diesel + PostgreSQL**, с оригиналния Next.js фронт и
Tauri v2 desktop обвивка. Една база, без SQLite/sync; desktop-ът се свързва със
сървъра през порт, а web вариантът е същият код, сервиран от axum.

> Няма продуктови данни — схемата се проектира чисто. Бизнес логиката (ДДС,
> SAF-T, склад WAC/FIFO/LIFO, производство, дивиденти, банки, печат, UBL) се
> пренася едно към едно от ytbuch. Виж [PLAN.md](PLAN.md) за фази и статус.

## Стек

- **Backend:** Rust, Axum 0.8, tokio, Diesel 2.4 + diesel-async + deadpool,
  PostgreSQL 17. Argon2id пароли + JWT (HS256).
- **Frontend:** оригиналният ytbuch Next.js 16 (App Router), пренесен едно към
  едно; работи срещу dzambo API чрез proxy.
- **Desktop:** Tauri v2 (`src-tauri/`), ts-rs + tauri-specta за типове/native.
- **SAF-T:** XML V1.0.1, мапинг на свободния сметкоплан към НАП номенклатурите.

## Структура

```
crates/
  domain/    модели, DTO (ts-rs), auth, номенклатури, vat_ex
  storage/   Diesel схема + заявки; migrations/0001..0015
  api/       axum router, handlers, SAF-T, UBL, печат, НАП файл (vatfile)
  server/    bin: API + сервиране на built SPA (ServeDir)
frontend/    Next.js 16 фронт (app/, components/, lib/)
src-tauri/   Tauri v2 обвивка
```

## Изисквания

- Rust (stable), Node.js 20+, Docker (за PostgreSQL).
- `cargo`, `npx`.

## Стартиране (dev)

### 1. PostgreSQL

```bash
docker run -d --name dzambo-postgres \
  -e POSTGRES_USER=dzambo -e POSTGRES_PASSWORD=dzambo -e POSTGRES_DB=dzambo \
  -p 5434:5432 postgres:17
```

### 2. API сървър

Миграциите и seed-ът (НАП номенклатури, демо фирма, роли) се прилагат
автоматично при старт.

```bash
cargo run -p server
# API: http://127.0.0.1:8095
```

`DATABASE_URL` по подразбиране: `postgres://dzambo:dzambo@127.0.0.1:5434/dzambo`.

### 3. Frontend (Next.js)

```bash
cd frontend
npm install
YTBUCH_API_PROXY=http://127.0.0.1:8095 npx next dev -p 3022
# Frontend: http://127.0.0.1:3022
```

### 4. Desktop (Tauri, по избор)

```bash
cargo tauri dev   # devUrl сочи към Next dev сървъра :3022
```

## Портове

| Услуга | Адрес |
|--------|-------|
| PostgreSQL | `127.0.0.1:5434` (контейнер `dzambo-postgres`) |
| API | `http://127.0.0.1:8095` |
| Next фронт | `http://127.0.0.1:3022` |

8092/3020 са заети от ytbuch; 5433 — от botyaga.

## Dev достъп

- Потребител: `superadmin`
- Парола: `dzambo-admin-dev` (env `DZAMBO_SUPERADMIN_PASSWORD`)
- Login: `POST /v1/auth/token` с `{"sub":"superadmin","password":"dzambo-admin-dev"}`
  → `access_token` (Bearer).

## Основни endpoint-и

- Фактури/дневник: `/v1/invoices`, `/v1/journal`, пост/сторно, печат, UBL.
- ДДС: `/v1/vat/sales`, `/v1/vat/purchases`, **`/v1/vat/export?type=pokupki|prodagbi|vies|zip`**
  (точен НАП файл: Windows-1251, фиксирана ширина, CRLF).
- SAF-T: `/v1/saft/*` (месец/година/при поискване), `/v1/nomenclatures?kind=`.
- Склад: `/v1/stock/rules`, `/v1/stock/writeoff`, `/v1/stock/rebuild`,
  `/v1/material-issues`, `/v1/internal-docs`, `/v1/locations`.
- Роли/одит: `/v1/roles`, `/v1/audit-logs`.

## Проверки (батерии)

Преди всяко качване — всички зелени:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

SAF-T се валидира с `xmllint --noout --schema BG_SAFT_Schema_V_1.0.1.xsd` (0 грешки).

## Конвенции

- Файлове ≤800 реда; по-дългите се делят на модули.
- Rust без warnings; commit съобщения и коментари на български.
- Сумите в JSON са низове; черно сторно с отрицателен знак (без обръщане Дт/Кт).
- Миграциите са в `crates/storage/migrations/`.

## Лиценз

Вътрешен проект на cyberbuch.