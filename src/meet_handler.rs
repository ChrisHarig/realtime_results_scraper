use scraper::{Html, Selector, ElementRef};
use std::collections::HashMap;
use std::error::Error;

use crate::utils::fetch_html;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Meet containing all events, base URL, and meet title
pub struct Meet {
    pub events: HashMap<String, Event>,
    pub base_url: String,
    pub title: Option<String>,
}

/// Event with links to prelims and finals pages
pub struct Event {
    pub name: String,
    pub number: u32,
    pub prelims_link: Option<String>,
    pub finals_link: Option<String>,
}

/// Parsed event link from index page
struct EventLink {
    href: String,
    event_name: String,
    event_num: u32,
    session: char,
}

impl Meet {
    /// Creates a new Meet with the given base URL
    pub fn new(base_url: String) -> Meet {
        Meet {
            events: HashMap::new(),
            base_url,
            title: None,
        }
    }

    /// Sets the meet title
    pub fn set_title(&mut self, title: String) {
        self.title = Some(title);
    }

    /// Adds an event to the meet
    pub fn add_event(&mut self, name: String, event: Event) {
        self.events.insert(name, event);
    }

    /// Returns a mutable reference to an event by name
    pub fn get_event_mut(&mut self, name: &str) -> Option<&mut Event> {
        self.events.get_mut(name)
    }
}

impl Event {
    /// Creates a new Event with name and number
    pub fn new(name: String, number: u32) -> Event {
        Event {
            name,
            number,
            prelims_link: None,
            finals_link: None,
        }
    }

    /// Sets the prelims or finals link based on session
    pub fn set_link(&mut self, link: String, session: char) {
        match session {
            'P' => self.prelims_link = Some(link),
            'F' => self.finals_link = Some(link),
            _ => {}
        }
    }
}

impl EventLink {
    /// Extracts event info from an index page link element
    fn from_element(link: ElementRef) -> Option<Self> {
        let href = link.value().attr("href")?.to_string();
        let text = link.text().next()?.to_string();

        if !href.ends_with(".htm") {
            return None;
        }

        let code = href.trim_end_matches(".htm");
        if code.len() < 4 {
            return None;
        }

        let session = code.chars().nth(code.len() - 4)?;
        if session != 'P' && session != 'F' {
            return None;
        }

        let event_num = code[code.len() - 3..].parse().unwrap_or(0);

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
// MEET INDEX PARSING
// ============================================================================

/// Extracts the meet title from the index page HTML
fn extract_meet_title(html: &str) -> Option<String> {
    let document = Html::parse_document(html);

    // Try to find title in first <h2> tag
    let h2_selector = Selector::parse("h2").ok()?;
    if let Some(h2) = document.select(&h2_selector).next() {
        let text = h2.text().collect::<String>();
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    // Fallback: try <pre> tag
    let pre_selector = Selector::parse("pre").ok()?;
    if let Some(pre) = document.select(&pre_selector).next() {
        let content = pre.text().collect::<String>();
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('-') || trimmed.starts_with('=') {
                continue;
            }
            if !trimmed.is_empty() && !trimmed.to_lowercase().contains("event") {
                return Some(trimmed.to_string());
            }
        }
    }

    // Fallback: try HTML title tag
    let title_selector = Selector::parse("title").ok()?;
    if let Some(title) = document.select(&title_selector).next() {
        let text = title.text().collect::<String>();
        if !text.is_empty() {
            return Some(text.trim().to_string());
        }
    }

    None
}

/// Fetches and parses a meet index page, returning a Meet with all event links
pub async fn parse_meet_index(url: &str) -> Result<Meet, Box<dyn Error>> {
    let url = url.trim_end_matches('/');
    let mut meet = Meet::new(url.to_string());

    let index_url = format!("{}/evtindex.htm", url);
    let html = fetch_html(&index_url).await?;

    // Extract meet title
    if let Some(title) = extract_meet_title(&html) {
        meet.set_title(title);
    }

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
