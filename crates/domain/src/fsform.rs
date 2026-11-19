//! Валидация и разбор на формули за финансови отчети.
//! Порт на ytbuch reports_fs::check/parse — изчислението е в reports_fs.

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Piece {
    At(String),
    Acct { pat: String, which: String },
    Corr { debit: String, credit: String },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Tok {
    pub neg: bool,
    pub piece: Piece,
}

pub fn check(raw: &str) -> Result<usize, String> {
    parse(raw).map(|t| t.len())
}

pub(crate) fn parse(raw: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = raw.trim().chars().collect();
    if chars.is_empty() {
        return Ok(Vec::new());
    }
    let mut i = 0;
    let mut first = true;
    let mut out = Vec::new();
    while i < chars.len() {
        i = skip(&chars, i);
        if i >= chars.len() {
            break;
        }
        let mut neg = false;
        if chars[i] == '+' {
            i += 1;
            first = false;
        } else if chars[i] == '-' || chars[i] == '−' {
            neg = true;
            i += 1;
            first = false;
        } else if first {
            first = false;
        } else {
            return Err("очакван знак + или −".into());
        }
        i = skip(&chars, i);
        if i >= chars.len() {
            return Err("формулата свършва след знака".into());
        }
        let piece = if chars[i] == '@' {
            i += 1;
            let (code, n) = read_until(&chars, i, |c| {
                c == '+' || c == '-' || c == '−' || c == ' ' || c == '\t'
            });
            if code.is_empty() {
                return Err("липсва код след @".into());
            }
            i = n;
            Piece::At(code)
        } else {
            let (acc, n) = read_acc(&chars, i);
            if acc.is_empty() {
                return Err("липсва сметка".into());
            }
            i = n;
            let Some((kind, kn)) = kind_at(&chars, i) else {
                return Err(format!("липсва Д/К/ОД/ОК/НД/НК след {acc}"));
            };
            i += kn;
            i = skip(&chars, i);
            if i < chars.len() && chars[i] == '/' {
                i += 1;
                i = skip(&chars, i);
                let (acc2, n2) = read_acc(&chars, i);
                if acc2.is_empty() {
                    return Err("липсва кредитна сметка след /".into());
                }
                i = n2;
                let Some((_, kn2)) = kind_at(&chars, i) else {
                    return Err("липсва Д или К след втората сметка".into());
                };
                i += kn2;
                Piece::Corr {
                    debit: acc,
                    credit: acc2,
                }
            } else {
                Piece::Acct {
                    pat: acc,
                    which: kind.into(),
                }
            }
        };
        out.push(Tok { neg, piece });
    }
    Ok(out)
}

fn skip(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
        i += 1;
    }
    i
}

fn read_acc(chars: &[char], i: usize) -> (String, usize) {
    let mut p = i;
    while p < chars.len() && (chars[p].is_ascii_digit() || chars[p] == '*') {
        p += 1;
    }
    (chars[i..p].iter().collect(), p)
}

fn read_until(chars: &[char], i: usize, stop: impl Fn(char) -> bool) -> (String, usize) {
    let mut p = i;
    while p < chars.len() && !stop(chars[p]) {
        p += 1;
    }
    (chars[i..p].iter().collect(), p)
}

fn kind_at(chars: &[char], i: usize) -> Option<(&'static str, usize)> {
    let rest: String = chars[i..].iter().collect();
    for (src, code) in [
        ("ОД", "TD"),
        ("ОК", "TC"),
        ("НД", "OD"),
        ("НК", "OC"),
        ("Д", "D"),
        ("К", "C"),
    ] {
        if rest.starts_with(src) {
            return Some((code, src.chars().count()));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_formulas() {
        assert!(check("+201Д+202Д-242К").is_ok());
        assert!(check("+@bal.b.i.t+@bal.b.ii.t").is_ok());
        assert!(check("+50*Д/411К").is_ok());
        assert!(check("+601ОД").is_ok());
        assert_eq!(check("").unwrap(), 0);
    }

    #[test]
    fn invalid_formulas() {
        assert!(check("+201").is_err()); // липсва Д/К
        assert!(check("201Д 202К").is_err()); // втори токен без знак
        assert!(check("+@").is_err()); // липсва код след @
        assert!(check("+").is_err()); // свършва след знака
    }

    #[test]
    fn catalog_defaults() {
        assert!(crate::fsnom::known("balance"));
        assert!(!crate::fsnom::known("xxx"));
        assert!(crate::fsnom::has_code("balance", "bal.t"));
        assert!(!crate::fsnom::defaults("balance").is_empty());
        assert!(!crate::fsnom::defaults("pl").is_empty());
        assert_eq!(crate::fsnom::statements().len(), 4);
    }
}
