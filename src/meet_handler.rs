use scraper::{Html, Selector, ElementRef};
use std::collections::HashMap;
use std::error::Error;
use futures::future::join_all;

use crate::{process_event, ParsedEvent, fetch_html};
use crate::event_handler::EventResults;
use crate::relay_handler::RelayResults;

const VALID_DOMAIN: &str = "swimmeetresults.tech";

// ============================================================================
// DATA STRUCTURES
// ============================================================================

// Meet structure - contains all events and meets page
pub struct Meet {
    pub events: HashMap<String, Event>,
    pub base_url: String,
}

// Event within a meet (links to prelims/finals pages)
pub struct Event {
    pub name: String,
    pub number: u32,
    pub prelims_link: Option<String>,
    pub finals_link: Option<String>,
}

// Parsed info from an event link on the index page
struct EventLink {
    href: String,
    event_name: String,
    event_num: u32,
    session: char,
}

impl Meet {
    pub fn new(base_url: String) -> Meet {
        Meet {
            events: HashMap::new(),
            base_url,
        }
    }

    pub fn add_event(&mut self, name: String, event: Event) {
        self.events.insert(name, event);
    }

    pub fn get_event_mut(&mut self, name: &str) -> Option<&mut Event> {
        self.events.get_mut(name)
    }

    pub fn print_events(&self) {
        for (name, event) in &self.events {
            println!("Event {}: {}", event.number, name);
            if let Some(prelims) = &event.prelims_link {
                println!("  Prelims: {}", prelims);
            }
            if let Some(finals) = &event.finals_link {
                println!("  Finals: {}", finals);
            }
            println!();
        }
    }
}

impl Event {
    pub fn new(name: String, number: u32) -> Event {
        Event {
            name,
            number,
            prelims_link: None,
            finals_link: None,
        }
    }

    pub fn set_link(&mut self, link: String, session: char) {
        match session {
            'P' => self.prelims_link = Some(link),
            'F' => self.finals_link = Some(link),
            _ => eprintln!("WARNING: Invalid session '{}'", session),
        }
    }
}

impl EventLink {
    /// Extract event info from an index page link. Returns None for non-event links.
    fn from_element(link: ElementRef) -> Option<Self> {
        let href = link.value().attr("href")?.to_string();
        let text = link.text().next()?.to_string();

        // Must be a .htm file
        if !href.ends_with(".htm") {
            eprintln!("Skipping non-htm link: {}", href);
            return None;
        }

        let code = href.trim_end_matches(".htm");
        if code.len() < 4 {
            eprintln!("Skipping link with short code: {}", href);
            return None;
        }

        // Check for session type (P or F) at expected position
        let session = code.chars().nth(code.len() - 4)?;
        if session != 'P' && session != 'F' {
            eprintln!("Skipping link without P/F session marker: {}", href);
            return None;
        }

        // Extract event number (used for display)
        let event_num = code[code.len() - 3..].parse().unwrap_or(0);

        // Clean up event name: remove "Event X" prefix and "Prelims"/"Finals" suffix
        let event_name = text
            .split_once(' ')
            .map(|(_, rest)| rest.trim())
            .unwrap_or(&text)
            .replace(" Prelims", "")
            .replace(" Finals", "");

        Some(EventLink { href, event_name, event_num, session })
    }
}

// ============================================================================
// URL VALIDATION
// ============================================================================

/// Validates an event URL has correct structure, returns session type (P or F)
pub fn validate_event_url(url: &str) -> Result<char, Box<dyn Error>> {
    let url = url.trim_end_matches('/');

    if !url.contains(VALID_DOMAIN) {
        return Err(format!("Invalid domain - expected {}", VALID_DOMAIN).into());
    }

    if !url.ends_with(".htm") {
        return Err("Event URL must end with .htm".into());
    }

    let filename = url.rsplit('/').next().unwrap_or("");
    let code = filename.trim_end_matches(".htm");

    if code.len() < 10 {
        return Err(format!(
            "Invalid event filename '{}' - expected pattern like 240327F003.htm",
            filename
        ).into());
    }

    let session = code.chars().nth(code.len() - 4).unwrap_or('?');
    if session != 'P' && session != 'F' {
        return Err(format!(
            "Invalid session type '{}' in filename - expected 'P' (prelims) or 'F' (finals)",
            session
        ).into());
    }

    let event_num = &code[code.len() - 3..];
    if !event_num.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!(
            "Invalid event number '{}' - expected 3 digits",
            event_num
        ).into());
    }

    Ok(session)
}

/// Validates a meet URL has correct structure
pub fn validate_meet_url(url: &str) -> Result<(), Box<dyn Error>> {
    let url = url.trim_end_matches('/');

    if !url.contains(VALID_DOMAIN) {
        return Err(format!("Invalid domain - expected {}", VALID_DOMAIN).into());
    }

    if url.ends_with(".htm") {
        return Err("Meet URL should not end with .htm - use an event URL instead".into());
    }

    let after_domain = url.split(VALID_DOMAIN).nth(1).unwrap_or("");
    if after_domain.trim_matches('/').is_empty() {
        return Err("Missing meet name in URL - expected format: https://swimmeetresults.tech/Meet-Name".into());
    }

    Ok(())
}

// ============================================================================
// MEET PROCESSING
// ============================================================================

/// Parse a meet index page and return the Meet structure with all event links
pub async fn parse_meet_index(url: &str) -> Result<Meet, Box<dyn Error>> {
    let url = url.trim_end_matches('/');
    let mut meet = Meet::new(url.to_string());

    let index_url = format!("{}/evtindex.htm", url);
    let html = fetch_html(&index_url).await?;
    let document = Html::parse_document(&html);
    let selector = Selector::parse("a").unwrap();

    for link in document.select(&selector) {
        if let Some(event_link) = EventLink::from_element(link) {
            let full_url = format!("{}/{}", url, event_link.href);

            if let Some(event) = meet.get_event_mut(&event_link.event_name) {
                event.set_link(full_url, event_link.session);
            } else {
                let mut event = Event::new(event_link.event_name.clone(), event_link.event_num);
                event.set_link(full_url, event_link.session);
                meet.add_event(event_link.event_name, event);
            }
        }
    }

    Ok(meet)
}

/// Process all events in a meet and return results (individual events and relay events)
pub async fn process_meet(url: &str) -> Result<(Vec<EventResults>, Vec<RelayResults>), Box<dyn Error>> {
    validate_meet_url(url)?;

    let meet = parse_meet_index(url).await?;

    // Collect all event tasks
    let event_tasks: Vec<(String, String, char)> = meet.events.iter()
        .flat_map(|(_, event)| {
            [(&event.prelims_link, 'P'), (&event.finals_link, 'F')]
                .into_iter()
                .filter_map(|(link, session)| {
                    link.as_ref().map(|l| (event.name.clone(), l.clone(), session))
                })
        })
        .collect();

    // Process all events in parallel - process_event handles relay detection
    let futures: Vec<_> = event_tasks.iter()
        .map(|(_, link, session)| process_event(link, *session))
        .collect();

    let results = join_all(futures).await;

    // Separate results by type
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

    Ok((individual_results, relay_results))
}
