use scraper::{Html, Selector};
use serde::Serialize;
use std::error::Error;

use crate::metadata::{EventMetadata, RaceInfo};
use crate::utils::{is_dq_status, is_year_pattern, is_valid_time_format};

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Cumulative split time at a distance
#[derive(Debug, Clone, Serialize)]
pub struct Split {
    pub distance: u16,
    pub time: String,
}

/// Individual swimmer result
#[derive(Debug, Clone, Serialize)]
pub struct Swimmer {
    pub place: Option<u8>,
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
    pub session: char,
    pub metadata: Option<EventMetadata>,
    pub race_info: Option<RaceInfo>,
    pub swimmers: Vec<Swimmer>,
}

// ============================================================================
// INDIVIDUAL EVENT PARSING
// ============================================================================

/// Parses individual (non-relay) event HTML and extracts swimmer results
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

            if is_swimmer_line(current_line) {
                // Find the next swimmer line or end of content
                let mut next_idx = i + 1;
                while next_idx < lines.len() {
                    let next_line = lines[next_idx].trim();
                    if !next_line.is_empty() && is_swimmer_line(next_line) {
                        break;
                    }
                    next_idx += 1;
                }

                if let Some(swimmer) = parse_swimmer_section(&lines[i..next_idx]) {
                    swimmers.push(swimmer);
                }

                i = next_idx;
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

/// Checks if a line starts a swimmer result (place number or -- for DQ)
fn is_swimmer_line(line: &str) -> bool {
    match line.split_whitespace().next() {
        Some(token) => {
            let is_place = token.chars().all(|c| c.is_ascii_digit());
            let is_dq = token == "--";
            is_place || is_dq
        }
        None => false,
    }
}

/// Parses a swimmer section (main line + split lines) into a Swimmer
fn parse_swimmer_section(lines: &[&str]) -> Option<Swimmer> {
    let main_line = lines[0].trim();
    let parts: Vec<&str> = main_line.split_whitespace().collect();

    if parts.len() < 5 {
        return None;
    }

    let is_dq_entry = parts[0] == "--";
    let place: Option<u8> = if is_dq_entry {
        None
    } else {
        Some(parts[0].parse().ok()?)
    };

    let last = parts.last()?;

    // Determine field positions based on entry type
    let (final_time, seed_time, end_offset) = if last.parse::<u8>().is_ok() {
        (parts[parts.len() - 2], Some(parts[parts.len() - 3].to_string()), 3)
    } else if is_dq_status(last) {
        (*last, Some(parts[parts.len() - 2].to_string()), 2)
    } else {
        let seed = if parts.len() > 2 {
            Some(parts[parts.len() - 2].to_string())
        } else {
            None
        };
        (*last, seed, 2)
    };

    // Find year position
    let mut year_idx = None;
    for (i, &part) in parts.iter().enumerate().skip(1).take(parts.len().saturating_sub(end_offset + 1)) {
        if is_year_pattern(part) {
            year_idx = Some(i);
            break;
        }
    }
    let year_idx = year_idx?;

    let name = parts[1..year_idx].join(" ");
    let year = parts[year_idx];
    let school_end = parts.len() - end_offset;
    let school = parts[year_idx + 1..school_end].join(" ");

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

/// Extracts reaction time and split times from swimmer lines
fn parse_splits(lines: &[&str]) -> (Option<String>, Vec<Split>) {
    let mut splits = Vec::new();
    let mut reaction_time: Option<String> = None;

    for line in lines.iter().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        for part in line.split_whitespace() {
            if part.starts_with('(') {
                continue;
            }

            if part.starts_with('r') {
                reaction_time = Some(part.to_string());
                continue;
            }

            let is_time = !part.contains('(')
                && part.chars().next().is_some_and(|c| c.is_ascii_digit())
                && is_valid_time_format(part);

            if is_time {
                splits.push(Split {
                    distance: (splits.len() as u16 + 1) * 50,
                    time: part.to_string(),
                });
            }
        }
    }

    (reaction_time, splits)
}
