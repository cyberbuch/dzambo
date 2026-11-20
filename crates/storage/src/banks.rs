//! Банкови сметки и транзакции.

use bigdecimal::{BigDecimal, Zero};
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    schema::{bank_accounts, bank_transactions},
    StorageError,
};

#[derive(Queryable, Selectable, Insertable, Debug)]
#[diesel(table_name = bank_accounts)]
pub struct BankAccount {
    pub id: i64,
    pub company_id: i64,
    pub iban: String,
    pub bic: String,
    pub bank_name: String,
    pub currency: String,
    pub name: String,
    pub account_number: String,
    pub gl_account_id: i64,
    pub import_format: String,
    pub balance: BigDecimal,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = bank_transactions)]
pub struct BankTransaction {
    pub id: i64,
    pub company_id: i64,
    pub bank_account_id: i64,
    pub tx_date: NaiveDate,
    pub amount: BigDecimal,
    pub direction: String,
    pub counterpart_name: String,
    pub description: String,
    pub reference: String,
    pub status: String,
    pub journal_entry_id: i64,
    pub is_allocated: bool,
    pub account_id: i64,
    pub counterpart_id: i64,
}

pub async fn accounts(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<BankAccount>, StorageError> {
    bank_accounts::table
        .filter(bank_accounts::company_id.eq(company_id))
        .order(bank_accounts::id.asc())
        .select(BankAccount::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn create_account(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    name: &str,
    iban: &str,
    currency: &str,
) -> Result<BankAccount, StorageError> {
    diesel::insert_into(bank_accounts::table)
        .values((
            bank_accounts::company_id.eq(company_id),
            bank_accounts::name.eq(name),
            bank_accounts::iban.eq(iban),
            bank_accounts::currency.eq(currency),
        ))
        .returning(BankAccount::as_returning())
        .get_result(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn transactions(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    account_id: i64,
) -> Result<Vec<BankTransaction>, StorageError> {
    bank_transactions::table
        .filter(bank_transactions::company_id.eq(company_id))
        .filter(bank_transactions::bank_account_id.eq(account_id))
        .order(bank_transactions::tx_date.desc())
        .select(BankTransaction::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

/// CSV импорт: редове „дата;сума;посока;контрагент;описание;референция".
pub async fn import_csv(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    account_id: i64,
    csv: &str,
) -> Result<i64, StorageError> {
    let mut n = 0i64;
    for line in csv.lines() {
        let cols: Vec<&str> = line
            .split(';')
            .map(|c| c.trim().trim_matches('"'))
            .collect();
        if cols.len() < 2 {
            continue;
        }
        let Ok(date) = NaiveDate::parse_from_str(cols[0], "%Y-%m-%d") else {
            continue;
        };
        let amount: BigDecimal = cols[1].replace(',', ".").parse().unwrap_or_default();
        let direction = cols.get(2).copied().unwrap_or("in");
        let cp = cols.get(3).copied().unwrap_or("");
        let desc = cols.get(4).copied().unwrap_or("");
        let reference = cols.get(5).copied().unwrap_or("");
        diesel::insert_into(bank_transactions::table)
            .values((
                bank_transactions::company_id.eq(company_id),
                bank_transactions::bank_account_id.eq(account_id),
                bank_transactions::tx_date.eq(date),
                bank_transactions::amount.eq(amount),
                bank_transactions::direction.eq(direction),
                bank_transactions::counterpart_name.eq(cp),
                bank_transactions::description.eq(desc),
                bank_transactions::reference.eq(reference),
            ))
            .execute(conn)
            .await?;
        n += 1;
    }
    Ok(n)
}

pub async fn tx_by_id(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> Result<Option<BankTransaction>, StorageError> {
    bank_transactions::table
        .find(id)
        .select(BankTransaction::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}

pub async fn set_tx_status(
    conn: &mut AsyncPgConnection,
    id: i64,
    status: &str,
    entry_id: i64,
) -> Result<(), StorageError> {
    diesel::update(bank_transactions::table.find(id))
        .set((
            bank_transactions::status.eq(status),
            bank_transactions::journal_entry_id.eq(entry_id),
        ))
        .execute(conn)
        .await?;
    Ok(())
}

pub async fn account_by_id(
    conn: &mut AsyncPgConnection,
    id: i64,
) -> Result<Option<BankAccount>, StorageError> {
    bank_accounts::table
        .find(id)
        .select(BankAccount::as_select())
        .first(conn)
        .await
        .optional()
        .map_err(StorageError::from)
}

/// Импорт на вече парснати транзакции (MT940/camt.053).
pub async fn import_parsed(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    account_id: i64,
    txs: &[crate::bank_formats::Tx],
) -> Result<i64, StorageError> {
    let mut n = 0i64;
    for t in txs {
        let Ok(date) = NaiveDate::parse_from_str(&t.date, "%Y-%m-%d") else {
            continue;
        };
        let amount: BigDecimal = t.amount.parse().unwrap_or_default();
        if amount.is_zero() {
            continue;
        }
        let direction = if amount < BigDecimal::default() {
            "out"
        } else {
            "in"
        };
        diesel::insert_into(bank_transactions::table)
            .values((
                bank_transactions::company_id.eq(company_id),
                bank_transactions::bank_account_id.eq(account_id),
                bank_transactions::tx_date.eq(date),
                bank_transactions::amount.eq(amount),
                bank_transactions::direction.eq(direction),
                bank_transactions::counterpart_name.eq(&t.contra_name),
                bank_transactions::description.eq(&t.description),
                bank_transactions::reference.eq(&t.reference),
            ))
            .execute(conn)
            .await?;
        n += 1;
    }
    Ok(n)
}

/// Изтриване на банкова транзакция (+ дневника ѝ, ако е постната).
pub async fn delete_tx(
    conn: &mut AsyncPgConnection,
    company_id: i64,
    id: i64,
) -> Result<bool, StorageError> {
    let Some(tx) = tx_by_id(conn, id).await? else {
        return Ok(false);
    };
    if tx.company_id != company_id {
        return Ok(false);
    }
    if tx.journal_entry_id > 0 {
        diesel::delete(
            crate::schema::journal_lines::table
                .filter(crate::schema::journal_lines::entry_id.eq(tx.journal_entry_id)),
        )
        .execute(conn)
        .await?;
        diesel::delete(crate::schema::journal_entries::table.find(tx.journal_entry_id))
            .execute(conn)
            .await?;
    }
    let n = diesel::delete(
        bank_transactions::table
            .filter(bank_transactions::company_id.eq(company_id))
            .find(id),
    )
    .execute(conn)
    .await?;
    Ok(n > 0)
}

/// Неразнесени транзакции (is_allocated=false) за auto-allocate.
pub async fn list_unallocated(
    conn: &mut AsyncPgConnection,
    company_id: i64,
) -> Result<Vec<BankTransaction>, StorageError> {
    bank_transactions::table
        .filter(bank_transactions::company_id.eq(company_id))
        .filter(bank_transactions::is_allocated.eq(false))
        .order(bank_transactions::id.asc())
        .select(BankTransaction::as_select())
        .load(conn)
        .await
        .map_err(StorageError::from)
}

pub async fn mark_allocated(
    conn: &mut AsyncPgConnection,
    id: i64,
    account_id: i64,
    counterpart_id: i64,
) -> Result<(), StorageError> {
    diesel::update(bank_transactions::table.find(id))
        .set((
            bank_transactions::is_allocated.eq(true),
            bank_transactions::account_id.eq(account_id),
            bank_transactions::counterpart_id.eq(counterpart_id),
        ))
        .execute(conn)
        .await?;
    Ok(())
}

/// Референции на вече импортирани транзакции по сметка (за duplicate detection).
pub async fn refs_for_account(
    conn: &mut AsyncPgConnection,
    account_id: i64,
) -> Result<Vec<String>, StorageError> {
    bank_transactions::table
        .filter(bank_transactions::bank_account_id.eq(account_id))
        .select(bank_transactions::reference)
        .load(conn)
        .await
        .map_err(StorageError::from)
}
