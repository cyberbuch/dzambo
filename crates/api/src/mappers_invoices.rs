//! Фактури и дневник → DTO (суми като низове).

use domain::dto::{InvoiceDto, InvoiceLineDto, JournalEntryDto, JournalLineDto};
use storage::{invoices::InvoiceRow, journal::JournalRow};

fn money(v: &bigdecimal::BigDecimal) -> String {
    format!("{v:.2}")
}

fn qty(v: &bigdecimal::BigDecimal) -> String {
    format!("{v:.4}")
}

pub fn invoice_line_dto(l: &storage::models::invoices::InvoiceLine) -> InvoiceLineDto {
    InvoiceLineDto {
        id: l.id,
        code: l.code.clone(),
        description: l.description.clone(),
        quantity: Some(qty(&l.quantity)),
        unit: l.unit.clone(),
        unit_price: Some(money(&l.unit_price)),
        vat_rate: Some(format!("{:.2}", l.vat_rate)),
        net_amount: Some(money(&l.net_amount)),
        vat_amount: Some(money(&l.vat_amount)),
        total_amount: Some(money(&l.total_amount)),
        product_id: l.product_id,
    }
}

pub fn invoice_dto(row: &InvoiceRow) -> InvoiceDto {
    let h = &row.0;
    InvoiceDto {
        id: h.id,
        direction: h.direction.clone(),
        document_type: h.document_type.clone(),
        number: h.number.clone(),
        issue_date: h.issue_date.to_string(),
        due_date: h.due_date.map(|d| d.to_string()).unwrap_or_default(),
        counterpart_id: h.counterpart_id,
        counterpart_name: h.counterpart_name.clone(),
        currency: h.currency.clone(),
        currency_rate: Some(format!("{}", h.currency_rate)),
        status: h.status.clone(),
        notes: h.notes.clone(),
        net_amount: Some(money(&h.net_amount)),
        vat_amount: Some(money(&h.vat_amount)),
        total_amount: Some(money(&h.total_amount)),
        accounting_month: h.accounting_month.clone(),
        original_invoice_id: h.original_invoice_id,
        lines: row.1.iter().map(invoice_line_dto).collect(),
    }
}

pub fn invoice_header_dto(h: &storage::models::invoices::Invoice) -> InvoiceDto {
    InvoiceDto {
        id: h.id,
        direction: h.direction.clone(),
        document_type: h.document_type.clone(),
        number: h.number.clone(),
        issue_date: h.issue_date.to_string(),
        due_date: h.due_date.map(|d| d.to_string()).unwrap_or_default(),
        counterpart_id: h.counterpart_id,
        counterpart_name: h.counterpart_name.clone(),
        currency: h.currency.clone(),
        currency_rate: Some(format!("{}", h.currency_rate)),
        status: h.status.clone(),
        notes: h.notes.clone(),
        net_amount: Some(money(&h.net_amount)),
        vat_amount: Some(money(&h.vat_amount)),
        total_amount: Some(money(&h.total_amount)),
        accounting_month: h.accounting_month.clone(),
        original_invoice_id: h.original_invoice_id,
        lines: Vec::new(),
    }
}

pub fn journal_line_dto(l: &storage::models::journal::JournalLine) -> JournalLineDto {
    JournalLineDto {
        id: l.id,
        account_id: l.account_id,
        account_number: l.account_number.clone(),
        account_name: l.account_name.clone(),
        direction: l.direction.clone(),
        amount: money(&l.amount),
        vat_amount: money(&l.vat_amount),
        currency: l.currency.clone(),
        fx_quantity: qty(&l.fx_quantity),
        fx_rate: format!("{}", l.fx_rate),
    }
}

pub fn journal_dto(row: &JournalRow) -> JournalEntryDto {
    let e = &row.0;
    JournalEntryDto {
        id: e.id,
        entry_date: e.entry_date.to_string(),
        document_type: e.document_type.clone(),
        document_id: e.document_id,
        document_number: e.document_number.clone(),
        description: e.description.clone(),
        vat_type: e.vat_type.clone(),
        counterpart_id: e.counterpart_id,
        counterpart_name: e.counterpart_name.clone(),
        document_date: e.document_date.to_string(),
        vat_period: e.vat_period.clone(),
        lines: row.1.iter().map(journal_line_dto).collect(),
    }
}
