use scraper::{Html, Selector};
use serde::Serialize;
use std::error::Error;

use crate::fetch_html;
use crate::event_handler::{is_valid_time_format, Split};
use crate::metadata::{EventMetadata, RaceInfo, parse_event_metadata, parse_race_info, extract_event_name};

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Individual swimmer within a relay team
#[derive(Debug, Clone, Serialize)]
pub struct RelaySwimmer {
    pub name: String,
    pub year: String,
    pub reaction_time: Option<String>,
}

/// Relay team result
#[derive(Debug, Clone, Serialize)]
pub struct RelayTeam {
    pub place: u8,
    pub team_name: String,
    pub seed_time: Option<String>,
    pub final_time: String,
    pub swimmers: Vec<RelaySwimmer>,
    #[serde(skip)]
    pub splits: Vec<Split>,
}

/// Complete relay event results with metadata
#[derive(Debug)]
pub struct RelayResults {
    pub event_name: String,
    pub session: char,
    pub metadata: Option<EventMetadata>,
    pub race_info: Option<RaceInfo>,
    pub teams: Vec<RelayTeam>,
}

// ============================================================================
// MAIN PROCESSING
// ============================================================================

/// Process a single relay event URL - fetches and parses
pub async fn process_relay_event(url: &str, session: char) -> Result<RelayResults, Box<dyn Error>> {
    let html = fetch_html(url).await?;
    let event_name = extract_event_name(&html)
        .ok_or("Could not find event name in page")?;

    let metadata = parse_event_metadata(&html);
    let race_info = parse_race_info(&event_name);

    parse_relay_event_html(&html, &event_name, session, metadata, race_info)
}

/// Parse relay event HTML
pub fn parse_relay_event_html(
    html: &str,
    event_name: &str,
    session: char,
    metadata: Option<EventMetadata>,
    race_info: Option<RaceInfo>,
) -> Result<RelayResults, Box<dyn Error>> {
    let document = Html::parse_document(html);
    let mut teams = Vec::new();

    let pre_selector = Selector::parse("pre").unwrap();
    if let Some(pre) = document.select(&pre_selector).next() {
        let content = pre.text().collect::<String>();
        let lines: Vec<&str> = content.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let current_line = lines[i].trim();

            // Check if this is a team result line (starts with place number)
            if is_relay_team_line(current_line) {
                // Find the next team line or end of content
                let mut next_team_line_idx = i + 1;
                while next_team_line_idx < lines.len() {
                    let next_line = lines[next_team_line_idx].trim();
                    if !next_line.is_empty() && is_relay_team_line(next_line) {
                        break;
                    }
                    next_team_line_idx += 1;
                }

                // Parse team section
                if let Some(team) = parse_relay_team_section(&lines[i..next_team_line_idx]) {
                    teams.push(team);
                }

                i = next_team_line_idx;
                continue;
            }
            i += 1;
        }
    }

    Ok(RelayResults {
        event_name: event_name.to_string(),
        session,
        metadata,
        race_info,
        teams,
    })
}

// ============================================================================
// TEAM PARSING
// ============================================================================

/// Check if a line is a relay team result line (place number + team name)
/// Team lines: "1 Florida                             1:21.66    1:20.15N  40"
/// Not team lines: swimmer lines starting with "1)", split lines
fn is_relay_team_line(line: &str) -> bool {
    let first_token = line.split_whitespace().next();
    match first_token {
        Some(token) => {
            // Must be purely digits (place number)
            // And the line must NOT contain ") " which indicates swimmer lines like "1) Name"
            token.chars().all(|c| c.is_ascii_digit()) && !line.contains(") ")
        }
        None => false,
    }
}

/// Parse a relay team section (main line + swimmer lines + split lines)
fn parse_relay_team_section(lines: &[&str]) -> Option<RelayTeam> {
    let main_line = lines[0].trim();

    // Parse main line: place team_name seed_time final_time points
    // Example: "1 Florida                             1:21.66    1:20.15N  40"
    let parts: Vec<&str> = main_line.split_whitespace().collect();

    if parts.len() < 4 {
        return None;
    }

    let place: u8 = parts[0].parse().ok()?;

    // Check for DQ entries
    if main_line.contains("DQ") || main_line.contains("DFS") {
        // Skip disqualified teams for now
        return None;
    }

    // Work backwards: points, final_time, seed_time
    // Note: some entries might not have points (exhibitions)
    let last = parts.last()?;

    let (final_time, seed_time) = if last.parse::<u8>().is_ok() {
        // Last is points
        let final_time = parts[parts.len() - 2];
        let seed_time = parts[parts.len() - 3];
        (final_time, Some(seed_time.to_string()))
    } else {
        // No points, last is final_time
        let final_time = *last;
        let seed_time = if parts.len() > 2 {
            Some(parts[parts.len() - 2].to_string())
        } else {
            None
        };
        (final_time, seed_time)
    };

    // Team name is everything between place and seed_time
    let team_end = parts.len() - if last.parse::<u8>().is_ok() { 3 } else { 2 };
    let team_name = parts[1..team_end].join(" ");

    // Parse swimmers from subsequent lines
    let swimmers = parse_relay_swimmers(&lines[1..]);

    // Parse splits (reaction time for swimmer 1 is in splits)
    let (first_swimmer_reaction, splits) = parse_relay_splits(&lines[1..]);

    // Update first swimmer's reaction time if found
    let mut swimmers = swimmers;
    if !swimmers.is_empty() {
        swimmers[0].reaction_time = first_swimmer_reaction;
    }

    Some(RelayTeam {
        place,
        team_name,
        seed_time,
        final_time: final_time.to_string(),
        swimmers,
        splits,
    })
}

