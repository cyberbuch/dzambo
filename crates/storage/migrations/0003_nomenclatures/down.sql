DROP TABLE IF EXISTS document_series;
DROP TABLE IF EXISTS ultimate_parents;
DROP TABLE IF EXISTS beneficial_owners;
DROP TABLE IF EXISTS company_locations;
DROP TABLE IF EXISTS product_extra;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS counterpart_extra;
DROP TABLE IF EXISTS counterparts;
ALTER TABLE accounts DROP COLUMN IF EXISTS is_currency;
