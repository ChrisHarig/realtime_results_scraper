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

pub struct Meet { //give meet the base url that it can pass down to each event to create the full url?
    events: HashMap<String, Event>,
    base_url: String,
    //add Date? Name? Location? 
}

impl Meet {
    // Methods we need: check for event given an event name (IMPLEMENTED)
    //add event to meet
    pub fn new(events: HashMap<String, Event>, base_url: String) -> Meet {
        Meet {events, base_url}
    }
    
    /// Gets a mutable reference to an event by its name
    pub fn get_event_by_name_mut(&mut self, name: &str) -> Option<&mut Event> {
        self.events.get_mut(name)
    }

    pub fn print_events(&mut self) {
        let event_names: Vec<_> = self.events.keys().collect();
        
        for event_name in event_names {
            if let Some(event) = self.events.get(event_name) {
                println!("Event {}: {}", event.number, event_name);
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

// Raw event has a session associated with it while event does not
pub struct RawEvent<'a> { 
    link: ElementRef<'a>,
    href: String,
    text: String,
    event_name: String,
    event_num: u32,
    session: char,
}

impl<'a> RawEvent<'a> {
    /// Creates a new RawEvent if the link has valid href and text
    pub fn new(link: ElementRef<'a>) -> Option<Self> {
        // Extract href and text
        let href = link.value().attr("href")?.to_string();
        let text = link.text().next()?.to_string();
        
        // Validate link is of the correct type
        if !href.ends_with(".htm") {
            return None;
        }

        let event_code = href.trim_end_matches(".htm");
        
        let last_four = &event_code[event_code.len().saturating_sub(4)..];
        let session = last_four.chars().next()?;

        // Check if event is a valid prelims or finals event, ignore if not
        if !(session == 'P' || session == 'F') || !last_four[1..].chars().all(|c| c.is_ascii_digit()) {
            return None;
        }

        let event_num = match last_four[1..].parse::<u32>() {
            Ok(num) => num,
            Err(_) => return None,
        };

        // Parse the event name, removing "Prelims" or "Finals" if present
        let event_name = text
            .split_once(' ')
            .map(|(_, rest)| rest.trim())
            .unwrap_or(&text)
            .replace(" Prelims", "")
            .replace(" Finals", "");
        
        Some(RawEvent { 
            link, 
            href, 
            text, 
            event_name,
            event_num,
            session,
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

        // Use the event_name, session and code we extracted in RawEvent
        let name = &raw_event.event_name;
        let session = &raw_event.session;
        let number = &raw_event.event_num;

        // Create full URL by combining base_url with href
        let full_url = format!("{}/{}", base_url, &raw_event.href);
        
        // Set appropriate link based on event type
        let (prelims_link, finals_link) = if session == &'P' { 
            (Some(full_url), None)
        } else {
            (None, Some(full_url))
        };

        Some(Event {
            name: name.to_string(),
            number: *number, //dont understand why dereference this
            prelims_link,
            finals_link,
        })
    }

    /// Adds a link to an existing event, I think we can delete has_link
    pub fn add_link(&mut self, link: String, session: char) {
        match session {
            'P' => {
                if self.prelims_link.is_none() {
                    self.prelims_link = Some(link);
                } else {
                    eprintln!("WARNING: Attempting to add duplicate prelims link to event {}", self.name);
                }
            },
            'F' => {
                if self.finals_link.is_none() {
                    self.finals_link = Some(link);
                } else {
                    eprintln!("WARNING: Attempting to add duplicate finals link to event {}", self.name);
                }
            },
            _ => eprintln!("WARNING: Invalid session '{}' when adding link", session),
        }
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

async fn process_links(base_url: &str) -> Result<Meet, Box<dyn std::error::Error>> {
    // Create a new meet with an empty HashMap
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
            // Create full URL by combining base_url with href
            let full_url = format!("{}/{}", base_url, &raw_event.href);
            
            // Get event with corresponding name if it exists
            if let Some(event) = meet.get_event_by_name_mut(&raw_event.event_name) {
                // Add the link to the existing event
                event.add_link(full_url, raw_event.session);
            } else {
                // Create a new event if it does not already exist
                if let Some(event) = Event::from_raw_event(&raw_event, base_url) {
                    // Add the event to the meet using the event name as the key
                    meet.events.insert(raw_event.event_name.clone(), event);
                }
            }
        }
    }
    
    Ok(meet)
}

async fn process_meet_pages(meet: Meet) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nProcessing individual event pages...");
    for event_num in meet.events.keys() {
        if let Some(event) = meet.events.get(event_num) {
            // Process finals if available, otherwise prelims
            if let Some(link) = &event.finals_link {
                match process_event_page(&event.name, link, 'F').await {
                    Ok(results) => print_results(&results),
                    Err(e) => {
                        if e.to_string().contains("Relay events are not currently supported") {
                            println!("Warning: Skipping relay event '{}' - {}", event.name, e);
                        } else {
                            // For other errors, propagate them up
                            return Err(e);
                        }
                    }
                }
            } else if let Some(link) = &event.prelims_link {
                match process_event_page(&event.name, link, 'P').await {
                    Ok(results) => print_results(&results),
                    Err(e) => {
                        if e.to_string().contains("Relay events are not currently supported") {
                            println!("Warning: Skipping relay event '{}' - {}", event.name, e);
                        } else {
                            // For other errors, propagate them up
                            return Err(e);
                        }
                    }
                }
            }
            
            // Add a small delay between requests to be nice to the server
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }
    Ok(())
}

//MAKE-IMPLEMENTED
// Print out each event and each of it's endpoints (Prelims and Finals)

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Welcome to the Swim Meet Results Parser!");
    println!("Please select an option:");
    println!("1. Parse a particular event's page");
    println!("   Example: https://swimmeetresults.tech/NCAA-Division-I-Men-2024/evtF001.htm");
    println!("2. Parse a complete meet");
    println!("   Example: https://swimmeetresults.tech/NCAA-Division-I-Men-2024");
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    match input.trim() {
        "1" => {
            // Parse an individual event page
            println!("---Individual page parsing selected---");
            println!("Enter the full URL to the event page:");
            println!("Note: URL should end with .htm and include the event code (e.g., evtF001)");
            let mut page_url = String::new();
            std::io::stdin().read_line(&mut page_url)?;

            // Create a RawEvent if the link has valid href and text (((MAY NEED TO REMOVE FIRST PART OF LINK)))
            if let Some(raw_event) = RawEvent::new(link) {

                // Create full URL by combining base_url with href
                let full_url = format!("{}/{}", base_url, &raw_event.href);
            
                // Create an event from the raw event
                if let Some(event) = Event::from_raw_event(&raw_event, base_url) {
                    match process_event_page(&event.name.trim(), &page_url.trim(), session).await { //debug i assume
                        Ok(results) => print_results(&results),
                        Err(e) => {
                            if e.to_string().contains("Relay events are not currently supported") {
                                println!("Warning: This is a relay event which is not currently supported.");
                            } else {
                                eprintln!("Error processing event: {}", e);
                                return Err(e);
                            }
                        }
                    }
                }
            }
        }
        },
        "2" => {
            // Parse a whole meet
            // You'll implement the individual page parsing here
            println!("---Meet parsing selected---");
            println!("Enter the base URL for the meet:");
            println!("Note: This should be the base URL without any specific event page");
            let mut page_url = String::new();
            std::io::stdin().read_line(&mut page_url)?;

            let mut meet = process_links(page_url).await?;
            
            meet.print_events();
            
            process_meet_pages(meet).await?;
        },
        _ => {
            println!("Invalid option. Please run the program again and select option 1 or 2.");
        }
    }
    
    Ok(())
}


