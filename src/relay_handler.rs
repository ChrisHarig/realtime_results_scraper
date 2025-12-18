use scraper::{Html, Selector};
use serde::Serialize;
use std::error::Error;

use crate::utils::{fetch_html, is_dq_status, is_year_pattern, is_valid_time_format};
use crate::event_handler::Split;
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
    pub place: Option<u8>,
    pub team_name: String,
    pub seed_time: Option<String>,
    pub final_time: String,
    pub dq_description: Option<String>,
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

/// Fetches and parses a relay event URL.
pub async fn process_relay_event(url: &str, session: char) -> Result<RelayResults, Box<dyn Error>> {
    let html = fetch_html(url).await?;
    let event_name = extract_event_name(&html)
        .ok_or("Could not find event name in page")?;

    let metadata = parse_event_metadata(&html);
    let race_info = parse_race_info(&event_name);

    parse_relay_event_html(&html, &event_name, session, metadata, race_info)
}

/// Parses relay event HTML and extracts team results.
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

            if is_relay_team_line(current_line) {
                // Find the next team line or end of content
                let mut next_idx = i + 1;
                while next_idx < lines.len() {
                    let next_line = lines[next_idx].trim();
                    if !next_line.is_empty() && is_relay_team_line(next_line) {
                        break;
                    }
                    next_idx += 1;
                }

                if let Some(team) = parse_relay_team_section(&lines[i..next_idx]) {
                    teams.push(team);
                }

                i = next_idx;
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

/// Checks if a line starts a relay team result (place number or -- for DQ).
fn is_relay_team_line(line: &str) -> bool {
    match line.split_whitespace().next() {
        Some(token) => {
            let is_place = token.chars().all(|c| c.is_ascii_digit());
            let is_dq = token == "--";
            (is_place || is_dq) && !line.contains(") ")
        }
        None => false,
    }
}

/// Parses a relay team section (main line + swimmers + splits) into a RelayTeam.
fn parse_relay_team_section(lines: &[&str]) -> Option<RelayTeam> {
    let main_line = lines[0].trim();
    let parts: Vec<&str> = main_line.split_whitespace().collect();

    if parts.len() < 3 {
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
    let (final_time, seed_time, team_end) = if last.parse::<u8>().is_ok() {
        (parts[parts.len() - 2], Some(parts[parts.len() - 3].to_string()), parts.len() - 3)
    } else if is_dq_status(last) {
        let seed = if parts.len() > 3 {
            Some(parts[parts.len() - 2].to_string())
        } else {
            None
        };
        (*last, seed, parts.len() - 2)
    } else {
        let seed = if parts.len() > 2 {
            Some(parts[parts.len() - 2].to_string())
        } else {
            None
        };
        (*last, seed, parts.len() - 2)
    };

    let team_name = parts[1..team_end].join(" ");

    // Check for DQ description on the next line
    let dq_description = if is_dq_entry && lines.len() > 1 {
        let next_line = lines[1].trim();
        if !next_line.is_empty()
            && !next_line.starts_with("1)")
            && !next_line.starts_with("r:")
            && !next_line.starts_with("r+")
            && next_line.chars().any(|c| c.is_ascii_alphabetic())
            && !next_line.contains(") ")
        {
            Some(next_line.to_string())
        } else {
            None
        }
    } else {
        None
    };

    let swimmer_start_idx = if dq_description.is_some() { 2 } else { 1 };
    let mut swimmers = parse_relay_swimmers(&lines[swimmer_start_idx..]);
    let (first_swimmer_reaction, splits) = parse_relay_splits(&lines[swimmer_start_idx..]);

    if !swimmers.is_empty() {
        swimmers[0].reaction_time = first_swimmer_reaction;
    }

    Some(RelayTeam {
        place,
        team_name,
        seed_time,
        final_time: final_time.to_string(),
        dq_description,
        swimmers,
        splits,
    })
}

/// Extracts four swimmers from relay swimmer lines.
fn parse_relay_swimmers(lines: &[&str]) -> Vec<RelaySwimmer> {
    let mut swimmers: Vec<RelaySwimmer> = vec![
        RelaySwimmer { name: String::new(), year: String::new(), reaction_time: None },
        RelaySwimmer { name: String::new(), year: String::new(), reaction_time: None },
        RelaySwimmer { name: String::new(), year: String::new(), reaction_time: None },
        RelaySwimmer { name: String::new(), year: String::new(), reaction_time: None },
    ];

    for line in lines {
        let line = line.trim();

        // Skip split lines (no alphabetic characters except 'r')
        let has_name_chars = line.chars().any(|c| c.is_ascii_alphabetic() && c != 'r');
        if !has_name_chars {
            continue;
        }

        // Skip lines without swimmer markers
        if !line.starts_with("1)") && !line.starts_with("2)")
            && !line.starts_with("3)") && !line.starts_with("4)")
        {
            continue;
        }

        for swimmer_num in 1..=4 {
            let marker = format!("{})", swimmer_num);
            let search_pattern = format!("{}) ", swimmer_num);

            if let Some(pos) = line.find(&search_pattern) {
                if pos > 0 && !line[..pos].ends_with(char::is_whitespace) {
                    continue;
                }

                let after_marker = &line[pos + marker.len()..];
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

/// Parses a single swimmer's info (name, year, reaction time).
fn parse_single_relay_swimmer(text: &str, swimmer_num: usize) -> Option<RelaySwimmer> {
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let mut reaction_time: Option<String> = None;
    let mut start_idx = 0;

    // Swimmers 2-4 may have reaction time before name
    if swimmer_num > 1 && parts[0].starts_with('r') {
        reaction_time = Some(parts[0].to_string());
        start_idx = 1;
    }

    if start_idx >= parts.len() {
        return None;
    }

    // Find year position
    let mut year_idx = None;
    for (i, &part) in parts.iter().enumerate().skip(start_idx) {
        if is_year_pattern(part) {
            year_idx = Some(i);
            break;
        }
    }

    let (name, year) = if let Some(yi) = year_idx {
        (parts[start_idx..yi].join(" "), parts[yi].to_string())
    } else {
        (parts[start_idx..].join(" "), String::new())
    };

    Some(RelaySwimmer {
        name,
        year,
        reaction_time,
    })
}

/// Extracts first swimmer reaction time and split times from relay lines.
fn parse_relay_splits(lines: &[&str]) -> (Option<String>, Vec<Split>) {
    let mut splits = Vec::new();
    let mut first_reaction: Option<String> = None;

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Skip swimmer lines
        if line.starts_with("1)") || line.starts_with("2)")
            || line.starts_with("3)") || line.starts_with("4)")
        {
            continue;
        }

        for part in line.split_whitespace() {
            if part.starts_with('(') {
                continue;
            }

            if part.starts_with('r') {
                if first_reaction.is_none() {
                    first_reaction = Some(part.to_string());
                }
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

    (first_reaction, splits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dq_relay_team() {
        let lines = vec![
            "-- Missouri                           3:06.12         DQ",
            "      Early take-off swimmer #4",
            "    1) Bochenski, Grant SR           2) r:-0.01 Ottke, Logan SO",
            "    3) r:0.00 Zubik, Jan JR          4) r:-0.04 Nebrich, Lucas FR",
            "    r:+0.71  21.81        45.58 (45.58)",
        ];

        let team = parse_relay_team_section(&lines).expect("Should parse DQ team");

        assert_eq!(team.place, None);
        assert_eq!(team.team_name, "Missouri");
        assert_eq!(team.final_time, "DQ");
        assert_eq!(team.seed_time, Some("3:06.12".to_string()));
        assert_eq!(team.dq_description, Some("Early take-off swimmer #4".to_string()));
        assert_eq!(team.swimmers.len(), 4);
        assert_eq!(team.swimmers[0].name, "Bochenski, Grant");
    }

    #[test]
    fn test_parse_dfs_relay_team() {
        let lines = vec![
            "-- Wisconsin                          3:06.22        DFS",
            "      Declared false start - Misc",
            "    1) Lorenz, Sam FR                2) Wiegand, Ben SR",
            "    3) Jones, Charles JR             4) Morris, Christopher SR",
        ];

        let team = parse_relay_team_section(&lines).expect("Should parse DFS team");

        assert_eq!(team.place, None);
        assert_eq!(team.team_name, "Wisconsin");
        assert_eq!(team.final_time, "DFS");
        assert_eq!(team.dq_description, Some("Declared false start - Misc".to_string()));
    }

    #[test]
    fn test_parse_normal_relay_team() {
        let lines = vec![
            "1 Florida                             1:21.66    1:20.15N  40",
            "    1) Chaney, Adam SR               2) r:0.18 Smith, Julian JR",
            "    3) r:0.19 Liendo, Josh SO        4) r:0.07 McDuff, Macguire JR",
        ];

        let team = parse_relay_team_section(&lines).expect("Should parse normal team");

        assert_eq!(team.place, Some(1));
        assert_eq!(team.team_name, "Florida");
        assert_eq!(team.final_time, "1:20.15N");
        assert_eq!(team.dq_description, None);
    }

    #[test]
    fn test_is_relay_team_line_with_dq() {
        assert!(is_relay_team_line("-- Missouri                           3:06.12         DQ"));
        assert!(is_relay_team_line("1 Florida                             1:21.66    1:20.15N  40"));
        assert!(!is_relay_team_line("    1) Bochenski, Grant SR           2) r:-0.01 Ottke, Logan SO"));
        assert!(!is_relay_team_line("      Early take-off swimmer #4"));
    }
}
