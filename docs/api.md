# REST API

Base: `http://127.0.0.1:8095`. Всички пътища `/v1/...`. Auth: `Authorization: Bearer <token>`.

## Договори

- **Пагинация:** `{items, count, total, limit, offset, has_more}`; `limit` default 50, max 500.
- **Грешка:** `{"detail": "<текст>"}` + HTTP статус (400/401/403/404/409/422/502).
- **Сумите:** низове с десетична запетая (`"1234.56"`).

## Auth / профил

| Метод | Път | Описание |
|-------|-----|----------|
| POST | `/v1/auth/token` | вход `{sub\|email, password}` → `access_token` (или `mfa_token`) |
| POST | `/v1/auth/mfa` | втора факторна стъпка `{mfa_token, code}` → token |
| GET | `/v1/auth/registration-status` | `{registration_enabled}` |
| POST | `/v1/auth/register` | нова фирма + админ |
| POST | `/v1/auth/forgot-password` | токен за възстановяване |
| POST | `/v1/auth/reset-password` | смяна на парола |
| GET | `/v1/me` | профил + permissions за активната фирма |
| POST | `/v1/me/password` | смяна на парола |
| POST | `/v1/me/email` | смяна на email |
| POST | `/v1/me/totp/start\|confirm\|disable` | 2FA enrollment |

## Номенклатури

| Метод | Път |
|-------|-----|
| GET/POST/PATCH/DELETE | `/v1/counterparts[/{id}]`, `/v1/products[/{id}]`, `/v1/accounts`, `/v1/related/*` |
| GET | `/v1/nomenclatures?kind=`, `/v1/nomenclature-kinds` |
| GET | `/v1/vat-exemptions`, `/v1/cn8-suggest` (POST), `/v1/vies-lookup`, POST `/v1/counterparts/vies` |
| GET/POST | `/v1/product-name-mappings` |
| GET | `/v1/locations`, POST `/v1/locations` |

## Фактури / дневник

| Метод | Път |
|-------|-----|
| GET/POST | `/v1/invoices`, GET/PATCH/DELETE `/v1/invoices/{id}`, `/post`, `/unpost`, `/print`, `/ubl` |
| POST | `/v1/invoices/bulk/{action}` (post\|unpost\|delete) |
| GET/POST/PUT/DELETE | `/v1/journal`, DELETE `/v1/journal/bulk/delete` |
| POST | `/v1/fixed-assets/depreciation/preview`, `/post` |

## ДДС / SAF-T

| Метод | Път |
|-------|-----|
| GET | `/v1/vat/sales`, `/v1/vat/purchases`, `/v1/vat/registers?period` |
| PUT | `/v1/vat/manual?period` — ръчни клетки 33/70/71/80/81/82 |
| GET | `/v1/vat/export?period&type=pokupki\|prodagbi\|vies\|zip` — точен НАП файл |
| GET | `/v1/saft/export`, `/v1/saft/nomenclatures`, `/v1/saft/movement-mappings` |
| GET/POST/PATCH/DELETE | `/v1/saft/movement-mappings[/{id}]`, `/defaults` |

## Склад

| Метод | Път |
|-------|-----|
| GET/POST | `/v1/stock/rules[/{id}]`, `/v1/material-issues[/{id}]` |
| POST | `/v1/material-issues/{id}/confirm\|/unpost`, `/v1/stock/writeoff`, `/v1/stock/rebuild` |
| GET | `/v1/stock/report`, `/v1/inventory/at-location?location_id` |
| GET/POST | `/v1/internal-docs[/{id}]`, `/confirm`, `/print` |
| GET | `/v1/locations` |

## Банки

| Метод | Път |
|-------|-----|
| GET/POST | `/v1/bank-accounts`, `/v1/bank-transactions` |
| POST | `/v1/bank-transactions/import\|/preview\|/auto-allocate\|/auto-allocate/preview\|/bulk/delete` |
| GET/POST/DELETE | `/v1/bank-matching-rules[/{id}]`, `/defaults` |

## Справки

| Метод | Път |
|-------|-----|
| GET | `/v1/reports?kind=trial_balance\|counterpart_trial\|general_ledger\|chronological\|counterpart_chrono\|correspondence_ledger\|balance_sheet\|income_statement\|cash_flow\|equity\|stock&from&to&account&counterpart_id` |
| GET | `/v1/reports/export?kind&from&to&format=pdf\|xlsx\|ods&bg=0\|1` |

## Прочее

- `/v1/exchange-rates[/{rate|cross|import}]`
- `/v1/dividends[/{id}/pay]`, `/v1/dividend-distributions[/{id}]`
- `/v1/production-orders[/{id}/confirm]`, `/v1/recipes[/{id}]`
- `/v1/fixed-assets[...]` + `/revalue|/move|/conserve|/end-conserve|/dispose|/events`, `/fixed-asset-categories`
- `/v1/opening-balances`, `/v1/accounting-periods/close|reopen`, `/v1/closing-operations/preview|run`
- `/v1/dashboard`, `/v1/active-company`, `/v1/addresses/parse`
- `/v1/roles`, `/v1/users`, `/v1/system-settings[...]`, `/v1/audit-logs`
