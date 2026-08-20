use super::super::*;
use super::common::argument_value;

pub(crate) fn parse_retention_argument(
    args: &[String],
    flag: &str,
    unlimited_label: &str,
    maximum: i64,
) -> Option<i64> {
    let value = argument_value(args, flag)?;
    if value.eq_ignore_ascii_case(unlimited_label) {
        return Some(0);
    }
    match value.parse::<i64>() {
        Ok(value) if (0..=maximum).contains(&value) => Some(value),
        _ => {
            eprintln!("{flag} must be {unlimited_label} or a number from 0 to {maximum}.");
            std::process::exit(2);
        }
    }
}

pub(crate) fn setting_i64(db: &DbState, key: &str, fallback: i64) -> Result<i64> {
    Ok(db
        .get_setting(key)?
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(fallback))
}

pub(crate) fn retention_count_label(value: i64, unit: &str) -> String {
    if value == 0 {
        "Unlimited".to_string()
    } else {
        format!("{value} {unit}")
    }
}

pub(crate) fn retention_age_label(value: i64) -> String {
    if value == 0 {
        "Forever".to_string()
    } else {
        format!("{value} days")
    }
}
