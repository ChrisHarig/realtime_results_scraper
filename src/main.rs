use std::io::{self, Write};
use scraper::{Html, Selector, ElementRef};
use reqwest;
use tokio;
use std::collections::HashMap;
mod page_handler;
use page_handler::{process_event_page, print_results};

// -----------------------------------------------------------------------------------------
// Processes the index page, stores each event pair, and makes calls to process each event
// -----------------------------------------------------------------------------------------

pub struct Meet { //give meet the base url that it can pass down toe each event to create the full url
    events: HashMap<String, Vec<Event>>,
    base_url: String,
    //add Date? Name? Location? 
}

impl Meet {
    //add event to meet
    pub fn new(events: HashMap<String, Vec<Event>>, base_url: String) -> Meet {
        Meet {events, base_url}
    }

    /// Adds an event to the meet or raises error if there is a duplicate event 
    pub fn process_event(&mut self, event: Event) {
        let event_name = event.name;
        if let Some(existing) = self.events.get_mut(&event.name) {
            // Merge the event information based on type (Prelims or Finals)
            // Call a method in Event for this
            match event.e_type {
                Event_Type::Prelims => {
                    if existing.prelims_link.is_some() {
                        eprintln!("WARNING: Duplicate prelims event found!");
                        eprintln!("  Event Number: {}", event_name);
                        eprintln!("  Existing event: {:?}", existing);
                        eprintln!("  New event: {:?}", event);
                    } else {
                        existing.prelims_link = event.prelims_link;
                    }
                },
                Event_Type::Finals => {
                    if existing.finals_link.is_some() {
                        eprintln!("WARNING: Duplicate finals event found!");
                        eprintln!("  Event Number: {}", event_name);
                        eprintln!("  Existing event: {:?}", existing);
                        eprintln!("  New event: {:?}", event);
                    } else {
                        existing.finals_link = event.finals_link;
                    }
                },
                _ => unreachable!(), // We validated event_type earlier
            }
        } else {
            self.events.insert(event.name, event);
        }
    }

    pub fn print_events(&mut self) {
        let mut event_nums: Vec<_> = self.events.keys().collect();
        event_nums.sort_by(|a, b| a.parse::<i32>().unwrap_or(0).cmp(&b.parse::<i32>().unwrap_or(0)));
        
        for event_num in event_nums {
            if let Some(event) = self.events.get(event_num) {
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

pub struct Raw_Event {
    link: ElementRef<'_>,
    href: String,
    text: String,
}

impl Raw_Event {
    /// Creates a new Raw_Event if the link has valid href and text elements
    pub fn new(link: ElementRef<'_>) -> Option<Self> {
        // Extract href and text
        let href = link.value().attr("href")?.to_string();
        let text = link.text().next()?.to_string();
        
        // Validate link is of the correct type
        if !href.ends_with(".htm") {
            return None;
        }
        
        Some(Raw_Event { 
            link, 
            href, 
            text, 
        })
    }
}

#[derive(Default)]
pub struct Event {
    name: String,
    number: u32,
    e_type: Event_Type,
    prelims_link: Option<String>,  // Some events might not have prelims
    finals_link: Option<String>,
}

impl Event {
    pub fn new(name: String, number: u32, e_type: Event_Type, prelims_link: Option<String>, finals_link: Option<String>) -> Event {
        Event {name, number, e_type, prelims_link, finals_link}
    }
    
    /// Creates an Event from a Raw_Event, validating event type and number
    pub fn from_raw_event(raw_event: &Raw_Event, base_url: &str) -> Option<Self> {
        let event_code = raw_event.href.trim_end_matches(".htm");
        
        // Validate the last 4 characters follow the pattern 'P' or 'F' followed by 3 digits
        let last_four = &event_code[event_code.len().saturating_sub(4)..];
        let event_type = last_four.chars().next()?;

        // Check if event is a valid prelims or finals event, ignore if not
        if !(event_type == 'P' || event_type == 'F') || !last_four[1..].chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        
        // Get the event number (the 3 digits after P/F)
        let event_num = match last_four[1..].parse::<u32>() {
            Ok(num) => num,
            Err(_) => return None,
        };

        // Parse the event name, removing "Prelims" or "Finals" if present
        let event_name = raw_event.text
            .split_once(' ')
            .map(|(_, rest)| rest.trim())
            .unwrap_or(&raw_event.text)
            .replace(" Prelims", "")
            .replace(" Finals", "");

        // Create full URL by combining base_url with href
        let full_url = format!("{}/{}", base_url, &raw_event.href);

        // Create the event with the appropriate type
        let e_type = if event_type == 'P' { 
            Event_Type::Prelims 
        } else { 
            Event_Type::Finals 
        };
        
        let (prelims_link, finals_link) = if event_type == 'P' {
            (Some(full_url), None)
        } else {
            (None, Some(full_url))
        };

        Some(Event {
            name: event_name.to_string(),
            number: event_num,
            e_type,
            prelims_link,
            finals_link,
        })
    }
}

// Fetches the HTML from the given URL, should be an index page
async fn fetch_html(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    Ok(response.text().await?)
}


//MAKE-IMPLEMENTED
// Print out each event and each of it's endpoints (Prelims and Finals)

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Base URL for the meet results (will eventually be passed in by a user)
    let base_url = "https://swimmeetresults.tech/NCAA-Division-I-Men-2024"; 

    let mut meet = Meet::new(HashMap::new(), String::from(base_url));
    
    // Get the index page
    let index_url = format!("{}/evtindex.htm", base_url);
    
    // Fetch and parse index page
    let index_html = fetch_html(&index_url).await?;
    let index_document = Html::parse_document(&index_html);
    
    // Select all links in the index
    let link_selector = Selector::parse("a").unwrap();
    
    // Process each link
    for link in index_document.select(&link_selector) {
        // Create a Raw_Event if the link has valid href and text
        if let Some(raw_event) = Raw_Event::new(link) {
            // Create an Event from the Raw_Event, validating event type and number
            if let Some(event) = Event::from_raw_event(&raw_event, base_url) {
                meet.process_event(event);
            }
        }
    }
    
    meet.print_events();
    
    println!("\nProcessing individual event pages...");
    for event_num in meet.events.keys() {
        if let Some(event) = meet.events.get(event_num) {
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
    }

    Ok(())
}


