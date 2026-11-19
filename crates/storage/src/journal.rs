//! Дневник: статии и редове.

use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

pub use crate::models::journal::{JournalEntry, JournalLine, NewJournalEntry, NewJournalLine};

use crate::{
    schema::{journal_entries, journal_lines},
    StorageError,
};

pub type JournalRow = (JournalEntry, Vec<JournalLine>);

pub async fn list(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<JournalEntry>, StorageError> {
    journal_entries::table
        .filter(journal_entries::company_id.eq(company_id))
        .order(journal_entries::id.desc())
        .select(JournalEntry::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn entry_with_lines(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> Result<Option<JournalRow>, StorageError> {
    let Some(entry) = journal_entries::table
        .find(id)
        .select(JournalEntry::as_select())
        .first(conn)
        .await
        .optional()?
    else {
        return Ok(None);
    };
    let lines = journal_lines::table
        .filter(journal_lines::entry_id.eq(id))
        .order(journal_lines::id.asc())
        .select(JournalLine::as_select())
        .load(conn)
        .await?;
    Ok(Some((entry, lines)))
}

pub async fn insert_entry(
    conn: &mut AsyncPgConnection,
    new: &NewJournalEntry,
    lines: &[NewJournalLine],
) -> Result<i64, StorageError> {
    let id: i64 = diesel::insert_into(journal_entries::table)
        .values(new)
        .returning(journal_entries::id)
        .get_result(conn)
        .await?;
    for line in lines {
        diesel::insert_into(journal_lines::table)
            .values(NewJournalLine {
                entry_id: id,
                ..clone_line(line)
            })
            .execute(conn)
            .await?;
    }
    Ok(id)
}

fn clone_line(l: &NewJournalLine) -> NewJournalLine {
    NewJournalLine {
        entry_id: l.entry_id,
        account_id: l.account_id,
        account_number: l.account_number.clone(),
        account_name: l.account_name.clone(),
        direction: l.direction.clone(),
        amount: l.amount.clone(),
        vat_amount: l.vat_amount.clone(),
        currency: l.currency.clone(),
        fx_quantity: l.fx_quantity.clone(),
        fx_rate: l.fx_rate.clone(),
    }
}

/// Трие дневните статии на документ (при unpost/изтриване).
pub async fn delete_by_document(
    conn: &mut AsyncPgConnection,
    document_id: i64,
) -> Result<(), StorageError> {
    diesel::sql_query("DELETE FROM journal_lines WHERE entry_id IN (SELECT id FROM journal_entries WHERE document_id = $1)")
        .bind::<diesel::sql_types::BigInt, _>(document_id)
        .execute(conn)
        .await?;
    diesel::delete(journal_entries::table.filter(journal_entries::document_id.eq(document_id)))
        .execute(conn)
        .await?;
    Ok(())
}
