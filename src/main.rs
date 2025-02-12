use std::io::{self, Write};
use scraper::{Html, Selector};
use reqwest;
use tokio;
use std::collections::HashMap;
mod page_handler;
use page_handler::{process_event_page, print_results};

// -----------------------------------------------------------------------------------------
// Processes the index page, stores each event pair, and makes calls to process each event
// -----------------------------------------------------------------------------------------

pub struct Meet {
    events: HashMap<String, Vec<Event>>,
    //add Date? Name? Location? 
}

impl Meet {
    //add event to meet
    pub fn add_event(&mut self, event: Event) {
        self.events.insert(event.name, event);
    }

    pub fn process_event(&mut self, event_key: String, event_type: char, event: Event) {
        if let Some(existing) = self.events.get_mut(&event_key) {
            // Merge the event information based on type (Prelims or Finals)
            // Call a method in Event for this
            match event_type {
                Prelims => {
                    if existing.prelims_link.is_some() {
                        eprintln!("WARNING: Duplicate prelims event found!");
                        eprintln!("  Event Number: {}", event_key);
                        eprintln!("  Existing event: {:?}", existing);
                        eprintln!("  New event: {:?}", event);
                    } else {
                        existing.prelims_link = event.prelims_link;
                    }
                },
                Finals => {
                    if existing.finals_link.is_some() {
                        eprintln!("WARNING: Duplicate finals event found!");
                        eprintln!("  Event Number: {}", event_key);
                        eprintln!("  Existing event: {:?}", existing);
                        eprintln!("  New event: {:?}", event);
                    } else {
                        existing.finals_link = event.finals_link;
                    }
                },
                _ => unreachable!(), // We validated event_type earlier
            }
        } else {
            events.insert(event_key, event);
        }
    }

    pub fn print_events(&mut self) {
        let mut event_nums: Vec<_> = self.events.keys().collect();
        event_nums.sort_by(|a, b| a.parse::<i32>().unwrap_or(0).cmp(&b.parse::<i32>().unwrap_or(0)));
        
        for event_num in event_nums {
            if let Some(event) = events.get(event_num) {
                println!("Event {}: {}", event_num, event.name);
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

}

pub enum Event_Type {
    Prelims,
    Finals,
}

#[derive(Default)]
pub struct Event {
    name: String,
    number: u32,
    type: Event_Type,
    prelims_link: Option<String>,  // Some events might not have prelims
    finals_link: Option<String>,
}

impl Event {

    /// Returns true if the event has a prelims link
    pub fn has_prelims(&self) -> bool {
        self.prelims_link.is_some()
    }

    /// Returns true if the event has a finals link 
    pub fn has_finals(&self) -> bool {
        self.finals_link.is_some()
    }
}

pub struct Raw_Event {
    link: String,
}

impl Raw_Event {

    pub fn set_link(&mut self, link: String) {
        self.link = link;
    }

    /// Gets event informating from link, if link is invlaid at any point, returns None
    pub fn parse_event_info_from_link(&self) -> Result<Event, Box<dyn std::error::Error>> {
        let href = self.link;
        
        let event = Event::default();

        if !href.ends_with(".htm") {
            return None;
        }

        let event_code = href.trim_end_matches(".htm");
        
        // Validate the last 4 characters follow the pattern 'P' or 'F' followed by 3 digits
        let last_four = &event_code[event_code.len().saturating_sub(4)..];
        let event_type = last_four.chars().next()?;

        // Check if event is a valid prelims or finals event, ignore if not
        if !(event_type == 'P' || event_type == 'F') || !last_four[1..].chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        
        // Get the event number (the 3 digits after P/F)
        let event_num = &last_four[1..];
        
        // Parse the event name, removing "Prelims" or "Finals" if present
        let event_name = event_text
            .split_once(' ')
            .map(|(_, rest)| rest.trim())
            .unwrap_or(event_text)
            .replace(" Prelims", "")
            .replace(" Finals", "");

        // Create full URLs by combining base_url with href
        let full_url = format!("{}/{}", base_url, href);

        let (prelims_link, finals_link) = if event_type == 'P' {
            (Some(full_url), None)
        } else {
            (None, Some(full_url))
        };

        Some((
            Event {
                name: event_name.to_string(),
                number: event_num.to_string(),
                type: event_type,
                prelims_link,
                finals_link,
            }
        ))
    }

}

//NON-IMPLEMENTED
// Fetches the HTML from the given URL, should be an index page
async fn fetch_html(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    Ok(response.text().await?)
}


//MAKE-IMPLEMENTED
// Print out each event and each of it's endpoints (Prelims and Finals)

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut meet = Meet::new();
    let mut events = meet.events;

    // Base URL for the meet results
    let base_url = "https://swimmeetresults.tech/NCAA-Division-I-Men-2024"; 
    
    // Get the index page
    let index_url = format!("{}/evtindex.htm", base_url);
    
    // Fetch and parse index page
    let index_html = fetch_html(&index_url).await?;
    let index_document = Html::parse_document(&index_html);
    
    // Select all links in the index
    let link_selector = Selector::parse("a").unwrap();
    
    //DECIDE ON IMPLEMENTATION
    // Process each link
    for link in index_document.select(&link_selector) {
        if let Some(href) = link.value().attr("href") {
            if let Some(event_text) = link.text().next() {
                if let Some((event_key, event_type, event)) = parse_event_from_link(href, &event_text, base_url) {
                    process_events(&mut events, event_key, event_type, event);
                }
            }
        }
    }
    
    print_events(&events);
    
    //MAKE-IMPLEMENTED
    println!("\nProcessing individual event pages...");
    for (event_num, event) in &events {
        // Process finals if available, otherwise prelims
        if let Some(link) = &event.finals_link {
            let results = process_event_page(&event.name, link, 'F').await?;
            print_results(&results);
        } else if let Some(link) = &event.prelims_link {
            let results = process_event_page(&event.name, link, 'P').await?;
            print_results(&results);
        }
        
        // Add a small delay between requests to be nice to the server
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    Ok(())
}


