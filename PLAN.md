# dzambo — миграция на ytbuch

Същата счетоводна програма като ytbuch, нов стек. Източник: `/home/ziko/ytgit/ytbuch`
(Rust + Axum 0.8, YottaDB/Octo, Next.js 16). Няма продуктови данни — схемата се
проектира чисто, без миграция на данни.

**Модел:** desktop приложение (Tauri), което се свързва със сървъра през порт
(HTTP). Една база — PostgreSQL. Без SQLite, без локално копие, без sync.
Desktop-ът винаги работи срещу сървъра; web вариантът е същият код, пуснат
другояче.

| Слой | ytbuch (старо) | dzambo (ново) |
|------|----------------|---------------|
| API | Rust + Axum 0.8 + tokio | същото, запазва се |
| ORM/база | низов SQL през tokio-postgres → Octo | Diesel + diesel-async → PostgreSQL |
| Frontend | Next.js 16 App Router, обикновен CSS | Tauri v2 + Vite + React + TS |
| Дизайн | globals.css / module.css | Tailwind CSS + shadcn/ui |
| Състояние | fetch + localStorage | Zustand (сесия/активна фирма) + TanStack Query (сървърни данни) |
| Типове | ръчен fetch клиент | ts-rs (HTTP DTO) + tauri-specta (native команди) |

## Неприкосновени правила на домейна

Пренасят се едно към едно от ytbuch, не се пренаписват:

- Черно сторно: отрицателен знак, без обръщане на Дт/Кт.
- Сумите в JSON са низове. `company_id` дели данните по фирма.
- Постнат документ е immutable — корекция само със сторно.
- API договорът: `items`/`count` за списъци, `{"detail":...}` за грешки,
  страниране по 50 с търсене измежду всички редове.
- Бизнес логиката (33k реда: ДДС, SAF-T, склад WAC/FIFO/LIFO, производство,
  дивиденти, банки CSV/MT940/camt.053, печат PDF/DOCX/ODT/UBL, сканиране,
  VIES, ЕЦБ) се пренася модул по модул, не се преизмисля.
- i18n речниците (bg/en), каталозите на НАП (сметки 360, мерни единици 127,
  данъчни кодове 169, КН8) и демо фирма YT ООД се пренасят.

## Архитектура

```
crates/
  core/      домейн: модели, DTO (serde + ts_rs::TS), счетоводни правила,
             dec (десетични), номенклатури, SAF-T XML, UBL, печат
  storage/   Diesel + diesel-async + deadpool: схема, миграции, заявки (Pg)
  api/       axum router + handlers, JWT, rate limit
  server/    bin: api + storage, сервира и built SPA-та (ServeDir)
frontend/    Vite + React + TS + Tailwind + shadcn/ui
  src/
    bindings/   ts-rs DTO типове + tauri-specta bindings (генерирани)
    lib/        api.ts (HTTP клиент), platform.ts (web ↔ tauri адаптер)
    stores/     Zustand: сесия, активна фирма, адрес на сървъра
    features/   екрани по домейн: invoices, vat, stock, ...
    components/ shadcn/ui + общи
desktop/     Tauri v2 shell: обвива SPA-та, tauri-specta native команди
```

### Портове (dev)

| Услуга | Адрес |
|--------|-------|
| PostgreSQL | `127.0.0.1:5434` — контейнер `dzambo-postgres`, потребител/парола/база `dzambo` |
| API (server bin) | `http://127.0.0.1:8095` |
| Vite dev | `http://127.0.0.1:5173` |

8092 и 3020 са заети от ytbuch; 5433 — от botyaga.

### Една кодова база, два начина на пускане

- **Desktop:** Tauri зарежда SPA-та; `api.ts` сочи към конфигурируем адрес на
  сървъра (първи старт / настройки, по подразбиране `http://127.0.0.1:8095`).
  JWT токенът се пази в tauri-plugin-store.
- **Web:** същият axum `server` сервира built SPA-та през `ServeDir` — още от
  фаза 1, не като отделен проект. `api.ts` сочи към same-origin. JWT в
  localStorage. Deploy = `docker compose up` (api + postgres, 2 контейнера).

### Правилото, което прави web преработката безплатна

**Споделеният frontend код никога не вика Tauri директно.** Цялата native
функционалност минава през `lib/platform.ts` адаптер:

| Възможност | web | desktop (tauri-specta) |
|------------|-----|------------------------|
| токен | localStorage | tauri-plugin-store |
| печат | browser print / PDF сваляне | native печат |
| запис на файл | browser download | native save диалог |
| отваряне на файл | input[type=file] | native open диалог |

