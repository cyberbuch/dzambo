# Модел на данните

Една PostgreSQL база (контейнер `dzambo-postgres`, :5434; user/pass/db `dzambo`).
Миграциите са в `crates/storage/migrations/` (24 последователни), embedded в
сървърния бинар (`embed_migrations!`) и се прилагат автоматично при старт.

## Конвенции за типове

| Домейн | Тип | Бележка |
|--------|-----|---------|
| id | BIGINT GENERATED ALWAYS AS IDENTITY | |
| пари/количества | NUMERIC(18,4)/(18,6) | rust BigDecimal; JSON — низ |
| време | TIMESTAMPTZ / DATE | |
| флагове | BOOLEAN | |
| изтриване | ON DELETE CASCADE по FK | |

- Няма float по пътя JSON ↔ Diesel ↔ Pg.
- `company_id` дели данните по фирма; активната фирма се резолва от потребителя.
- Справките работят със `i128` в стотинки (scale 100).

## Миграции

```
0001_spike           първоначална схема (сонда)
0002_core            основни таблици (companies, accounts, users, ...)
0003_nomenclatures   номенклатури НАП
0004_invoices        фактури
0005_opening         начални салда
0006_banks           банки
0007_bank_rules      правила за разнасяне
0008_fixed_assets    ДМА
0009_stock           партиди (inventory_lots)
0010_stock_docs      складови документи
0011_dividends       дивиденти
0012_production      производство
0013_idempotency     идемпотентност (client_key)
0014_invoice_fields  допълнителни полета на фактурата
0015_access_stock    роли, членства, одит, обекти, вътрешни документи, изписвания, правила за склад
0016_auth_flows      password_reset_tokens
0017_exchange_rates   курсове на ЕЦБ (+ BGN peg)
0018_fs_formulas     формули за финансови отчети
0019_saft_mappings   Дт/Кт кореспонденции SAF-T
0020_totp            2FA (totp_secret/pending, mfa_*, recovery_codes)
0021_asset_ops       събития на ДМА (prev_value)
0022_product_name_mappings
0023_bank_allocate   is_allocated/account_id/counterpart_id
0024_vat_manual      ръчни клетки на ДДС декларацията
```

## Ключови таблици

- **Фирми:** `companies`, `company_details` (1:1), `users`, `roles`, `user_companies`.
- **Номенклатури:** `accounts` (с `saft_account_type`), `counterparts`, `products`
  (+ `product_extra`), `beneficial_owners`, `locations`.
- **Документи:** `invoices` (+ `invoice_lines`), `journal_entries` (+ `journal_lines`),
  `idempotency_keys`.
- **ДДС:** `vat_manual` (ръчни клетки).
- **Склад:** `inventory_lots`, `stock_documents`, `internal_docs(+lines)`,
  `material_issues(+lines)`, `stock_account_rules`, `fixed_assets`.
- **Банки:** `bank_accounts`, `bank_transactions`, `bank_matching_rules`.
- **SAF-T:** `saft_account_mappings`.
- **Финансови отчети:** `fs_line_formulas` (по фирма).
- **Дивиденти:** `dividend_distributions` (+ `dividends` — share-ове).
- **Производство:** `recipes`, `production_orders`.
- **Одит:** `audit_logs`.
- **Настройки:** `settings` (key/value), `exchange_rates`.

## Seed

При старт: НАП номенклатури, демо фирма YT ООД, супер администратор, роли
(„Администратор на фирма“, „Счетоводител“, „Фактурист“), банкови правила по
подразбиране, BGN peg (1.95583).
