pub mod event_handler;
pub mod meet_handler;
pub mod metadata;
pub mod output;
pub mod relay_handler;

use std::error::Error;

use metadata::{parse_event_metadata, parse_race_info, extract_event_name};

// ============================================================================
// UTILITIES
// ============================================================================

/// Fetch HTML content from a URL
pub async fn fetch_html(url: &str) -> Result<String, Box<dyn Error>> {
    let response = reqwest::get(url).await?;
    Ok(response.text().await?)
}

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================
pub use meet_handler::{process_meet, parse_meet_index, validate_meet_url, validate_event_url, Meet, Event};
pub use metadata::{EventMetadata, RaceInfo};
pub use output::{print_results, print_results_with_options, write_csv, OutputOptions, write_relay_csv, print_relay_results, print_relay_results_with_options};
pub use event_handler::{parse_individual_event_html, EventResults, Swimmer, Split};
pub use relay_handler::{parse_relay_event_html, RelayResults, RelayTeam, RelaySwimmer};

// ============================================================================
// URL DETECTION
// ============================================================================

#[derive(Debug, PartialEq)]
pub enum UrlType {
    Meet,
    Event,
}

/// If a url ends in .htm it's an event, otherwise it's a meet
pub fn detect_url_type(url: &str) -> UrlType {
    if url.trim_end_matches('/').ends_with(".htm") {
        UrlType::Event
    } else {
        UrlType::Meet
    }
}

// ============================================================================
// EVENT PROCESSING (dispatcher)
// ============================================================================

/// Each parsed event is either an individual event or a relay
#[derive(Debug)]
pub enum ParsedEvent {
    Individual(EventResults),
    Relay(RelayResults),
}

/// Processes a single event URL, either individual or relay, and returns a parsed event
pub async fn process_event(url: &str, session: char) -> Result<ParsedEvent, Box<dyn Error>> {
    let html = fetch_html(url).await?;
    let event_name = extract_event_name(&html)
        .ok_or("Could not find event name in page")?;

    let metadata = parse_event_metadata(&html);
    let race_info = parse_race_info(&event_name);

    // Check if relay and dispatch accordingly
    let is_relay = race_info.as_ref().map_or(false, |info| info.is_relay);

    if is_relay {
        let result = parse_relay_event_html(&html, &event_name, session, metadata, race_info)?;
        Ok(ParsedEvent::Relay(result))
    } else {
        let result = parse_individual_event_html(&html, &event_name, session, metadata, race_info)?;
        Ok(ParsedEvent::Individual(result))
    }
}

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

/// Main entry point - detects URL type and processes accordingly
/// Returns (individual_results, relay_results)
pub async fn parse(url: &str) -> Result<(Vec<EventResults>, Vec<RelayResults>), Box<dyn Error>> {
    match detect_url_type(url) {
        UrlType::Meet => process_meet(url).await,
        UrlType::Event => {
            let session = validate_event_url(url)?;
            match process_event(url, session).await? {
                ParsedEvent::Individual(result) => Ok((vec![result], vec![])),
                ParsedEvent::Relay(result) => Ok((vec![], vec![result])),
            }
        }
    }
}