/// Parse relay swimmers from swimmer lines
/// Format: "1) Chaney, Adam SR               2) r:0.18 Smith, Julian JR"
///         "3) r:0.19 Liendo, Josh SO        4) r:0.07 McDuff, Macguire JR"
fn parse_relay_swimmers(lines: &[&str]) -> Vec<RelaySwimmer> {
    let mut swimmers: Vec<RelaySwimmer> = vec![
        RelaySwimmer { name: String::new(), year: String::new(), reaction_time: None },
        RelaySwimmer { name: String::new(), year: String::new(), reaction_time: None },
        RelaySwimmer { name: String::new(), year: String::new(), reaction_time: None },
        RelaySwimmer { name: String::new(), year: String::new(), reaction_time: None },
    ];

    for line in lines {
        let line = line.trim();

        // Skip lines that look like split lines (no alphabetic characters except 'r' for reaction)
        // Swimmer lines contain names with letters, split lines are mostly digits and punctuation
        let has_name_chars = line.chars().any(|c| c.is_ascii_alphabetic() && c != 'r');
        if !has_name_chars {
            continue;
        }

        // Skip lines that don't start with a swimmer marker pattern "N) "
        // This avoids matching "3)" inside parenthetical times like "(9.93)"
        if !line.starts_with("1)") && !line.starts_with("2)") &&
           !line.starts_with("3)") && !line.starts_with("4)") {
            continue;
        }

        // Look for swimmer markers: "1)", "2)", "3)", "4)"
        for swimmer_num in 1..=4 {
            let marker = format!("{})", swimmer_num);

            // Only match markers at the start of a section (preceded by whitespace or start of line)
            let search_pattern = format!("{}) ", swimmer_num);
            if let Some(pos) = line.find(&search_pattern) {
                // Make sure this is a real marker (at start or after whitespace)
                if pos > 0 && !line[..pos].ends_with(char::is_whitespace) {
                    continue;
                }

                // Extract the portion after this marker until the next marker or end
                let after_marker = &line[pos + marker.len()..];

                // Find end of this swimmer's info (next marker with space or end of line)
                let end_pos = (2..=4)
                    .filter(|&n| n > swimmer_num)
                    .filter_map(|n| after_marker.find(&format!("{}) ", n)))
                    .min()
                    .unwrap_or(after_marker.len());

                let swimmer_text = after_marker[..end_pos].trim();

                if let Some(swimmer) = parse_single_relay_swimmer(swimmer_text, swimmer_num) {
                    swimmers[swimmer_num - 1] = swimmer;
                }
            }
        }
    }

    swimmers
}

/// Parse a single swimmer's info from text like "r:0.18 Smith, Julian JR" or "Chaney, Adam SR"
fn parse_single_relay_swimmer(text: &str, swimmer_num: usize) -> Option<RelaySwimmer> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let mut reaction_time: Option<String> = None;
    let mut start_idx = 0;

    // Check if first part is reaction time (swimmers 2-4 have it before their name)
    if swimmer_num > 1 && parts[0].starts_with('r') {
        reaction_time = Some(parts[0].to_string());
        start_idx = 1;
    }

    if start_idx >= parts.len() {
        return None;
    }

    // Find year pattern to determine where name ends
    let mut year_idx = None;
    for i in start_idx..parts.len() {
        if is_relay_year_pattern(parts[i]) {
            year_idx = Some(i);
            break;
        }
    }

    let (name, year) = if let Some(yi) = year_idx {
        let name = parts[start_idx..yi].join(" ");
        let year = parts[yi].to_string();
        (name, year)
    } else {
        // No year found, take all remaining as name
        let name = parts[start_idx..].join(" ");
        (name, String::new())
    };

    Some(RelaySwimmer {
        name,
        year,
        reaction_time,
    })
}

/// Check if string is a year pattern (FR, SO, JR, SR, etc.)
fn is_relay_year_pattern(s: &str) -> bool {
    if s.len() != 2 {
        return false;
    }
    matches!(s.to_uppercase().as_str(), "FR" | "SO" | "JR" | "SR" | "GR" | "5Y" | "RS" | "FF")
        || s.chars().all(|c| c.is_ascii_digit())
}

/// Parse splits from relay lines, returning (first_swimmer_reaction, splits)
fn parse_relay_splits(lines: &[&str]) -> (Option<String>, Vec<Split>) {
    let mut splits = Vec::new();
    let mut first_reaction: Option<String> = None;

    for line in lines {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        // Skip swimmer lines (start with "N) " pattern)
        if line.starts_with("1)") || line.starts_with("2)") ||
           line.starts_with("3)") || line.starts_with("4)") {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();

        for part in parts.iter() {
            // Skip delta times in parentheses
            if part.starts_with('(') {
                continue;
            }

            // Check for reaction time (first swimmer's reaction is in splits)
            if part.starts_with('r') {
                if first_reaction.is_none() {
                    first_reaction = Some(part.to_string());
                }
                continue;
            }

            // Check if this is a cumulative time
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

    (first_reaction, splits)
}
