//! Зареждане на Books за справките от дневника, сметките и началните салда.

use bigdecimal::BigDecimal;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use std::collections::HashSet;

use crate::schema::{accounts, companies, journal_entries, journal_lines, opening_balances};
use crate::StorageResult;
use domain::reports_data::{Acc, Books, Entry, Line, Open};

/// BigDecimal → стотинки (i128), закръглено до 2 знака.
fn cents(v: &BigDecimal) -> i128 {
    format!("{:.0}", v * BigDecimal::from(100))
        .parse()
        .unwrap_or(0)
}

pub async fn load(
    conn: &mut crate::DbConn,
    company_id: i64,
    from: &str,
    to: &str,
    account: &str,
    cpid: i64,
) -> StorageResult<Books> {
    let year: i32 = from.get(..4).unwrap_or("0").parse().unwrap_or(0);
    let firm: Option<(String, String)> = companies::table
        .find(company_id)
        .select((companies::name, companies::eik))
        .first(conn)
        .await
        .optional()?;
    let (firm_name, firm_eik) = firm.unwrap_or_default();

    let acc_rows: Vec<(i64, String, String)> = accounts::table
        .filter(accounts::company_id.eq(company_id))
        .order((accounts::number.asc(), accounts::id.asc()))
        .select((accounts::id, accounts::number, accounts::name))
        .load(conn)
        .await?;
    let accounts: Vec<Acc> = acc_rows
        .into_iter()
        .map(|(id, number, name)| Acc { id, number, name })
        .collect();
    let ids: HashSet<i64> = accounts.iter().map(|a| a.id).collect();

    let entry_rows: Vec<(i64, chrono::NaiveDate, String, String, i64, String)> =
        journal_entries::table
            .filter(journal_entries::company_id.eq(company_id))
            .order((journal_entries::entry_date.asc(), journal_entries::id.asc()))
            .select((
                journal_entries::id,
                journal_entries::entry_date,
                journal_entries::document_number,
                journal_entries::description,
                journal_entries::counterpart_id,
                journal_entries::counterpart_name,
            ))
            .load(conn)
            .await?;
    let entries: Vec<Entry> = entry_rows
        .into_iter()
        .map(|(id, day, doc, desc, cp_id, cp_name)| Entry {
            id,
            day: day.to_string(),
            doc,
            desc,
            cp_id,
            cp_name,
        })
        .collect();
    let entry_ids: HashSet<i64> = entries.iter().map(|e| e.id).collect();

    let line_rows: Vec<(i64, i64, String, BigDecimal)> = journal_lines::table
        .order(journal_lines::id.asc())
        .select((
            journal_lines::entry_id,
            journal_lines::account_id,
            journal_lines::direction,
            journal_lines::amount,
        ))
        .load(conn)
        .await?;
    let lines: Vec<Line> = line_rows
        .into_iter()
        .filter(|(eid, _, _, _)| entry_ids.contains(eid))
        .map(|(entry_id, account_id, dir, amount)| Line {
            entry_id,
            account_id,
            debit: dir == "debit",
            amount: cents(&amount),
        })
        .collect();

    let open_rows: Vec<(i64, BigDecimal, BigDecimal)> = opening_balances::table
        .filter(opening_balances::company_id.eq(company_id))
        .filter(opening_balances::year.eq(year))
        .order(opening_balances::id.asc())
        .select((
            opening_balances::account_id,
            opening_balances::debit,
            opening_balances::credit,
        ))
        .load(conn)
        .await?;
    let openings: Vec<Open> = open_rows
        .into_iter()
        .filter(|(aid, _, _)| ids.contains(aid))
        .map(|(account_id, debit, credit)| Open {
            account_id,
            cp_id: 0,
            debit: cents(&debit),
            credit: cents(&credit),
        })
        .collect();

    Ok(Books {
        company_id,
        accounts,
        entries,
        lines,
        openings,
        from: from.to_string(),
        to: to.to_string(),
        fiscal: format!("{year:04}-01-01"),
        account: account.to_string(),
        cpid,
        firm_name,
        firm_eik,
    })
}
