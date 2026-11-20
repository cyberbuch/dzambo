# Архитектура

## Стек

| Слой | Технология |
|------|-----------|
| API | Rust, Axum 0.8, tokio |
| ORM/база | Diesel 2.4 + diesel-async 0.10 + deadpool → PostgreSQL 17 |
| Frontend | Next.js 16 (App Router), оригиналните екрани на ytbuch |
| Desktop | Tauri v2 (webkit2gtk webview) |
| Типове | ts-rs (HTTP DTO) + tauri-specta (native команди) |
| Auth | Argon2id пароли + JWT HS256 (issuer `dzambo`, 7 дни) |
| SAF-T/UBL/печат | ръчни XML генератори; PDF (ironpress), DOCX/ODT (ferrodoc), вграден DejaVu |

## Слоеве и crate-ове

```
crates/
  domain/   чист домейн: auth (argon2/JWT/TOTP), dto (serde+ts-rs),
            fsnom/fsform (бланки и формули), reports_* (books/build/sheet),
            cn8 (каталог КН8), totp, addr (адресен парсер), noms/vat_ex/smap_def
  storage/  Diesel схема + миграции (24) + заявки; models/{invoices,journal,...}
  api/      axum router + handlers + SAF-T + UBL + печат + НАП файл (vatfile)
  server/   bin: API + статика + миграции при старт
frontend/   Next.js 16 (app/, components/, lib/) — proxy /v1 към API
src-tauri/  Tauri v2 launcher (ui/index.html) → remote app
```

## Request flow (web)

```
Браузер → Next frontend (:3022) ──/v1/*──▶ dzambo API (:8095) ──▶ PostgreSQL (:5434)
             │                                     ▲
             └── JWT Bearer, proxy rewrites ───────┘
```

Next сервира уеб интерфейса и proxy-ва `/v1`, `/ready`, `/health` към API-то
(rewrites в `next.config.ts`, `YTBUCH_API_PROXY`). API-то е stateless — всичко
състояние е в PostgreSQL, JWT в localStorage.

## Request flow (desktop)

Desktop-ът е **тънък клиент**: Tauri native прозорец с launcher
(`src-tauri/ui/index.html`), който въвежда адрес на сървъра и зарежда remote
Next app. След това потокът е същият като web (frontend → API). Няма локална
база; desktop-ът винаги е online към сървъра.

## Една кодова база, два начина на пускане

- **Web:** `docker compose up --build` (postgres + server + frontend).
- **Desktop:** `tauri build --bundles appimage` → самостоятелен AppImage.

## Модули (backend)

| Модул | Отговорност |
|-------|-------------|
| auth | token/register/forgot/reset/mfa, 2FA enrollment, me |
| access | роли/членства/permissions, одитна следа |
| invoices/journal/posting | фактури, дневник, атомарен пост/сторно |
| vat | дневници ДДС, точен НАП файл, ръчни клетки |
| saft | SAF-T XML 3 режима, номенклатури, movement-mappings |
| stock/stockops/stock_replay | склад WAC/FIFO/LIFO, изписване, rebuild, stock справка |
| fixed_assets/asset_ops | ДМА, амортизация, revalue/move/conserve/dispose |
| dividends/dividend_distributions | протоколи + разпределение по собственици |
| banks/bank_rules | сметки, CSV/MT940/camt.053, auto-allocate |
| reports | всички справки + export pdf/xlsx/ods |
| exchange-rates | ЕЦБ курсове |
| fs/formulas | формули за финансови отчети (по фирма) |
| idocs | вътрешни документи (протоколи за преместване) |
| product-name-mappings | мапинг на сканирани имена → артикули |
| vies | VIES REST lookup + създаване на контрагент |
| cn8 | КН8 класификация (каталог + Mistral AI) |
| scan | UBL импорт + Mistral OCR extract |
| system-settings / me | глобални настройки + профил |

## Конвенции

- Сумите в JSON са **низове**; черно сторно с **отрицателен знак**.
- `company_id` дели данните по фирма; активната фирма се резолва от потребителя.
- Грешки: `{"detail": "..."}` + HTTP статус.
- Списъци: `{items, count, total, limit, offset, has_more}`, лимит 50 (макс 500).
