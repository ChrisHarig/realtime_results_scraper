use scraper::{Html, Selector, ElementRef};
use reqwest;
use tokio;
use std::collections::HashMap;
mod page_handler;
use page_handler::{process_event_page, print_results};
use std::fmt;

// -----------------------------------------------------------------------------------------
// Processes the index page, stores each event pair, and makes calls to process each event
// ------------------------------------------------------------------------------------------

pub struct Meet { //give meet the base url that it can pass down to each event to create the full url
    events: HashMap<String, Event>,
    base_url: String,
    //add Date? Name? Location? 
}

impl Meet {
    //add event to meet
    pub fn new(events: HashMap<String, Event>, base_url: String) -> Meet {
        Meet {events, base_url}
    }

    pub fn append_base_url(&self, event: Event) -> Event {
        //Choose to use this or not
    }
    /// Adds an event to the meet or raises error if there is a duplicate event ///---MOVE LINK VALIDATION TO EVENT---///
    pub fn process_event(&mut self, event: Event) {
        let event_name = event.name.clone();
        if let Some(existing) = self.events.get_mut(&event_name) {
            // Merge the event information based on type (Prelims or Finals)
            match existing.prelims_link.is_some() {
                true => {
                    eprintln!("WARNING: Duplicate prelims event found!");
                    eprintln!("  Event Number: {}", event_name);
                    eprintln!("  Existing event: {}", existing);
                    eprintln!("  New event: {}", event);
                },
                false => {
                    existing.prelims_link = event.prelims_link;
                }
            }
            match existing.finals_link.is_some() {
                true => {
                    eprintln!("WARNING: Duplicate finals event found!");
                    eprintln!("  Event Number: {}", event_name);
                    eprintln!("  Existing event: {}", existing);
                    eprintln!("  New event: {}", event);
                },
                false => {
                    existing.finals_link = event.finals_link;
                }
            }
        } else {
            self.events.insert(event_name, event);
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

pub struct RawEvent<'a> {
    link: ElementRef<'a>,
    href: String,
    text: String,
}

impl<'a> RawEvent<'a> {
    /// Creates a new RawEvent if the link has valid href and text elements
    pub fn new(link: ElementRef<'a>) -> Option<Self> {
        // Extract href and text
        let href = link.value().attr("href")?.to_string();
        let text = link.text().next()?.to_string();
        
        // Validate link is of the correct type
        if !href.ends_with(".htm") {
            return None;
        }
        
        Some(RawEvent { 
            link, 
            href, 
            text, 
        })
    }
}

pub struct Event {
    name: String,
    number: u32,
    prelims_link: Option<String>,
    finals_link: Option<String>,
}

impl Event {
    pub fn new(name: String, number: u32, prelims_link: Option<String>, finals_link: Option<String>) -> Event {
        Event {name, number, prelims_link, finals_link}
    }
    
    /// Creates an Event from a RawEvent, validating event type and number
    pub fn from_raw_event(raw_event: &RawEvent, base_url: &str) -> Option<Self> {
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
        
        // Set appropriate link based on event type
        let (prelims_link, finals_link) = if event_type == 'P' { //should still work but double check
            (Some(full_url), None)
        } else {
            (None, Some(full_url))
        };

        Some(Event {
            name: event_name.to_string(),
            number: event_num,
            prelims_link,
            finals_link,
        })
    }
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Event {{ name: {}, number: {} }}", 
            self.name, 
            self.number,
        )
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
        // Create a RawEvent if the link has valid href and text
        if let Some(raw_event) = RawEvent::new(link) {
            // Create an Event from the RawEvent, validating event type and number
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


