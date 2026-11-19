//! Точен НАП файл: POKUPKI.TXT / PRODAGBI.TXT / DEKLAR.TXT / VIES + ZIP.
//! Фиксирана ширина, Windows-1251, CRLF, без заглавен ред — както ytbuch vat_files.

use serde_json::{json, Value};

fn cp1251(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for c in s.chars() {
        let u = c as u32;
        let b = match u {
            0x0410..=0x044F => (u - 0x0410 + 0xC0) as u8,
            0x0401 => 0xA8,
            0x0451 => 0xB8,
            0x2116 => 0xB9,
            0x201E => 0x84,
            0x201C => 0x93,
            0x201D => 0x94,
            0x201A => 0x82,
            0x2013 => 0x96,
            0x2014 => 0x97,
            0x20..=0x7E => u as u8,
            _ => b'?',
        };
        out.push(b);
    }
    out
}

fn pad_right(s: &str, w: usize) -> Vec<u8> {
    let mut b = cp1251(s);
    b.truncate(w);
    while b.len() < w {
        b.push(b' ');
    }
    b
}

fn pad_left(s: &str, w: usize) -> Vec<u8> {
    let mut b = cp1251(s);
    if b.len() > w {
        b = b[b.len() - w..].to_vec();
    }
    let mut out = vec![b' '; w - b.len()];
    out.extend(b);
    out
}

fn money(v: f64) -> String {
    format!("{:.2}", v)
}

fn num(v: f64, w: usize) -> Vec<u8> {
    pad_left(&money(v), w)
}

fn txt(s: &str, w: usize) -> Vec<u8> {
    pad_right(s, w)
}

#[derive(Default, Clone)]
pub struct Line {
    pub doc_type: String,
    pub number: String,
    pub date: String, // DD/MM/YYYY
    pub cin: String,
    pub name: String,
    pub goods: String,
    pub delivery: String,
    // продажби
    pub s_tb: f64,
    pub s_tv: f64,
    pub s_b20: f64,
    pub s_v20: f64,
    pub s_vopb: f64,
    pub s_82: f64,
    pub s_vopv: f64,
    pub s_pers: f64,
    pub s_b9: f64,
    pub s_v9: f64,
    pub s_ch3: f64,
    pub s_vod: f64,
    pub s_art: f64,
    pub s_svc: f64,
    pub s_69: f64,
    pub s_ex: f64,
    pub s_tri: f64,
    // покупки
    pub p_no: f64,
    pub p_fb: f64,
    pub p_fv: f64,
    pub p_pb: f64,
    pub p_pv: f64,
    pub p_adj: f64,
    pub p_vop: f64,
}

fn rate_kind(rate: f64) -> &'static str {
    let cents = (rate * 100.0).round() as i64;
    match cents {
        0 => "0",
        900 => "9",
        _ => "20",
    }
}

fn zero_bucket(reason: &str) -> &'static str {
    match reason {
        "VATEX-EU-IC" => "vod",
        "VATEX-EU-Z-173" => "art",
        "VATEX-EU-S-21" => "svc",
        _ => "ex",
    }
}

impl Line {
    pub fn add_sale(&mut self, base: f64, vat: f64, rate: f64, reason: &str) {
        self.s_tb += base;
        self.s_tv += vat;
        match rate_kind(rate) {
            "9" => {
                self.s_b9 += base;
                self.s_v9 += vat;
            }
            "0" => match zero_bucket(reason) {
                "vod" => self.s_vod += base,
                "art" => self.s_art += base,
                "svc" => self.s_svc += base,
                "ch3" => self.s_ch3 += base,
                _ => self.s_ex += base,
            },
            _ => {
                self.s_b20 += base;
                self.s_v20 += vat;
            }
        }
    }

    pub fn add_purch(&mut self, base: f64, vat: f64) {
        if vat == 0.0 {
            self.p_no += base;
        } else {
            self.p_fb += base;
            self.p_fv += vat;
        }
    }

