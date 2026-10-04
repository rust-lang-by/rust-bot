use chrono::NaiveTime;
use log::warn;
use teloxide::types::ChatId;

pub const DEFAULT_HN_API_BASE_URL: &str = "https://hacker-news.firebaseio.com/v0";
pub(crate) const DEFAULT_TOP_N: usize = 3;
pub(crate) const MAX_TOP_N: usize = 10;
const DEFAULT_HOUR_UTC: u32 = 17;

#[derive(Debug, Clone)]
pub struct HnDigestConfig {
    pub chat_ids: Vec<ChatId>,
    pub time_utc: NaiveTime,
    pub top_n: usize,
    pub run_on_startup: bool,
}

impl HnDigestConfig {
    /// `None` means the digest is disabled: `HN_DIGEST_CHAT_IDS` is unset or
    /// holds no valid chat id.
    pub fn from_env() -> Option<Self> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let chat_ids = parse_chat_ids(&lookup("HN_DIGEST_CHAT_IDS")?);
        if chat_ids.is_empty() {
            return None;
        }
        Some(Self {
            chat_ids,
            time_utc: parse_time(lookup("HN_DIGEST_TIME_UTC")),
            top_n: parse_top_n(lookup("HN_DIGEST_TOP_N")),
            run_on_startup: parse_flag(lookup("HN_DIGEST_RUN_ON_STARTUP")),
        })
    }
}

fn parse_chat_ids(raw: &str) -> Vec<ChatId> {
    raw.split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .filter_map(|id| match id.parse::<i64>() {
            Ok(id) => Some(ChatId(id)),
            Err(_) => {
                warn!("HN_DIGEST_CHAT_IDS: ignoring invalid chat id '{id}'");
                None
            }
        })
        .collect()
}

fn default_time() -> NaiveTime {
    NaiveTime::from_hms_opt(DEFAULT_HOUR_UTC, 0, 0).unwrap_or(NaiveTime::MIN)
}

fn parse_time(raw: Option<String>) -> NaiveTime {
    let Some(raw) = raw else {
        return default_time();
    };
    NaiveTime::parse_from_str(raw.trim(), "%H:%M").unwrap_or_else(|_| {
        warn!(
            "HN_DIGEST_TIME_UTC='{raw}' is not HH:MM; using {}",
            default_time()
        );
        default_time()
    })
}

fn parse_top_n(raw: Option<String>) -> usize {
    let Some(raw) = raw else {
        return DEFAULT_TOP_N;
    };
    match raw.trim().parse::<usize>() {
        Ok(n) if (1..=MAX_TOP_N).contains(&n) => n,
        _ => {
            warn!("HN_DIGEST_TOP_N='{raw}' is not in 1..={MAX_TOP_N}; using {DEFAULT_TOP_N}");
            DEFAULT_TOP_N
        }
    }
}

fn parse_flag(raw: Option<String>) -> bool {
    raw.is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "true" | "1" | "yes"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config_from(vars: &[(&str, &str)]) -> Option<HnDigestConfig> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        HnDigestConfig::from_lookup(|key| vars.get(key).cloned())
    }

    #[test]
    fn disabled_without_chat_ids() {
        assert!(config_from(&[]).is_none());
        assert!(config_from(&[("HN_DIGEST_CHAT_IDS", " , ")]).is_none());
        assert!(config_from(&[("HN_DIGEST_CHAT_IDS", "abc")]).is_none());
    }

    #[test]
    fn defaults_apply_when_only_chat_ids_set() {
        let config = config_from(&[("HN_DIGEST_CHAT_IDS", "-1001")]).unwrap();

        assert_eq!(config.chat_ids, vec![ChatId(-1001)]);
        assert_eq!(config.time_utc, NaiveTime::from_hms_opt(17, 0, 0).unwrap());
        assert_eq!(config.top_n, 3);
        assert!(!config.run_on_startup);
    }

    #[test]
    fn parses_all_variables() {
        let config = config_from(&[
            ("HN_DIGEST_CHAT_IDS", " -1001, bogus ,-1002 ,"),
            ("HN_DIGEST_TIME_UTC", "07:30"),
            ("HN_DIGEST_TOP_N", "5"),
            ("HN_DIGEST_RUN_ON_STARTUP", "TRUE"),
        ])
        .unwrap();

        assert_eq!(config.chat_ids, vec![ChatId(-1001), ChatId(-1002)]);
        assert_eq!(config.time_utc, NaiveTime::from_hms_opt(7, 30, 0).unwrap());
        assert_eq!(config.top_n, 5);
        assert!(config.run_on_startup);
    }

    #[test]
    fn invalid_values_fall_back_to_defaults() {
        assert_eq!(parse_time(Some("25:00".into())), default_time());
        assert_eq!(parse_time(Some("noon".into())), default_time());
        assert_eq!(parse_top_n(Some("0".into())), DEFAULT_TOP_N);
        assert_eq!(parse_top_n(Some("11".into())), DEFAULT_TOP_N);
        assert_eq!(parse_top_n(Some("x".into())), DEFAULT_TOP_N);
        assert_eq!(parse_top_n(Some("10".into())), 10);
        assert!(!parse_flag(Some("nope".into())));
        assert!(!parse_flag(None));
    }
}
