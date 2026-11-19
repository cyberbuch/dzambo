# dzambo — миграция на ytbuch

Същата счетоводна програма като ytbuch, нов стек. Източник: `/home/ziko/ytgit/ytbuch`
(Rust + Axum 0.8, YottaDB/Octo, Next.js 16). Няма продуктови данни — схемата се
проектира чисто, без миграция на данни. ytbuch е ~90% завършен и може да има
грешки; складът там не е тестван. Пренасяме логиката като референция, но коректно
за PostgreSQL.

**Модел:** desktop приложение (Tauri v2), което се свързва със сървъра през порт
(HTTP). Една база — PostgreSQL. Без SQLite, без локално копие, без sync.
Desktop-ът винаги работи срещу сървъра; web вариантът е същият код, пуснат
другояче (axum сервира SPA-та).

## Стек (реален)

| Слой | ytbuch (старо) | dzambo (ново) |
|------|----------------|---------------|
| API | Rust + Axum 0.8 + tokio | същото, запазва се |
| ORM/база | низов SQL през tokio-postgres → Octo | Diesel 2.4 + diesel-async 0.10 + deadpool → PostgreSQL 17 |
| Frontend | Next.js 16 App Router, обикновен CSS | **оригиналният Next.js 16 фронт, пренесен едно към едно** |
| Desktop | — | Tauri v2 обвивка (devUrl → Next dev сървъра) |
| Типове | ръчен fetch клиент | ts-rs (HTTP DTO) + tauri-specta (native команди) |

> Минималният Vite/shadcn фронт беше отхвърлен („няма нищо общо с изходния“).
> Затова пренесохме оригиналните екрани на ytbuch (app/, components/, lib/) и
> запазихме API договора, за да работят без пренаписване. Vite скелето е
> изместено в `frontend-vite-skeleton/` (gitignored).

## Неприкосновени правила на домейна

Пренасят се едно към едно от ytbuch, не се пренаписват:

- Черно сторно: отрицателен знак, без обръщане на Дт/Кт.
- Сумите в JSON са низове. `company_id` дели данните по фирма.
- Постнат документ е immutable — корекция само със сторно.
- API договорът: `{items,count,total,limit,offset,has_more}` за списъци,
  `{"detail":...}` за грешки; страниране по 50 (макс 500) с филтриране измежду
  всички редове.
- Бизнес логиката (ДДС, SAF-T, склад WAC/FIFO/LIFO, производство, дивиденти,
  банки CSV/MT940/camt.053, печат PDF/DOCX/ODT/UBL, сканиране, VIES, ЕЦБ) се
  пренася модул по модул.
- i18n речниците (bg/en), каталозите на НАП (сметки 360, мерни единици 127,
  данъчни кодове 169, КН8) и демо фирма YT ООД се пренасят.
- **SAF-T = мапинг на номенклатури:** всяка фирма има свободен сметкоплан;
  SAF-T изнася `AccountID` = НАП код (`accounts.saft_account_type`),
  `TaxpayerAccountID` = собствения номер.

## Архитектура

```
crates/
  domain/    домейн: модели, DTO (serde + ts_rs::TS), auth (argon2/JWT),
             номенклатури, vat_ex каталог
  storage/   Diesel + diesel-async + deadpool: схема, миграции, заявки (Pg)
    migrations/   0001..0015 (истинските миграции са ТУК, не в /migrations)
  api/       axum router + handlers, JWT middleware, SAF-T XML, UBL, печат,
             vatfile (точен НАП файл)
  server/    bin: api + storage, сервира и built SPA-та (ServeDir)
frontend/    оригиналният ytbuch Next.js 16 (app/, components/, lib/)
  src/bindings/   ts-rs DTO типове (генерирани от cargo test)
src-tauri/   Tauri v2 shell: devUrl http://127.0.0.1:3022
```

### Портове (dev)

| Услуга | Адрес |
|--------|-------|
| PostgreSQL | `127.0.0.1:5434` — контейнер `dzambo-postgres`, потребител/парола/база `dzambo` |
| API (server bin) | `http://127.0.0.1:8095` |
| Next фронт | `http://127.0.0.1:3022` (proxy `/v1`,`/ready`,`/health` → 8095) |

8092 и 3020 са заети от ytbuch; 5433 — от botyaga.
`DATABASE_URL` по подразбиране: `postgres://dzambo:dzambo@127.0.0.1:5434/dzambo`.
Dev superadmin: `superadmin` / `dzambo-admin-dev` (env `DZAMBO_SUPERADMIN_PASSWORD`).
Login: `POST /v1/auth/token` с `{sub|email, password}` → `access_token`.

### Една кодова база, два начина на пускане