    pub fn flip_if_credit(&mut self) {
        if !matches!(self.doc_type.as_str(), "03" | "13" | "23" | "29") {
            return;
        }
        let fields = [
            &mut self.s_tb,
            &mut self.s_tv,
            &mut self.s_b20,
            &mut self.s_v20,
            &mut self.s_vopb,
            &mut self.s_82,
            &mut self.s_vopv,
            &mut self.s_pers,
            &mut self.s_b9,
            &mut self.s_v9,
            &mut self.s_ch3,
            &mut self.s_vod,
            &mut self.s_art,
            &mut self.s_svc,
            &mut self.s_69,
            &mut self.s_ex,
            &mut self.s_tri,
            &mut self.p_no,
            &mut self.p_fb,
            &mut self.p_fv,
            &mut self.p_pb,
            &mut self.p_pv,
            &mut self.p_adj,
            &mut self.p_vop,
        ];
        let any_neg = fields.iter().any(|v| **v < 0.0);
        if any_neg {
            return;
        }
        for v in fields {
            *v = -*v;
        }
    }
}

pub struct Packet {
    pub period_nap: String,
    pub company_vat: String,
    pub company_name: String,
    pub branch: String,
    pub submitter: String,
    pub sales: Vec<Line>,
    pub purchases: Vec<Line>,
}

fn header(p: &Packet, row: usize) -> Vec<u8> {
    let mut b = txt(&p.company_vat, 15);
    b.extend(txt(&p.period_nap, 6));
    b.extend(pad_left(
        if p.branch.is_empty() { "0" } else { &p.branch },
        4,
    ));
    b.extend(pad_left(&row.to_string(), 15));
    b
}

fn common(p: &Packet, l: &Line, row: usize) -> Vec<u8> {
    let mut b = header(p, row);
    b.extend(txt(&l.doc_type, 2));
    b.extend(txt(&l.number, 20));
    b.extend(txt(&l.date, 10));
    b.extend(txt(&l.cin, 15));
    b.extend(txt(&l.name, 50));
    b.extend(txt(&l.goods, 30));
    b
}

pub fn pokupki(p: &Packet) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, l) in p.purchases.iter().enumerate() {
        let mut b = common(p, l, i + 1);
        b.extend(num(l.p_no, 15));
        b.extend(num(l.p_fb, 15));
        b.extend(num(l.p_fv, 15));
        b.extend(num(l.p_pb, 15));
        b.extend(num(l.p_pv, 15));
        b.extend(num(l.p_adj, 15));
        b.extend(num(l.p_vop, 15));
        b.extend(txt(&l.delivery, 2));
        b.extend([13, 10]);
        out.extend(b);
    }
    out
}

pub fn prodagbi(p: &Packet) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, l) in p.sales.iter().enumerate() {
        let mut b = common(p, l, i + 1);
        for v in [
            l.s_tb, l.s_tv, l.s_b20, l.s_v20, l.s_vopb, l.s_82, l.s_vopv, l.s_pers, l.s_b9, l.s_v9,
            l.s_ch3, l.s_vod, l.s_art, l.s_svc, l.s_69, l.s_ex, l.s_tri,
        ] {
            b.extend(num(v, 15));
        }
        b.extend(txt(&l.delivery, 2));
        b.extend([13, 10]);
        out.extend(b);
    }
    out
}

pub struct Cells {
    pub s01: f64,
    pub s20: f64,
    pub s11: f64,
    pub s21: f64,
    pub s12: f64,
    pub s22: f64,
    pub s23: f64,
    pub s13: f64,
    pub s24: f64,
    pub s14: f64,
    pub s15: f64,
    pub s16: f64,
    pub s17: f64,
    pub s18: f64,
    pub s19: f64,
    pub p30: f64,
    pub p31: f64,
    pub p41: f64,
    pub p32: f64,
    pub p42: f64,
    pub p43: f64,
    pub r40: f64,
    pub r50: f64,
    pub r60: f64,
}

pub fn cells(p: &Packet) -> Cells {
    let mut c = Cells {
        s01: 0.0,
        s20: 0.0,
        s11: 0.0,
        s21: 0.0,
        s12: 0.0,
        s22: 0.0,
        s23: 0.0,
        s13: 0.0,
        s24: 0.0,
        s14: 0.0,
        s15: 0.0,
        s16: 0.0,
        s17: 0.0,
        s18: 0.0,
        s19: 0.0,
        p30: 0.0,
        p31: 0.0,
        p41: 0.0,
        p32: 0.0,
        p42: 0.0,
        p43: 0.0,
        r40: 0.0,
        r50: 0.0,
        r60: 0.0,
    };
    for l in &p.sales {
        c.s01 += l.s_tb;
        c.s20 += l.s_tv;
        c.s11 += l.s_b20;
        c.s21 += l.s_v20;
        c.s12 += l.s_vopb + l.s_82;
        c.s13 += l.s_b9;
        c.s24 += l.s_v9;
        c.s14 += l.s_vod;
        c.s15 += l.s_ch3;
        c.s16 += l.s_art;
        c.s17 += l.s_svc;
        c.s18 += l.s_69 + l.s_tri;
        c.s19 += l.s_ex;
    }
    for l in &p.purchases {
        c.p30 += l.p_no + l.p_vop;
        c.p31 += l.p_fb;
        c.p41 += l.p_fv;
        c.p32 += l.p_pb;
        c.p42 += l.p_pv;
        c.p43 += l.p_adj;
    }
    c.r40 = c.p41 + c.p43;
    c.r50 = (c.s20 - c.r40).max(0.0);
    c.r60 = (c.r40 - c.s20).max(0.0);
    c
}

