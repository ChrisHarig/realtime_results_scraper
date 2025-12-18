use scraper::{Html, Selector};

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Metadata extracted from event page header
#[derive(Debug, Clone)]
pub struct EventMetadata {
    pub venue: Option<String>,
    pub meet_name: Option<String>,
    pub event_headline: String,
    pub records: Vec<String>,
}

/// Race type information parsed from event headline
#[derive(Debug, Clone)]
pub struct RaceInfo {
    pub event_number: u32,
    pub gender: Option<String>,
    pub distance: Option<u16>,
    pub course: Option<String>,
    pub stroke: Option<String>,
    pub is_relay: bool,
    pub other: Vec<String>,
}

impl RaceInfo {
    /// Returns course code (SCY, SCM, LCM) based on course string.
    pub fn course_code(&self) -> Option<&'static str> {
        let course = self.course.as_ref()?.to_lowercase();
        if course.contains("yard") {
            Some("SCY")
        } else if course.contains("lc") || course.contains("long") {
            Some("LCM")
        } else if course.contains("sc") || course.contains("short") {
            Some("SCM")
        } else if course.contains("meter") {
            Some("LCM")
        } else {
            None
        }
    }
}

// ============================================================================
// KNOWN VALUES FOR TOKEN CLASSIFICATION
// ============================================================================

const GENDERS: &[&str] = &["Men", "Women", "Boys", "Girls", "Mixed", "Male", "Female"];
const COURSE_WORDS: &[&str] = &["Yard", "Yards", "Meter", "Meters", "LC", "SC", "Long", "Short"];
const STROKES: &[&str] = &[
    "Freestyle", "Free",
    "Backstroke", "Back",
    "Breaststroke", "Breast",
    "Butterfly", "Fly",
    "Individual", "Medley", "IM",
    "Relay",
];

// ============================================================================
// PARSING - RACE INFO
// ============================================================================

/// Parses race information from event headline using token classification.
pub fn parse_race_info(headline: &str) -> Option<RaceInfo> {
    let tokens: Vec<&str> = headline.split_whitespace().collect();

    let event_idx = tokens.iter().position(|&t| t.eq_ignore_ascii_case("Event"))?;
    let event_number: u32 = tokens.get(event_idx + 1)?.parse().ok()?;

    let remaining = &tokens[event_idx + 2..];

    let mut gender: Option<String> = None;
    let mut distance: Option<u16> = None;
    let mut course_parts: Vec<String> = Vec::new();
    let mut stroke_parts: Vec<String> = Vec::new();
    let mut other: Vec<String> = Vec::new();

    for &token in remaining {
        if is_gender(token) {
            gender = Some(token.to_string());
        } else if is_distance(token) {
            distance = token.parse().ok();
        } else if is_course_word(token) {
            course_parts.push(token.to_string());
        } else if is_stroke_word(token) {
            stroke_parts.push(token.to_string());
        } else {
            other.push(token.to_string());
        }
    }

    let course = if course_parts.is_empty() {
        None
    } else {
        Some(course_parts.join(" "))
    };

    let stroke = if stroke_parts.is_empty() {
        None
    } else {
        Some(stroke_parts.join(" "))
    };

    let is_relay = headline.to_lowercase().contains("relay");

    Some(RaceInfo {
        event_number,
        gender,
        distance,
        course,
        stroke,
        is_relay,
        other,
    })
}

fn is_gender(token: &str) -> bool {
    GENDERS.iter().any(|&g| g.eq_ignore_ascii_case(token))
}

fn is_distance(token: &str) -> bool {
    token.parse::<u16>().is_ok()
}

fn is_course_word(token: &str) -> bool {
    COURSE_WORDS.iter().any(|&c| c.eq_ignore_ascii_case(token))
}

fn is_stroke_word(token: &str) -> bool {
    STROKES.iter().any(|&s| s.eq_ignore_ascii_case(token))
}

// ============================================================================
// PARSING - METADATA
// ============================================================================

