use std::{cmp::Ordering, fmt, str::FromStr};

use jiff::{civil::Date, tz::TimeZone, Timestamp};
use serde::{Deserialize, Serialize};

use crate::ConvcoError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VersionSchemeName {
    #[default]
    Semver,
    Calver,
}

impl fmt::Display for VersionSchemeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Semver => write!(f, "semver"),
            Self::Calver => write!(f, "calver"),
        }
    }
}

impl FromStr for VersionSchemeName {
    type Err = ConvcoError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "semver" => Ok(Self::Semver),
            "calver" => Ok(Self::Calver),
            _ => Err(ConvcoError::InvalidVersionScheme(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum VersionScheme {
    #[default]
    Semver,
    Calver(CalverFormat),
}

impl VersionScheme {
    pub fn resolve(name: VersionSchemeName, calver_format: &str) -> Result<Self, ConvcoError> {
        match name {
            VersionSchemeName::Semver => Ok(Self::Semver),
            VersionSchemeName::Calver => Ok(Self::Calver(CalverFormat::parse(calver_format)?)),
        }
    }
}

impl fmt::Display for VersionScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Semver => write!(f, "semver"),
            Self::Calver(_) => write!(f, "calver"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionTag {
    Semver(semver::Version),
    Calver(CalverVersion),
}

impl VersionTag {
    pub fn is_prerelease(&self) -> bool {
        match self {
            Self::Semver(version) => !version.pre.is_empty(),
            Self::Calver(version) => version.modifier.is_some(),
        }
    }

    pub fn patch_component(&self) -> u64 {
        match self {
            Self::Semver(version) => version.patch,
            Self::Calver(version) => version.component(2),
        }
    }

    pub fn component(&self, index: usize) -> u64 {
        match self {
            Self::Semver(version) => match index {
                0 => version.major,
                1 => version.minor,
                2 => version.patch,
                _ => 0,
            },
            Self::Calver(version) => version.component(index),
        }
    }
}

impl fmt::Display for VersionTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Semver(version) => write!(f, "{version}"),
            Self::Calver(version) => write!(f, "{version}"),
        }
    }
}

impl Ord for VersionTag {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Semver(left), Self::Semver(right)) => left.cmp(right),
            (Self::Calver(left), Self::Calver(right)) => left.cmp(right),
            (Self::Semver(_), Self::Calver(_)) => Ordering::Less,
            (Self::Calver(_), Self::Semver(_)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for VersionTag {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalverFormat {
    segments: Vec<CalverSegment>,
    modifier: bool,
    optional_segment: Option<usize>,
}

impl CalverFormat {
    pub fn parse(format: &str) -> Result<Self, ConvcoError> {
        let mut parts = split_format_parts(format)?;
        if !(2..=4).contains(&parts.len()) {
            return Err(ConvcoError::InvalidCalverFormat(
                "CalVer format must have two or three numeric segments and an optional modifier"
                    .to_owned(),
            ));
        }

        let mut modifier = false;
        let numeric_parts = if parts.last().is_some_and(|part| part == "MODIFIER") {
            modifier = true;
            parts.pop();
            parts.as_slice()
        } else {
            parts.as_slice()
        };

        if !(2..=3).contains(&numeric_parts.len()) {
            return Err(ConvcoError::InvalidCalverFormat(
                "CalVer format must have two or three numeric segments".to_owned(),
            ));
        }

        let mut has_calendar = false;
        let mut has_week = false;
        let mut has_month_or_day = false;
        let mut segments = Vec::with_capacity(numeric_parts.len());
        let mut optional_segment = None;
        for (index, part) in numeric_parts.iter().enumerate() {
            let (part, optional) = optional_part(part)?;
            if optional {
                if index != numeric_parts.len() - 1 {
                    return Err(ConvcoError::InvalidCalverFormat(
                        "only the final numeric CalVer segment can be optional".to_owned(),
                    ));
                }
                optional_segment = Some(index);
            }
            let segment = CalverSegment::parse(part)?;
            if optional && segment.is_calendar() {
                return Err(ConvcoError::InvalidCalverFormat(
                    "calendar CalVer segments cannot be optional".to_owned(),
                ));
            }
            has_calendar |= segment.is_calendar();
            has_week |= matches!(segment.token, CalverToken::Week { .. });
            has_month_or_day |= matches!(
                segment.token,
                CalverToken::Month { .. } | CalverToken::Day { .. }
            );
            segments.push(segment);
        }

        if !has_calendar {
            return Err(ConvcoError::InvalidCalverFormat(
                "CalVer format must include at least one calendar token".to_owned(),
            ));
        }
        if has_week && has_month_or_day {
            return Err(ConvcoError::InvalidCalverFormat(
                "CalVer format cannot mix week tokens with month or day tokens".to_owned(),
            ));
        }

        Ok(Self {
            segments,
            modifier,
            optional_segment,
        })
    }

    pub fn parse_version(&self, value: &str) -> Option<CalverVersion> {
        let mut parts = value.split('.').collect::<Vec<_>>();
        let modifier = if self.modifier && parts.len() == self.segments.len() + 1 {
            Some(parts.pop()?.to_owned())
        } else {
            None
        };
        if parts.len() != self.segments.len() {
            if self.optional_segment.is_some() && parts.len() == self.segments.len() - 1 {
                parts.push("0");
            } else {
                return None;
            }
        }
        if parts.len() != self.segments.len() {
            return None;
        }

        let mut values = Vec::with_capacity(parts.len());
        for (part, segment) in parts.iter().zip(&self.segments) {
            let value = segment.parse_value(part)?;
            values.push(value);
        }
        Some(CalverVersion {
            format: self.clone(),
            values,
            modifier,
        })
    }

    pub fn current_version(&self, date: Date) -> CalverVersion {
        let values = self
            .segments
            .iter()
            .map(|segment| segment.current_value(date))
            .collect();
        CalverVersion {
            format: self.clone(),
            values,
            modifier: None,
        }
    }

    fn same_calendar_period(&self, left: &CalverVersion, right: &CalverVersion) -> bool {
        self.segments
            .iter()
            .enumerate()
            .filter(|(_, segment)| segment.is_calendar())
            .all(|(index, _)| left.values.get(index) == right.values.get(index))
    }

    pub fn next_version(
        &self,
        current: Option<&CalverVersion>,
        has_release: bool,
        forced: bool,
        date: Date,
        existing: &[VersionTag],
    ) -> Result<(CalverVersion, bool), ConvcoError> {
        let mut next = self.current_version(date);
        let Some(current) = current else {
            return Ok((next, true));
        };

        if !has_release && !forced {
            return Ok((current.clone(), false));
        }

        if self.same_calendar_period(current, &next) {
            if let Some(counter_index) = self.segments.iter().rposition(|segment| {
                matches!(
                    segment.token,
                    CalverToken::Major | CalverToken::Minor | CalverToken::Micro
                )
            }) {
                for (index, segment) in self.segments.iter().enumerate() {
                    if index == counter_index {
                        next.values[index] = current.values[index] + 1;
                    } else if index < counter_index
                        && matches!(
                            segment.token,
                            CalverToken::Major | CalverToken::Minor | CalverToken::Micro
                        )
                    {
                        next.values[index] = current.values[index];
                    }
                }
            } else if existing
                .iter()
                .any(|version| matches!(version, VersionTag::Calver(version) if version == &next))
            {
                return Err(ConvcoError::DuplicateCalverVersion(next.to_string()));
            }
        }

        Ok((next, true))
    }
}

fn split_format_parts(format: &str) -> Result<Vec<String>, ConvcoError> {
    let Some((prefix, optional)) = format.split_once("(.") else {
        return Ok(format.split('.').map(str::to_owned).collect());
    };
    let optional = optional
        .strip_suffix(')')
        .ok_or_else(|| {
            ConvcoError::InvalidCalverFormat(
                "optional CalVer segments must be written as (.TOKEN)".to_owned(),
            )
        })?
        .to_owned();
    if optional.contains('.') {
        return Err(ConvcoError::InvalidCalverFormat(
            "only one final CalVer segment can be optional".to_owned(),
        ));
    }
    let mut parts = prefix.split('.').map(str::to_owned).collect::<Vec<_>>();
    parts.push(format!("({optional})"));
    Ok(parts)
}

fn optional_part(value: &str) -> Result<(&str, bool), ConvcoError> {
    if let Some(stripped) = value.strip_prefix('(') {
        return stripped
            .strip_suffix(')')
            .filter(|part| !part.is_empty())
            .map(|part| (part, true))
            .ok_or_else(|| {
                ConvcoError::InvalidCalverFormat(
                    "optional CalVer segments must be written as (.TOKEN)".to_owned(),
                )
            });
    }
    if value.ends_with(')') {
        return Err(ConvcoError::InvalidCalverFormat(
            "optional CalVer segments must be written as (.TOKEN)".to_owned(),
        ));
    }
    Ok((value, false))
}

impl Default for CalverFormat {
    fn default() -> Self {
        Self::parse("YYYY.0M.MICRO").expect("default CalVer format is valid")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CalverSegment {
    token: CalverToken,
    padded: bool,
}

impl CalverSegment {
    fn parse(value: &str) -> Result<Self, ConvcoError> {
        let (token, padded) = match value {
            "YYYY" => (CalverToken::Year { short: false }, false),
            "YY" => (CalverToken::Year { short: true }, false),
            "0Y" => (CalverToken::Year { short: true }, true),
            "MM" => (CalverToken::Month { padded: false }, false),
            "0M" => (CalverToken::Month { padded: true }, true),
            "WW" => (CalverToken::Week { padded: false }, false),
            "0W" => (CalverToken::Week { padded: true }, true),
            "DD" => (CalverToken::Day { padded: false }, false),
            "0D" => (CalverToken::Day { padded: true }, true),
            "MAJOR" => (CalverToken::Major, false),
            "MINOR" => (CalverToken::Minor, false),
            "MICRO" | "PATCH" => (CalverToken::Micro, false),
            other => return Err(ConvcoError::InvalidCalverToken(other.to_owned())),
        };
        Ok(Self { token, padded })
    }

    fn parse_value(&self, value: &str) -> Option<u64> {
        let expected_width = match self.token {
            CalverToken::Year { short: false } => Some(4),
            CalverToken::Year { short: true } => Some(2),
            _ if self.padded => Some(2),
            _ => None,
        };
        if expected_width.is_some_and(|width| value.len() != width) {
            return None;
        }

        let value = value.parse::<u64>().ok()?;
        let valid = match self.token {
            CalverToken::Year { short: false } => (1..=9999).contains(&value),
            CalverToken::Year { short: true } => value <= 99,
            CalverToken::Month { .. } => (1..=12).contains(&value),
            CalverToken::Week { .. } => (1..=53).contains(&value),
            CalverToken::Day { .. } => (1..=31).contains(&value),
            CalverToken::Major | CalverToken::Minor | CalverToken::Micro => true,
        };
        valid.then_some(value)
    }

    fn is_calendar(&self) -> bool {
        matches!(
            self.token,
            CalverToken::Year { .. }
                | CalverToken::Month { .. }
                | CalverToken::Week { .. }
                | CalverToken::Day { .. }
        )
    }

    fn current_value(&self, date: Date) -> u64 {
        match self.token {
            CalverToken::Year { short: false } => date.year() as u64,
            CalverToken::Year { short: true } => (date.year() - 2000) as u64,
            CalverToken::Month { .. } => date.month() as u64,
            CalverToken::Week { .. } => date.iso_week_date().week() as u64,
            CalverToken::Day { .. } => date.day() as u64,
            CalverToken::Major | CalverToken::Minor | CalverToken::Micro => 0,
        }
    }

    fn format_value(&self, value: u64) -> String {
        if self.padded {
            format!("{value:02}")
        } else {
            value.to_string()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CalverToken {
    Year { short: bool },
    Month { padded: bool },
    Week { padded: bool },
    Day { padded: bool },
    Major,
    Minor,
    Micro,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalverVersion {
    format: CalverFormat,
    values: Vec<u64>,
    modifier: Option<String>,
}

impl CalverVersion {
    pub fn component(&self, index: usize) -> u64 {
        self.values.get(index).copied().unwrap_or_default()
    }

    pub fn format(&self) -> &CalverFormat {
        &self.format
    }
}

impl fmt::Display for CalverVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = self
            .format
            .segments
            .iter()
            .enumerate()
            .zip(&self.values)
            .filter_map(|((index, segment), value)| {
                if self.format.optional_segment == Some(index) && *value == 0 {
                    None
                } else {
                    Some(segment.format_value(*value))
                }
            })
            .collect::<Vec<_>>();
        if let Some(modifier) = &self.modifier {
            parts.push(modifier.clone());
        }
        write!(f, "{}", parts.join("."))
    }
}

impl Ord for CalverVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.values
            .cmp(&other.values)
            .then_with(|| match (&self.modifier, &other.modifier) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(left), Some(right)) => left.cmp(right),
            })
    }
}

impl PartialOrd for CalverVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn utc_today() -> Date {
    Timestamp::now().to_zoned(TimeZone::UTC).date()
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::*;

    #[test]
    fn calver_format_supports_calendar_tokens() {
        for format in [
            "YYYY.0M.MICRO",
            "YYYY.0M(.MICRO)",
            "YY.MM.PATCH",
            "0Y.0M.0D",
            "YYYY.0W.MICRO",
            "YYYY.0M.MICRO.MODIFIER",
        ] {
            CalverFormat::parse(format).unwrap();
        }
    }

    #[test]
    fn semver_scheme_does_not_parse_calver_format() {
        assert_eq!(
            VersionScheme::resolve(VersionSchemeName::Semver, "not-a-calver-format").unwrap(),
            VersionScheme::Semver
        );
    }

    #[test]
    fn calver_format_rejects_invalid_tokens_and_mixed_week() {
        assert!(CalverFormat::parse("YYYY.NOPE.MICRO").is_err());
        assert!(CalverFormat::parse("YYYY.0M.0W").is_err());
        assert!(CalverFormat::parse("MAJOR.MINOR.MICRO").is_err());
    }

    #[test]
    fn calver_formats_and_parses_padded_values() {
        let format = CalverFormat::parse("YYYY.0M.MICRO").unwrap();
        let version = format.current_version(date(2026, 7, 9));
        assert_eq!(version.to_string(), "2026.07.0");
        assert_eq!(
            format.parse_version("2026.07.3").unwrap().to_string(),
            "2026.07.3"
        );
        assert!(format.parse_version("2026.7.3").is_none());
    }

    #[test]
    fn calver_rejects_invalid_calendar_widths_and_ranges() {
        let format = CalverFormat::parse("YYYY.0M.0D").unwrap();
        for version in [
            "26.07.09",
            "2026.7.09",
            "2026.00.09",
            "2026.13.09",
            "2026.07.00",
            "2026.07.32",
        ] {
            assert!(
                format.parse_version(version).is_none(),
                "unexpectedly parsed {version}"
            );
        }

        let short_year = CalverFormat::parse("YY.MM").unwrap();
        assert!(short_year.parse_version("6.7").is_none());
        assert!(short_year.parse_version("2026.7").is_none());
        assert!(short_year.parse_version("26.7").is_some());
    }

    #[test]
    fn calver_bumps_rightmost_counter_without_a_micro_segment() {
        let format = CalverFormat::parse("YYYY.MAJOR").unwrap();
        let current = format.parse_version("2026.3").unwrap();
        let existing = vec![VersionTag::Calver(current.clone())];
        let (next, changed) = format
            .next_version(Some(&current), true, false, date(2026, 7, 9), &existing)
            .unwrap();
        assert!(changed);
        assert_eq!(next.to_string(), "2026.4");

        let format = CalverFormat::parse("YYYY.MAJOR.MINOR").unwrap();
        let current = format.parse_version("2026.3.7").unwrap();
        let (next, _) = format
            .next_version(Some(&current), true, false, date(2026, 7, 9), &[])
            .unwrap();
        assert_eq!(next.to_string(), "2026.3.8");
    }

    #[test]
    fn calver_optional_final_segment_parses_and_trims_zero() {
        let format = CalverFormat::parse("YYYY.0M(.MICRO)").unwrap();
        assert_eq!(
            format.parse_version("2026.07").unwrap().to_string(),
            "2026.07"
        );
        assert_eq!(
            format.parse_version("2026.07.0").unwrap().to_string(),
            "2026.07"
        );
        assert_eq!(
            format.parse_version("2026.07.1").unwrap().to_string(),
            "2026.07.1"
        );
    }

    #[test]
    fn calver_versions_sort_by_components_and_modifier() {
        let format = CalverFormat::parse("YYYY.0M.MICRO.MODIFIER").unwrap();
        let stable = format.parse_version("2026.07.1").unwrap();
        let rc = format.parse_version("2026.07.1.rc1").unwrap();
        let next = format.parse_version("2026.07.2").unwrap();
        assert!(stable > rc);
        assert!(next > stable);
    }
}