pub fn deklar(p: &Packet, manual: &Value) -> Vec<u8> {
    let c = cells(p);
    let mut b = txt(&p.company_vat, 15);
    b.extend(txt(&p.company_name, 50));
    b.extend(txt(&p.period_nap, 6));
    b.extend(txt(&p.submitter, 50));
    b.extend(pad_left(&p.sales.len().to_string(), 15));
    b.extend(pad_left(&p.purchases.len().to_string(), 15));
    for v in [
        c.s01, c.s20, c.s11, c.s21, c.s12, c.s22, c.s23, c.s13, c.s24, c.s14, c.s15, c.s16, c.s17,
        c.s18, c.s19, c.p30, c.p31, c.p41, c.p32, c.p42, c.p43,
    ] {
        b.extend(num(v, 15));
    }
    let k33 = manual["cell_33"].as_str().unwrap_or("0.00");
    b.extend(pad_right(if k33.is_empty() { "0.00" } else { k33 }, 4));
    let m = |k: &str, d: f64| {
        manual[k]
            .as_str()
            .and_then(|s| s.replace(',', ".").parse::<f64>().ok())
            .unwrap_or(d)
    };
    for v in [
        c.r40,
        c.r50,
        c.r60,
        m("cell_70", 0.0),
        m("cell_71", c.p41),
        m("cell_80", 0.0),
        m("cell_81", c.r50),
        m("cell_82", 0.0),
    ] {
        b.extend(num(v, 15));
    }
    b.extend([13, 10]);
    b
}

pub fn vies(p: &Packet, partners: &[(String, String, f64, f64, f64)]) -> Vec<u8> {
    let mm_yyyy = format!("{}/{}", &p.period_nap[4..6], &p.period_nap[0..4]);
    let mut out = Vec::new();
    let mut h = txt("VHR", 3);
    h.extend(txt(&mm_yyyy, 7));
    h.extend(pad_left(&partners.len().to_string(), 5));
    h.extend([13, 10]);
    out.extend(h);
    let mut d = txt("VDR", 3);
    d.extend(txt(&p.company_vat, 15));
    d.extend(txt(&p.company_name, 150));
    d.extend(txt("", 50));
    d.extend(txt("", 4));
    d.extend(txt("", 150));
    d.extend(txt("R", 1));
    d.extend([13, 10]);
    out.extend(d);
    let mut t = txt("VTR", 3);
    t.extend(txt(&p.company_vat, 15));
    t.extend(txt(&p.company_name, 150));
    t.extend(txt("", 200));
    t.extend([13, 10]);
    out.extend(t);
    let total: f64 = partners.iter().map(|x| x.2 + x.3 + x.4).sum();
    let vod: f64 = partners.iter().map(|x| x.2).sum();
    let mut tt = txt("TTR", 3);
    tt.extend(pad_left(&money(total), 12));
    tt.extend(pad_left(&money(vod), 12));
    tt.extend([13, 10]);
    out.extend(tt);
    for (i, (vin, _name, k3, k4, k5)) in partners.iter().enumerate() {
        let mut v = txt("VIR", 3);
        v.extend(pad_left(&(i + 1).to_string(), 5));
        v.extend(txt(vin, 15));
        v.extend(pad_left(&money(*k3), 12));
        v.extend(pad_left(&money(*k4), 12));
        v.extend(pad_left(&money(*k5), 12));
        v.extend(txt("", 7));
        v.extend([13, 10]);
        out.extend(v);
    }
    out
}

// ---- ZIP (stored, method 0) ----

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

