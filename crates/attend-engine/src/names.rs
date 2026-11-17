use crate::{AliasMapping, MeetingProfile, RosterPerson};
use pinyin::ToPinyin;
use std::{collections::HashMap, sync::OnceLock};

/// AttendFlow-compatible normalization: trim/lowercase, remove whitespace and punctuation.
pub fn normalize_name(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| {
            !c.is_whitespace()
                && *c != '\u{3000}'
                && !matches!(
                    *c,
                    '(' | ')'
                        | '（'
                        | '）'
                        | '['
                        | ']'
                        | '【'
                        | '】'
                        | '{'
                        | '}'
                        | '<'
                        | '>'
                        | '《'
                        | '》'
                        | '"'
                        | '\''
                        | '“'
                        | '”'
                        | '‘'
                        | '’'
                        | '.'
                        | ','
                        | '，'
                        | '。'
                        | '!'
                        | '！'
                        | '?'
                        | '？'
                        | '、'
                        | ':'
                        | '：'
                        | ';'
                        | '；'
                        | '_'
                        | '-'
                )
        })
        .collect()
}

/// Toneless lower-case pinyin. Characters without a dictionary entry are preserved.
pub fn build_pinyin(value: &str) -> String {
    static OVERRIDES: OnceLock<HashMap<String, String>> = OnceLock::new();
    let overrides = OVERRIDES.get_or_init(|| {
        serde_json::from_str(include_str!("pinyin_overrides.json")).unwrap_or_default()
    });
    if let Some(value) = overrides.get(value) {
        return value.clone();
    }
    value
        .chars()
        .map(|c| {
            c.to_pinyin()
                .map(|p| p.plain().to_lowercase())
                .unwrap_or_else(|| c.to_lowercase().collect())
        })
        .collect()
}

pub fn is_likely_shared_name(value: &str) -> bool {
    let lower = value.to_lowercase();
    ["/", ",", "&", "，", "、", "和", "|"]
        .iter()
        .any(|d| value.contains(d))
        || lower.split_whitespace().any(|w| w == "and")
        || value
            .chars()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|w| ('\u{4e00}'..='\u{9fff}').contains(&w[0]) && w[1].is_whitespace())
}

pub fn parse_shared_name_tokens(value: &str) -> Vec<String> {
    let normalized = value
        .replace(" and ", "|")
        .replace(" AND ", "|")
        .replace(" And ", "|")
        .replace(" 和 ", "|");
    let parts: Vec<String> = normalized
        .split(|c| matches!(c, '、' | '，' | ',' | '/' | '&' | '|'))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();
    if parts.len() <= 1 && is_likely_shared_name(value) {
        value
            .split_whitespace()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect()
    } else {
        parts
    }
}

pub fn candidate_people<'a>(
    zoom_name: &str,
    roster: &'a [RosterPerson],
    aliases: &[AliasMapping],
) -> Vec<&'a RosterPerson> {
    let key = normalize_name(zoom_name);
    let mut out = Vec::new();
    for person in roster {
        let alias = aliases
            .iter()
            .any(|a| a.person_id == person.person_id && normalize_name(&a.zoom_name) == key);
        if alias
            || normalize_name(&person.canonical_name) == key
            || normalize_name(&person.pinyin) == key
        {
            if !out
                .iter()
                .any(|p: &&RosterPerson| p.person_id == person.person_id)
            {
                out.push(person);
            }
        }
    }
    out
}

pub fn profile_includes_group(patterns: &[String], group: &str) -> bool {
    patterns.is_empty()
        || patterns
            .iter()
            .any(|p| wildcard_match(p.trim(), group.trim()))
}
fn wildcard_match(pattern: &str, value: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    if pattern == "*" {
        return true;
    }
    let segments: Vec<_> = pattern.split('*').filter(|x| !x.is_empty()).collect();
    let mut at = 0;
    for (i, seg) in segments.iter().enumerate() {
        if i == 0 && !pattern.starts_with('*') {
            if !value.starts_with(seg) {
                return false;
            }
            at = seg.len();
        } else if let Some(pos) = value[at..].find(seg) {
            at += pos + seg.len();
        } else {
            return false;
        }
    }
    pattern.ends_with('*') || at == value.len()
}

pub fn select_profile<'a>(
    meeting_id: &str,
    topic: &str,
    profiles: &'a [MeetingProfile],
) -> Option<&'a MeetingProfile> {
    let active: Vec<_> = profiles.iter().filter(|p| p.active).collect();
    active
        .iter()
        .find(|p| p.meeting_ids.iter().any(|id| id == meeting_id))
        .copied()
        .or_else(|| {
            active
                .iter()
                .find(|p| {
                    p.topic_keywords.iter().any(|kw| {
                        !kw.is_empty() && topic.to_lowercase().contains(&kw.to_lowercase())
                    })
                })
                .copied()
        })
        .or_else(|| active.iter().find(|p| p.is_default).copied())
        .or_else(|| active.first().copied())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalize_and_shared_names() {
        assert_eq!(normalize_name(" 毛 若水（A） "), "毛若水a");
        assert_eq!(
            parse_shared_name_tokens("刘志远 赵美玲"),
            vec!["刘志远", "赵美玲"]
        );
        assert!(is_likely_shared_name("Alice and Bob"));
    }
    #[test]
    fn wildcard_groups() {
        assert!(profile_includes_group(&["Class-*".into()], "Class-A"));
        assert!(!profile_includes_group(&["Class-*".into()], "Other"));
    }
}
