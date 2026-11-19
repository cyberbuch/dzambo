CREATE TABLE fs_line_formulas (
    id BIGSERIAL PRIMARY KEY,
    company_id BIGINT NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    statement TEXT NOT NULL,
    line_code TEXT NOT NULL,
    formula TEXT NOT NULL DEFAULT '',
    UNIQUE (company_id, statement, line_code)
);
