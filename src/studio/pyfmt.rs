//! The slice of Python string formatting the plans rely on: `{v}` placeholders with an optional `,` grouping and `.Nf`
//! precision (`'${v:.0f}'`, `'{v:,.2f}'`), and thousands grouping for counters.

use anyhow::{bail, Context, Result};

/// `f"{value:,.{dec}f}"`
pub fn thousands(value: f64, dec: usize) -> String {
    let text = format!("{:.dec$}", value.abs());
    let (int, frac) = text.split_once('.').map_or((text.as_str(), None), |(i, f)| (i, Some(f)));
    let mut grouped = String::new();
    for (i, ch) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let negative = value < 0.0 && text.chars().any(|c| c.is_ascii_digit() && c != '0');
    format!("{}{grouped}{}", if negative { "-" } else { "" }, frac.map_or(String::new(), |f| format!(".{f}")))
}

/// Fill the single `{v...}` placeholder of `template` with `v`.
pub fn format_v(template: &str, v: f64) -> Result<String> {
    let Some(open) = template.find('{') else { return Ok(template.to_string()) };
    let close = template[open..].find('}').map(|i| open + i).with_context(|| format!("unclosed placeholder in {template:?}"))?;
    let field = &template[open + 1..close];
    let spec = match field.strip_prefix('v') {
        Some("") => "",
        Some(rest) => rest.strip_prefix(':').with_context(|| format!("unsupported placeholder {{{field}}}"))?,
        None => bail!("unsupported placeholder {{{field}}}: only {{v}} is available"),
    };
    let (group, spec) = spec.strip_prefix(',').map_or((false, spec), |s| (true, s));
    let value = match spec {
        "" => format!("{v}"),
        _ => {
            let digits = spec.strip_prefix('.').and_then(|s| s.strip_suffix('f')).with_context(|| format!("unsupported format spec {spec:?}"))?;
            let dec: usize = digits.parse().with_context(|| format!("bad precision in {spec:?}"))?;
            if group { thousands(v, dec) } else { format!("{v:.dec$}") }
        }
    };
    Ok(format!("{}{value}{}", &template[..open], &template[close + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_like_python() {
        assert_eq!(thousands(1234567.891, 2), "1,234,567.89");
        assert_eq!(thousands(827.0, 0), "827");
        assert_eq!(thousands(-1000.0, 0), "-1,000");
        assert_eq!(thousands(-0.0001, 1), "0.0");
    }

    #[test]
    fn fills_the_placeholder() {
        assert_eq!(format_v("${v:.0f}", 10.4).unwrap(), "$10");
        assert_eq!(format_v("£{v:,.1f}M", 1234.56).unwrap(), "£1,234.6M");
        assert_eq!(format_v("plain", 1.0).unwrap(), "plain");
        assert!(format_v("{x}", 1.0).is_err());
    }
}
