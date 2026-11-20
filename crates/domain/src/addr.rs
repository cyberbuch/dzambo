//! Разделяне на адрес за SAF-T (локална евристика). Порт на ytbuch addr::split_local.

use serde_json::{json, Value};

pub struct Parts {
    pub street_name: String,
    pub building_number: String,
    pub address_building: String,
    pub additional_address_detail: String,
    pub city: String,
    pub post_code: String,
    pub region: String,
    pub country: String,
    pub address_type: String,
    pub ok: bool,
}

impl Parts {
    fn empty(country: &str) -> Self {
        Self {
            street_name: String::new(),
            building_number: String::new(),
            address_building: String::new(),
            additional_address_detail: String::new(),
            city: String::new(),
            post_code: String::new(),
            region: String::new(),
            country: country.to_string(),
            address_type: String::new(),
            ok: false,
        }
    }
}

pub fn parts_json(p: &Parts, note: &str) -> Value {
    json!({
        "street_name": p.street_name, "building_number": p.building_number,
        "address_building": p.address_building, "additional_address_detail": p.additional_address_detail,
        "city": p.city, "post_code": p.post_code, "region": p.region, "country": p.country,
        "address_type": p.address_type, "parse_ok": p.ok, "parse_note": note,
    })
}

fn region_of_city(city: &str) -> String {
    let want = city.trim().to_lowercase();
    if want.is_empty() {
        return String::new();
    }
    for line in crate::noms::entries("regions") {
        let mut p = line.splitn(2, '|');
        let code = p.next().unwrap_or("");
        let name = p.next().unwrap_or("");
        if name.trim().to_lowercase() == want {
            return code.to_string();
        }
    }
    String::new()
}

fn take_digits(token: &str) -> bool {
    token.chars().next().is_some_and(|c| c.is_ascii_digit())
}

pub fn split_local(raw: &str, country_hint: &str, city_hint: &str, post_hint: &str) -> Parts {
    let country = {
        let c = country_hint.trim().to_ascii_uppercase();
        if c.is_empty() {
            "BG".into()
        } else {
            c
        }
    };
    let text = raw.trim();
    if text.is_empty() || text == "---" {
        return Parts::empty(&country);
    }
    let mut lines: Vec<String> = text
        .replace('\r', "")
        .split(['\n', ','])
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|s| !s.is_empty())
        .collect();
    if let Some(last) = lines.last().cloned() {
        let up = last.to_ascii_uppercase();
        if up == "BG"
            || up == "BULGARIA"
            || last == "България"
            || (up.len() == 2 && up.chars().all(|c| c.is_ascii_alphabetic()))
        {
            lines.pop();
        }
    }
    let mut post = post_hint.trim().to_string();
    let mut city = city_hint.trim().to_string();
    let mut kept = Vec::new();
    for line in lines {
        let words: Vec<&str> = line.split_whitespace().collect();
        if post.is_empty() {
            if let Some(pos) = words
                .iter()
                .position(|w| w.len() == 4 && w.chars().all(|c| c.is_ascii_digit()))
            {
                post = words[pos].to_string();
                let after = words[pos + 1..].join(" ");
                if city.is_empty() && !after.is_empty() {
                    city = after;
                }
                let before = words[..pos].join(" ");
                if !before.is_empty() {
                    kept.push(before);
                }
                continue;
            }
        }
        kept.push(line);
    }
    let street_line = kept.first().cloned().unwrap_or_default();
    let mut words: Vec<String> = street_line
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    if words.first().is_some_and(|w| {
        let l = w.trim_end_matches('.').to_lowercase();
        l == "ул" || l == "бул" || l == "жк" || l == "улица" || l == "булевард"
    }) {
        words.remove(0);
    }
    let mut extra = Vec::new();
    let mut building = String::new();
    words.retain(|w| {
        let l = w.to_lowercase();
        if l.starts_with("ет") || l.starts_with("вх") || l.starts_with("ап") || l.starts_with("fl")
        {
            extra.push(w.clone());
            false
        } else if l.starts_with("бл") || l.starts_with("блок") {
            building = w
                .trim_start_matches("бл.")
                .trim_start_matches("блок")
                .trim()
                .to_string();
            if building.is_empty() {
                building = w.clone();
            }
            false
        } else {
            true
        }
    });
    let mut number = String::new();
    if words.last().is_some_and(|w| take_digits(w)) {
        number = words.pop().unwrap_or_default();
    }
    let street = words.join(" ");
    if city.is_empty() {
        if let Some(second) = kept.get(1) {
            city = second.clone();
        }
    }
    let mut p = Parts::empty(&country);
    p.street_name = street.chars().take(70).collect();
    p.building_number = number.chars().take(18).collect();
    p.address_building = building.chars().take(35).collect();
    p.additional_address_detail = extra.join(" ").chars().take(70).collect();
    p.city = city.chars().take(35).collect();
    p.post_code = post.chars().take(18).collect();
    p.region = region_of_city(&p.city);
    p.address_type = if p.street_name.is_empty() {
        String::new()
    } else {
        "StreetAddress".into()
    };
    p.ok = !p.street_name.is_empty() || !p.city.is_empty();
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_bg_address() {
        let p = split_local("ул. Витоша 15, ет. 3, ап. 7, 1000 София", "BG", "", "");
        assert_eq!(p.street_name, "Витоша");
        assert_eq!(p.building_number, "15");
        assert_eq!(p.post_code, "1000");
        assert_eq!(p.city, "София");
        assert!(p.ok);
    }
}
