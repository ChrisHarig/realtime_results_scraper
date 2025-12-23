pub mod event_handler;
pub mod meet_handler;
pub mod metadata;
pub mod output;
pub mod relay_handler;
pub mod utils;

use std::error::Error;
use futures::future::join_all;

use metadata::{parse_event_metadata, parse_race_info};
use utils::{fetch_html, extract_session_from_url};

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================

pub use meet_handler::{parse_meet_index, Meet, Event};
pub use metadata::{EventMetadata, RaceInfo};
pub use output::{print_individual_results, write_individual_csv, write_relay_csv, print_relay_results, write_metadata_csv, write_results_to_folders, OutputOptions};
pub use event_handler::{parse_individual_event_html, EventResults, Swimmer, Split};
pub use relay_handler::{parse_relay_event_html, RelayResults, RelayTeam, RelaySwimmer};
pub use utils::{generate_unique_id, sanitize_name};

// ============================================================================
// PARSED RESULTS
// ============================================================================

/// Complete parsed results with optional meet info
#[derive(Debug)]
pub struct ParsedResults {
    pub individual_results: Vec<EventResults>,
    pub relay_results: Vec<RelayResults>,
    pub meet_title: Option<String>,
}

// ============================================================================
// URL DETECTION
// ============================================================================

/// URL type for routing to appropriate parser
#[derive(Debug, PartialEq)]
pub enum UrlType {
    Meet,
    Event,
}

/// Detects if a URL points to a meet index or individual event
pub fn detect_url_type(url: &str) -> UrlType {
    if url.trim_end_matches('/').ends_with(".htm") {
        UrlType::Event
    } else {
        UrlType::Meet
    }
}

// ============================================================================
// EVENT PROCESSING
// ============================================================================

/// Parsed event result (individual or relay)
#[derive(Debug)]
pub enum ParsedEvent {
    Individual(EventResults),
    Relay(RelayResults),
}

/// Fetches and parses a single event URL, dispatching to individual or relay parser
pub async fn process_event(url: &str, session: char) -> Result<ParsedEvent, Box<dyn Error>> {
    let html = fetch_html(url).await?;
    let metadata = parse_event_metadata(&html).ok_or_else(|| {
        eprintln!("Error: Could not parse event metadata from page");
        "Could not find event metadata in page"
    })?;
    let event_name = metadata.event_headline.clone();
    let race_info = parse_race_info(&event_name);
    let is_relay = race_info.as_ref().is_some_and(|info| info.is_relay);

    if is_relay {
        let result = parse_relay_event_html(&html, &event_name, session, Some(metadata), race_info)?;
        Ok(ParsedEvent::Relay(result))
    } else {
        let result = parse_individual_event_html(&html, &event_name, session, Some(metadata), race_info)?;
        Ok(ParsedEvent::Individual(result))
    }
}

// ============================================================================
// MEET PROCESSING
// ============================================================================

/// Fetches and parses all events in a meet, returning individual and relay results with meet info
pub async fn process_meet(url: &str) -> Result<ParsedResults, Box<dyn Error>> {
    let meet = parse_meet_index(url).await?;
    let meet_title = meet.title.clone();

    let event_tasks: Vec<(String, String, char)> = meet.events.iter()
        .flat_map(|(_, event)| {
            [(&event.prelims_link, 'P'), (&event.finals_link, 'F')]
                .into_iter()
                .filter_map(|(link, session)| {
                    link.as_ref().map(|l| (event.name.clone(), l.clone(), session))
                })
        })
        .collect();

    let futures: Vec<_> = event_tasks.iter()
        .map(|(_, link, session)| process_event(link, *session))
        .collect();

    let results = join_all(futures).await;

    let mut individual_results = Vec::new();
    let mut relay_results = Vec::new();

    for (i, result) in results.into_iter().enumerate() {
        let event_name = &event_tasks[i].0;
        match result {
            Ok(ParsedEvent::Individual(er)) => individual_results.push(er),
            Ok(ParsedEvent::Relay(rr)) => relay_results.push(rr),
            Err(e) => {
                eprintln!("Error processing {}: {}", event_name, e);
            }
        }
    }

    Ok(ParsedResults {
        individual_results,
        relay_results,
        meet_title,
    })
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

/// Parses a meet or event URL, returning individual and relay results with meet info
pub async fn parse(url: &str) -> Result<ParsedResults, Box<dyn Error>> {
    match detect_url_type(url) {
        UrlType::Meet => process_meet(url).await,
        UrlType::Event => {
            let session = extract_session_from_url(url).ok_or_else(|| {
                eprintln!("Error: Could not determine session (P/F) from URL: {}", url);
                "Could not determine session (P/F) from URL"
            })?;
            match process_event(url, session).await? {
                ParsedEvent::Individual(result) => {
                    let meet_title = result.metadata.as_ref()
                        .and_then(|m| m.meet_name.clone());
                    Ok(ParsedResults {
                        individual_results: vec![result],
                        relay_results: vec![],
                        meet_title,
                    })
                },
                ParsedEvent::Relay(result) => {
                    let meet_title = result.metadata.as_ref()
                        .and_then(|m| m.meet_name.clone());
                    Ok(ParsedResults {
                        individual_results: vec![],
                        relay_results: vec![result],
                        meet_title,
                    })
                },
            }
        }
    }
}
