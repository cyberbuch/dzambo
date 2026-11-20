ALTER TABLE bank_transactions DROP COLUMN IF EXISTS counterpart_id;
ALTER TABLE bank_transactions DROP COLUMN IF EXISTS account_id;
ALTER TABLE bank_transactions DROP COLUMN IF EXISTS is_allocated;