- **Desktop:** Tauri зарежда Next dev сървъра (:3022); API-то е на :8095.
- **Web:** същият axum `server` сервира built SPA-та през `ServeDir`.
  Deploy = docker compose (api + postgres, 2 контейнера).

## Схема (само PostgreSQL)

Octo ограниченията изчезват: истински транзакции за пост/сторно, индекси, FK,
IDENTITY. Пари/количества = NUMERIC; JSON остава низ; никакъв float по пътя
JSON ↔ Diesel ↔ Pg. `core` crate е преименуван на `domain` (конфликт с diesel
макрата). Diesel features: `64-column-tables`, `serde_json`.

## Статус по фази

| Фаза | Съдържание | Статус |
|------|-----------|--------|
| 0. Скеле + сонда | workspace, Tauri shell, ts-rs/tauri-specta, сонда diesel-async | ✅ |
| 1. Вертикален разрез | Pg схема + seed (НАП, YT ООД), auth, фирми, табло, ServeDir | ✅ |
| 2. Номенклатури | сметкоплан, контрагенти, стоки (КН8), настройки | ✅ |
| 3. Фактури + дневник | чернова → номер → пост → черно сторно (транзакция); печат/UBL | ✅ |
| 4. ДДС | дневници, **точен НАП файл**, периоди/заключване, начални салда | ✅ |
| 5. Операции | банки CSV/MT940/camt.053, правила, ДМА, склад, производство, дивиденти | ✅ (финни дългове) |
| 6. SAF-T + справки | SAF-T XML (месец/година/при поискване) — **0 XSD грешки**; одитна следа | ✅ |
| 7. Auth flows + роли | роли/членства/permissions, одит, вътрешни документи, изписване, правила за склад | ✅ (register/forgot/2FA остават) |
| 8. Desktop + deploy | tauri-specta през platform.ts, инсталатори, docker compose | ⏳ |

### Пренесена реална логика (commit 3dd3b84, миграция 0015)

- **Роли/членства:** `roles.permissions` = JSON масив от ключове; seed на
  „Администратор на фирма“, „Счетоводител“, „Фактурист“; CRUD за супер админ.
- **Одит:** `audit_logs` се пише при пост/отпост/изтриване на фактура, протокол,
  изписване; филтри (entity/action/user/q/changed_only) + `amount_changed`.
- **Вътрешни документи:** CRUD, next-number (10 цифри), confirm (премества ДМА +
  event; продуктовите редове се движат от rebuild), печат PDF/DOCX/ODT.
- **Изписване:** `material_issues` (МИ-{id}) CRUD + confirm/unpost през rebuild;
  `stock_account_rules` (goods/production/material); месечно изписване
  (Дт разход / Кт материали), идемпотентно по месец.
- **Партиди:** `stock_replay` rebuild WAC/FIFO/LIFO, презапис на наличности и
  journal-а на ръчните изписвания.
- **НАП файл:** `api/vatfile.rs` — POKUPKI (276 байта), PRODAGBI (426),
  DEKLAR (592), VIES + ZIP (stored); фиксирана ширина, Windows-1251, CRLF;
  `/v1/vat/export?type=pokupki|prodagbi|vies|zip`. Тестове заключват ширините.

## Финни дългове (за края)

- ДМА update/revalue/move; bulk UI; Postbank/UniCredit/OBB CSV варианти.
- Auth flows: register/forgot/reset/2FA/registration-status.
- Оставащи 404-и, които оригиналният UI вика: addresses/parse, vies-lookup,
  cn8-suggest, scans/*, system-settings/*, dividend-distributions,
  fixed-asset-categories, product-name-mappings, fs/formulas, journal/bulk.
- Паритет: PATCH с пълно тяло вместо частичен; rate limit на логин; AI КН8;
  region→SAFT код; спец. документи 07/09 частичен пост.

## Какво НЕ се пренася

- Octo хаковете: escape-backslash детекция, simple query, `MAX(id)+1`, ръчен
  pool с reconnect, ръчно триене при провален пост.
- (От предишни версии: SQLite, dual-backend Diesel, oplog/sync, uuidv7 — няма
  нужда без offline. Vite/shadcn фронтът — заменен от оригиналния Next.)

## Конвенции

- Файловете са ≤800 реда — по-дълги се делят на модули (напр. stockops +
  stock_replay).
- Rust без warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
- Commit съобщенията и коментарите са на български.
- Миграциите са в `crates/storage/migrations/`.

## Проверки преди всяко качване

`cargo fmt --all` + `cargo clippy --workspace --all-targets -- -D warnings` +
`cargo test --workspace` — всички зелени; за SAF-T и `xmllint` 0 грешки.
Сонда преди поправка, не след. Щом минат — `git push origin main`.