/// Extracts metadata (venue, meet name, records) from HTML document.
pub fn parse_event_metadata(html: &str) -> Option<EventMetadata> {
    let document = Html::parse_document(html);
    let pre_selector = Selector::parse("pre").unwrap();

    let pre = document.select(&pre_selector).next()?;
    let content = pre.text().collect::<String>();
    let lines: Vec<&str> = content.lines().collect();

    let mut header_lines: Vec<String> = Vec::new();
    let mut event_headline = String::new();
    let mut records: Vec<String> = Vec::new();
    let mut found_event = false;

    for line in &lines {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.contains("Event") && trimmed.chars().any(|c| c.is_ascii_digit()) {
            event_headline = trimmed.to_string();
            found_event = true;
            continue;
        }

        if !found_event {
            header_lines.push(trimmed.to_string());
            continue;
        }

        if found_event {
            if trimmed.contains(':') && trimmed.chars().filter(|c| c.is_ascii_digit()).count() >= 4 {
                records.push(trimmed.to_string());
            }

            if trimmed.chars().next().is_some_and(|c| c.is_ascii_digit()) && !trimmed.contains(':') {
                break;
            }
        }
    }

    let venue = header_lines.first().cloned();
    let meet_name = header_lines.get(1).cloned();

    Some(EventMetadata {
        venue,
        meet_name,
        event_headline,
        records,
    })
}

/// Extracts event name from HTML bold tag.
pub fn extract_event_name(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("b").unwrap();

    for element in document.select(&selector) {
        let text = element.text().collect::<String>();
        if text.contains("Event") {
            return Some(text.trim().to_string());
        }
    }

    None
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_race_info_standard() {
        let info = parse_race_info("Event 10  Men 200 Yard IM").unwrap();
        assert_eq!(info.event_number, 10);
        assert_eq!(info.gender, Some("Men".to_string()));
        assert_eq!(info.distance, Some(200));
        assert_eq!(info.course, Some("Yard".to_string()));
        assert_eq!(info.stroke, Some("IM".to_string()));
        assert!(info.other.is_empty());
    }

    #[test]
    fn test_parse_race_info_with_age_group() {
        let info = parse_race_info("Event 5  Girls 13-14 100 Yard Freestyle").unwrap();
        assert_eq!(info.event_number, 5);
        assert_eq!(info.gender, Some("Girls".to_string()));
        assert_eq!(info.distance, Some(100));
        assert_eq!(info.course, Some("Yard".to_string()));
        assert_eq!(info.stroke, Some("Freestyle".to_string()));
        assert_eq!(info.other, vec!["13-14"]);
    }

    #[test]
    fn test_parse_race_info_lc_meter() {
        let info = parse_race_info("Event 3  Women 200 LC Meter Backstroke").unwrap();
        assert_eq!(info.event_number, 3);
        assert_eq!(info.gender, Some("Women".to_string()));
        assert_eq!(info.distance, Some(200));
        assert_eq!(info.course, Some("LC Meter".to_string()));
        assert_eq!(info.course_code(), Some("LCM"));
        assert_eq!(info.stroke, Some("Backstroke".to_string()));
    }

    #[test]
    fn test_parse_race_info_relay() {
        let info = parse_race_info("Event 1  Men 400 Yard Medley Relay").unwrap();
        assert_eq!(info.event_number, 1);
        assert_eq!(info.distance, Some(400));
        assert_eq!(info.stroke, Some("Medley Relay".to_string()));
        assert!(info.is_relay);
    }

    #[test]
    fn test_parse_race_info_individual_medley() {
        let info = parse_race_info("Event 7  Women 400 Yard Individual Medley").unwrap();
        assert_eq!(info.stroke, Some("Individual Medley".to_string()));
        assert!(!info.is_relay);
    }

    #[test]
    fn test_course_codes() {
        let mut info = parse_race_info("Event 1  Men 50 Yard Free").unwrap();
        assert_eq!(info.course_code(), Some("SCY"));

        info.course = Some("LC Meter".to_string());
        assert_eq!(info.course_code(), Some("LCM"));

        info.course = Some("SC Meter".to_string());
        assert_eq!(info.course_code(), Some("SCM"));

        info.course = Some("Meter".to_string());
        assert_eq!(info.course_code(), Some("LCM"));
    }
}
