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
    /// Returns course code (SCY, SCM, LCM) based on course string
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
const COURSE_WORDS: &[&str] = &["Yard", "Yards", "Meter", "Meters", "LC", "SC", "LCM", "SCM", "SCY", "Long", "Short"];
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

/// Parses race information from event headline using token classification
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

/// Checks if a line is a delimiter line (e.g., "=================")
fn is_delimiter_line(line: &str) -> bool {
    line.chars().all(|c| c == '=') && line.len() >= 5
}

// ============================================================================
// PARSING - METADATA
// ============================================================================

/// Extracts metadata (venue, meet name, records) from HTML document
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

    let mut in_records_section = false;

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
            // Records are between two "=====" delimiter lines
            if is_delimiter_line(trimmed) {
                if in_records_section {
                    // Second delimiter - end of records section
                    break;
                } else {
                    // First delimiter - start of records section
                    in_records_section = true;
                    continue;
                }
            }

            if in_records_section {
                records.push(trimmed.to_string());
            }
        }
    }

    // Find meet name - it appears after the "Site License" line
    let mut meet_name: Option<String> = None;
    let mut venue: Option<String> = None;
    let mut found_license = false;

    for line in &header_lines {
        if line.to_lowercase().contains("site license") || line.to_lowercase().contains("license hy-tek") {
            found_license = true;
            continue;
        }
        if found_license && meet_name.is_none() {
            meet_name = Some(line.clone());
        } else if meet_name.is_some() && venue.is_none() {
            venue = Some(line.clone());
            break;
        }
    }

    // Fallback to old behavior if no license line found
    if meet_name.is_none() {
        meet_name = header_lines.first().cloned();
        venue = header_lines.get(1).cloned();
    }

    Some(EventMetadata {
        venue,
        meet_name,
        event_headline,
        records,
    })
}

