//! Формули за финансови отчети — запис по фирма (каталогът е в domain::fsnom).

use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::schema::fs_line_formulas;
use crate::StorageResult;

#[derive(Queryable, Selectable, Insertable, Debug, Clone)]
#[diesel(table_name = fs_line_formulas)]
pub struct FsFormula {
    pub id: i64,
    pub company_id: i64,
    pub statement: String,
    pub line_code: String,
    pub formula: String,
}

pub async fn rows_of(
    conn: &mut crate::DbConn,
    company_id: i64,
    statement: &str,
) -> StorageResult<Vec<FsFormula>> {
    Ok(fs_line_formulas::table
        .filter(fs_line_formulas::company_id.eq(company_id))
        .filter(fs_line_formulas::statement.eq(statement))
        .order(fs_line_formulas::id.asc())
        .load(conn)
        .await?)
}

pub async fn upsert(
    conn: &mut crate::DbConn,
    company_id: i64,
    statement: &str,
    line_code: &str,
    formula: &str,
) -> StorageResult<FsFormula> {
    diesel::insert_into(fs_line_formulas::table)
        .values((
            fs_line_formulas::company_id.eq(company_id),
            fs_line_formulas::statement.eq(statement),
            fs_line_formulas::line_code.eq(line_code),
            fs_line_formulas::formula.eq(formula),
        ))
        .on_conflict((
            fs_line_formulas::company_id,
            fs_line_formulas::statement,
            fs_line_formulas::line_code,
        ))
        .do_update()
        .set(fs_line_formulas::formula.eq(formula))
        .get_result(conn)
        .await
        .map_err(Into::into)
}

pub async fn clear(
    conn: &mut crate::DbConn,
    company_id: i64,
    statement: &str,
) -> StorageResult<usize> {
    let n = diesel::delete(
        fs_line_formulas::table
            .filter(fs_line_formulas::company_id.eq(company_id))
            .filter(fs_line_formulas::statement.eq(statement)),
    )
    .execute(conn)
    .await?;
    Ok(n)
}
