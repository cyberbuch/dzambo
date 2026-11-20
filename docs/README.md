# dzambo — документация

dzambo е пренаписаната счетоводна система **ytbuch** върху нов стек:
Rust + Axum 0.8 + Diesel + PostgreSQL, с оригиналния Next.js 16 фронт и
Tauri v2 desktop обвивка. Една база, без SQLite/sync; desktop-ът е тънък
клиент към сървъра.

Документите:

| Файл | Съдържание |
|------|-----------|
| [architecture.md](architecture.md) | Стек, слоеве, модули, request flow, комуникация desktop↔web↔server |
| [api.md](api.md) | REST API по модули (endpoint групи), договори, пагинация, грешки |
| [data-model.md](data-model.md) | PostgreSQL схема, 24 миграции, ключови таблици, конвенции за типове |
| [business-rules.md](business-rules.md) | Счетоводни правила на домейна (черно сторно, ДДС, склад, SAF-T, справки) |
| [deployment.md](deployment.md) | Web deploy (docker compose) + desktop AppImage |
| [development.md](development.md) | Dev setup, батерии, конвенции, структура |
| [migration-parity.md](migration-parity.md) | Миграция ytbuch→dzambo: решения, паритет, ограничения |

## Кратко за проекта

- **Пълно покритие:** всички `/v1/` пътища, които оригиналният фронт вика, имат backend маршрут.
- **SAF-T:** XML V1.0.1, три режима (месец/година/при поискване), мапинг на свободен сметкоплан към НАП номенклатури.
- **Точен НАП файл:** POKUPKI/PRODAGBI/DEKLAR/VIES + ZIP, фиксирана ширина, Windows-1251, CRLF.
- **Reports:** оборотна (6 колони), главна книга, хронологичен, кореспонденции, баланс/ОПР/паричен поток/капитал, склад — всички с export pdf/xlsx/ods.

## Първи стъпки

```bash
docker compose up --build
# UI: http://localhost:3022  ·  API: http://localhost:8095
# login: superadmin / dzambo-admin-dev
```

Desktop (AppImage) — вж. [deployment.md](deployment.md).