tauri-specta типизира само тези команди. HTTP DTO-тата идват от ts-rs export
(`cargo test` генерира `frontend/src/bindings/`). TanStack Query клиентът е
един и същ — HTTP към axum, без значение дали сървърът е localhost или VPS.

## Схема (само PostgreSQL)

Octo ограниченията изчезват: истински транзакции за пост/сторно (в ytbuch
постът не е атомарен и се трие ръчно при провал), индекси, FK, IDENTITY.

| Домейн | Тип | Бележка |
|--------|-----|---------|
| id | BIGINT IDENTITY | както сега, без uuid нужда |
| пари/количества | NUMERIC(18,4) | rust_decimal в домейна; JSON остава низ; никакъв float |
| време | TIMESTAMPTZ | |
| изтриване | както е по домейн | одитната следа се пази |

~28 таблици от schema5.rs + schema_late.rs, преработени с истински типове.
**Фаза 0 включва жива сонда** на diesel-async + deadpool + Pg (docs ≠
поддържано от конкретната версия) — 20-редачен spike, не дизайн на хартия.

## Фази

Критерият за готово е работещ екран, не зелени тестове. Всеки екран се портва
в същата фаза като API-то си.

| Фаза | Съдържание | Готово, когато |
|------|-----------|----------------|
| **0. Скеле + сонда** | cargo workspace, Vite+React+Tailwind+shadcn, Tauri shell, platform.ts, ts-rs/tauri-specta bindings, CI (cargo test + clippy + vitest + tsc); жива сонда на diesel-async | празен прозорец + сондата минава срещу Pg |
| **1. Вертикален разрез** | Pg схема + миграции + seed (НАП, демо YT ООД); auth (argon2id, JWT) + фирми + табло; login → dashboard екрани; ServeDir за SPA-та | работещ екран в Tauri (срещу сървъра) и в браузър |
| **2. Номенклатури** | сметкоплан (seed без дублиране), контрагенти (ЕИК дедупликация), стоки (КН8), настройки на фирмата (обекти, собственици, серии) | паритет с docs/status.md по модулите |
| **3. Фактури + дневник** | чернова → номер → пост → черно сторно в една Diesel транзакция; универсална статия (количество × курс); печат PDF/DOCX/ODT, UBL | атомарен пост/сторно; тест за прекъсване |
| **4. ДДС** | дневници, файл за НАП, протоколи чл. 117, периоди/заключване, начални салда | затворен месец спира поста |
| **5. Операции** | банки (CSV/MT940/camt.053), правила за разнасяне, ДМА, склад (WAC/FIFO/LIFO), производство, дивиденти | паритет по модул |
| **6. SAF-T + справки** | SAF-T XML V1.0.1 (месец/година/при поискване), оборотна/главна книга/кореспонденции/баланс/ОПР/паричен поток, одитна следа | XSD валидацията минава |
| **7. Auth flows** | регистрация, забравена парола, 2FA (QR + резервни кодове), потребители/роли, VIES, ЕЦБ, сканиране (Mistral), UBL вход | пълен паритет с API договора |
| **8. Desktop + deploy** | tauri-specta команди през platform.ts (печат, файлове, store), настройка на адрес на сървъра, инсталатори deb/AppImage/msi; docker compose за web | инсталирано приложение + web deploy, пълен smoke test |

## Какво НЕ се пренася

- Octo хаковете: escape-backslash детекция, simple query, `MAX(id)+1`,
  ръчен pool с reconnect, ръчно триене при провален пост.
- Next.js SSR/middleware — SPA не ги нуждае; auth guard-ът е в клиента + API.
- Обикновеният CSS — заменя се от Tailwind + shadcn/ui.
- (От предишната версия на плана: SQLite, dual-backend Diesel, oplog/sync,
  uuidv7 — няма нужда без offline.)

## Рискове (честно)

1. **Обем:** ~33k реда Rust + ~40 екрана. Многоседмичен проект; фаза 1 е
   критичната — мине ли вертикалният разрез, останалото е механично пренасяне.
2. **Diesel миграцията е основната работа:** ~115+ низови SQL заявки стават
   DSL/проверени заявки. Бизнес логиката около тях остава непокътната.
3. **SAF-T XSD паритет** — задължителните възли (Payment, AssetTransaction,
   PhysicalStockEntry) зависят от данните; демо seed-ът трябва да ги покрива.
4. **Десетични суми** — никакъв float по пътя JSON ↔ Diesel ↔ Pg NUMERIC.
5. **Desktop = online винаги.** Няма offline режим — това е съзнателната
   граница, която държи проекта прост. Сървърът трябва да е достъпен.

## Проверки преди всяко качване

`cargo test` + `cargo clippy` + `vitest run` + `tsc --noEmit` + XSD валидация
на SAF-T — всички зелени. Сонда преди поправка, не след.