pub fn zip(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let nlen = name.len() as u16;
        let size = data.len() as u32;
        out.extend([0x50, 0x4b, 0x03, 0x04]);
        out.extend(20u16.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out.extend(0u16.to_le_bytes()); // stored
        out.extend(0u16.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out.extend(crc.to_le_bytes());
        out.extend(size.to_le_bytes());
        out.extend(size.to_le_bytes());
        out.extend(nlen.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out.extend(name.as_bytes());
        out.extend(data);
        central.extend([0x50, 0x4b, 0x01, 0x02]);
        central.extend(20u16.to_le_bytes());
        central.extend(20u16.to_le_bytes());
        central.extend(0u16.to_le_bytes());
        central.extend(0u16.to_le_bytes());
        central.extend(0u16.to_le_bytes());
        central.extend(0u16.to_le_bytes());
        central.extend(crc.to_le_bytes());
        central.extend(size.to_le_bytes());
        central.extend(size.to_le_bytes());
        central.extend(nlen.to_le_bytes());
        central.extend(0u16.to_le_bytes());
        central.extend(0u16.to_le_bytes());
        central.extend(0u16.to_le_bytes());
        central.extend(0u16.to_le_bytes());
        central.extend(0u32.to_le_bytes());
        central.extend(offset.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    out.extend(central);
    out.extend([0x50, 0x4b, 0x05, 0x06]);
    out.extend(0u16.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out.extend((files.len() as u16).to_le_bytes());
    out.extend((files.len() as u16).to_le_bytes());
    out.extend(cd_size.to_le_bytes());
    out.extend(cd_offset.to_le_bytes());
    out.extend(0u16.to_le_bytes());
    out
}

pub fn packet_json(p: &Packet) -> Value {
    let c = cells(p);
    json!({
        "period_nap": p.period_nap, "company_name": p.company_name, "company_vat": p.company_vat,
        "submitter": p.submitter, "sales_count": p.sales.len(), "purchase_count": p.purchases.len(),
        "cells": {"01": c.s01, "20": c.s20, "11": c.s11, "21": c.s21, "30": c.p30, "31": c.p31, "41": c.p41, "50": c.r50, "60": c.r60},
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packet() -> Packet {
        let mut sale = Line {
            doc_type: "01".into(),
            number: "42".into(),
            date: "02/10/2026".into(),
            cin: "BG987654321".into(),
            name: "Клиент Демо ООД".into(),
            goods: "Стоки".into(),
            ..Default::default()
        };
        sale.add_sale(1000.0, 200.0, 20.0, "");
        let mut purch = Line {
            doc_type: "01".into(),
            number: "7".into(),
            date: "01/10/2026".into(),
            cin: "BG123456789".into(),
            name: "Доставчик Демо ООД".into(),
            goods: "Материали".into(),
            ..Default::default()
        };
        purch.add_purch(500.0, 100.0);
        Packet {
            period_nap: "202610".into(),
            company_vat: "BG123456789".into(),
            company_name: "Демо ООД".into(),
            branch: "0".into(),
            submitter: "Иван Иванов".into(),
            sales: vec![sale],
            purchases: vec![purch],
        }
    }

    #[test]
    fn pokupki_row_width() {
        let b = pokupki(&packet());
        assert_eq!(b.len(), 276);
        assert_eq!(&b[274..], &[13, 10]);
    }

    #[test]
    fn prodagbi_row_width() {
        let b = prodagbi(&packet());
        assert_eq!(b.len(), 426);
        assert_eq!(&b[424..], &[13, 10]);
    }

    #[test]
    fn deklar_row_width() {
        let b = deklar(&packet(), &json!({}));
        assert_eq!(b.len(), 592);
        assert_eq!(&b[590..], &[13, 10]);
    }

    #[test]
    fn cp1251_cyrillic() {
        assert_eq!(cp1251("А"), vec![0xC0]);
        assert_eq!(cp1251("я"), vec![0xFF]);
        assert_eq!(cp1251("№"), vec![0xB9]);
    }

    #[test]
    fn credit_note_flips_once() {
        let mut l = Line {
            doc_type: "03".into(),
            ..Default::default()
        };
        l.add_sale(100.0, 20.0, 20.0, "");
        l.flip_if_credit();
        assert_eq!(l.s_tb, -100.0);
        l.flip_if_credit();
        assert_eq!(l.s_tb, -100.0);
    }

    #[test]
    fn zip_stored_structure() {
        let z = zip(&[("A.TXT", b"hello".to_vec())]);
        assert_eq!(&z[0..4], &[0x50, 0x4b, 0x03, 0x04]);
        assert!(z.windows(4).any(|w| w == [0x50, 0x4b, 0x05, 0x06]));
    }
}
