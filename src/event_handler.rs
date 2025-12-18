use scraper::{Html, Selector};
use serde::Serialize;
use std::error::Error;

use crate::metadata::{EventMetadata, RaceInfo};

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// A race split
#[derive(Debug, Clone, Serialize)]
pub struct Split {
    pub distance: u16,
    pub time: String,
}

/// Individual swimmer result
#[derive(Debug, Clone, Serialize)]
pub struct Swimmer {
    pub place: u8,
    pub name: String,
    pub year: String,
    pub school: String,
    pub seed_time: Option<String>,
    pub final_time: String,
    pub reaction_time: Option<String>,
    #[serde(skip)]
    pub splits: Vec<Split>,
}

/// Complete event results with metadata
#[derive(Debug)]
pub struct EventResults {
    pub event_name: String,
    pub session: char,  // 'P' for prelims, 'F' for finals
    pub metadata: Option<EventMetadata>,
    pub race_info: Option<RaceInfo>,
    pub swimmers: Vec<Swimmer>,
}

// ============================================================================
// INDIVIDUAL EVENT PARSING
// ============================================================================

/// Parse individual (non-relay) event HTML
pub fn parse_individual_event_html(
    html: &str,
    event_name: &str,
    session: char,
    metadata: Option<EventMetadata>,
    race_info: Option<RaceInfo>,
) -> Result<EventResults, Box<dyn Error>> {
    let document = Html::parse_document(html);
    let mut swimmers = Vec::new();

    let pre_selector = Selector::parse("pre").unwrap();
    if let Some(pre) = document.select(&pre_selector).next() {
        let content = pre.text().collect::<String>();
        let lines: Vec<&str> = content.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let current_line = lines[i].trim();

            // Check if this is a main swimmer line (starts with place number)
            if is_swimmer_line(current_line) {
                // Find the next main swimmer line or end of content
                let mut next_main_line_idx = i + 1;
                while next_main_line_idx < lines.len() {
                    let next_line = lines[next_main_line_idx].trim();
                    if !next_line.is_empty() && is_swimmer_line(next_line) {
                        break;
                    }
                    next_main_line_idx += 1;
                }

                // Parse swimmer section
                if let Some(swimmer) = parse_swimmer_section(&lines[i..next_main_line_idx]) {
                    swimmers.push(swimmer);
                    if swimmers.len() >= 16 {
                        break;
                    }
                }

                i = next_main_line_idx;
                continue;
            }
            i += 1;
        }
    }

    Ok(EventResults {
        event_name: event_name.to_string(),
        session,
        metadata,
        race_info,
        swimmers,
    })
}

// ============================================================================
// SWIMMER PARSING
// ============================================================================

/// Check if a line is a swimmer result line (place number + name) vs a split line
/// Swimmer lines: "1 Marchand, Leon JR ASU ..."
/// Split lines: "1:08.61 (23.99) ...", "r:+0.62 21.09 ..."
fn is_swimmer_line(line: &str) -> bool {
    let first_token = line.split_whitespace().next();
    match first_token {
        Some(token) => {
            // If first token is purely digits, it's a place number (swimmer line)
            // If it contains ':' or '.' it's likely a time (split line)
            if token.chars().all(|c| c.is_ascii_digit()) {
                true
            } else {
                false
            }
        }
        None => false,
    }
}

/// Known year/class patterns - always 2 characters
fn is_year_pattern(s: &str) -> bool {
    if s.len() != 2 {
        return false;
    }

    // Check for known letter patterns (FR, SO, JR, SR, etc.)
    if matches!(s.to_uppercase().as_str(), "FR" | "SO" | "JR" | "SR" | "GR" | "5Y" | "RS" | "FF") {
        return true;
    }

    // Check for 2-digit numeric year (e.g., "23", "24", "25")
    s.chars().all(|c| c.is_ascii_digit())
}

fn parse_swimmer_section(lines: &[&str]) -> Option<Swimmer> {
    let main_line = lines[0].trim();

    // Parse main line: place name(Last, First) year school(multi-word) seed_time final_time points
    let parts: Vec<&str> = main_line.split_whitespace().collect();

    if parts.len() < 6 {
        return None;
    }

    let place: u8 = parts[0].parse().ok()?;

    // Work backwards from the end for fixed fields
    let _points = parts.last()?.parse::<u8>().ok()?;
    let final_time = parts[parts.len()-2];
    let seed_time = Some(parts[parts.len()-3].to_string());

    // Find year by scanning forward from position 1 for year pattern
    // Name is "Last, First" format, so year comes after the name tokens
    let mut year_idx = None;
    for i in 1..parts.len()-3 {
        if is_year_pattern(parts[i]) {
            year_idx = Some(i);
            break;
        }
    }

    let year_idx = year_idx?;

    // Name is everything from position 1 to year
    let name = parts[1..year_idx].join(" ");
    let year = parts[year_idx];

    // School is everything between year and seed_time (supports multi-word schools)
    let school_end = parts.len() - 3;
    let school = parts[year_idx+1..school_end].join(" ");

    // Parse splits and reaction time
    let (reaction_time, splits) = parse_splits(lines);

    Some(Swimmer {
        place,
        name,
        year: year.to_string(),
        school,
        seed_time,
        final_time: final_time.to_string(),
        reaction_time,
        splits,
    })
}

/// Check if a string looks like a valid time format
/// Valid: 21.09, 44.62, 1:08.61, 4:02.31N, 4:02.31
/// Invalid: 1., 2., 10.
pub fn is_valid_time_format(s: &str) -> bool {
    // Strip trailing letters (like N for NCAA record)
    let s = s.trim_end_matches(|c: char| c.is_ascii_alphabetic());

    // Must contain : or .
    if !s.contains(':') && !s.contains('.') {
        return false;
    }

    // If contains :, format should be like M:SS.ss or MM:SS.ss
    if let Some(colon_pos) = s.find(':') {
        // Must have digits before and after colon
        let before = &s[..colon_pos];
        let after = &s[colon_pos + 1..];
        return !before.is_empty() &&
               before.chars().all(|c| c.is_ascii_digit()) &&
               after.contains('.') &&
               after.len() >= 4; // At least SS.s
    }

    // If just contains ., must have at least 2 digits after the decimal
    if let Some(dot_pos) = s.find('.') {
        let after = &s[dot_pos + 1..];
        return after.len() >= 2 && after.chars().all(|c| c.is_ascii_digit());
    }

    false
}

/// Returns (reaction_time, splits)
fn parse_splits(lines: &[&str]) -> (Option<String>, Vec<Split>) {
    let mut splits = Vec::new();
    let mut reaction_time: Option<String> = None;

    // Process all lines after the main swimmer line
    for line_idx in 1..lines.len() {
        let line = lines[line_idx].trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();

        for part in parts.iter() {
            // Skip delta times in parentheses like "(23.53)" - we want cumulative times
            if part.starts_with('(') {
                continue;
            }

            // Check if this is a reaction time (starts with "r")
            if part.starts_with('r') {
                reaction_time = Some(part.to_string());
                continue;
            }

            // Check if this is a cumulative time (contains : or single .)
            // Times look like: 21.09, 44.62, 1:08.61, 4:02.31N
            // Must have digits on both sides of : or . to be a valid time
            let is_time = !part.contains('(') &&
                          part.chars().next().map_or(false, |c| c.is_ascii_digit()) &&
                          is_valid_time_format(part);

            if is_time {
                let distance = (splits.len() as u16 + 1) * 50;

                splits.push(Split {
                    distance,
                    time: part.to_string(),
                });
            }
        }
    }

    (reaction_time, splits)
}
