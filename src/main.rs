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

// Represents a single event
struct SwimEvent {
    name: String,
    prelims_link: Option<String>,  // Some events might not have prelims
    finals_link: Option<String>,
}

// Fetches the HTML from the given URL, should be an index page
async fn fetch_html(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    Ok(response.text().await?)
}

fn parse_event_from_link(href: &str, event_text: &str) -> Option<(String, char, SwimEvent)> {
    if !href.ends_with(".htm") {
        return None;
    }

    let event_code = href.trim_end_matches(".htm");
    
    let event_type = event_code.chars().nth(event_code.len() - 4)?;
    let event_num = event_code[..event_code.len() - 4].chars().last()?;
    
    // Move on to Some() if the event exists
    Some((
        event_num.to_string(),
        event_type,
        SwimEvent {
            name: event_text.trim().to_string(),
            prelims_link: if event_type == 'P' { Some(href.to_string()) } else { None },
            finals_link: if event_type == 'F' { Some(href.to_string()) } else { None },
        }
    ))
}

fn process_event(events: &mut HashMap<String, SwimEvent>, event_key: String, event_type: char, event: SwimEvent) {
    events.entry(event_key)
        .and_modify(|e| {
            match event_type {
                'P' => e.prelims_link = event.prelims_link,
                'F' => e.finals_link = event.finals_link,
                _ => {}
            }
        })
        .or_insert(event);
}

fn print_events(events: &HashMap<String, SwimEvent>) {
    for (event_num, event) in events.iter() {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Base URL for the meet results
    let base_url = "https://swimmeetresults.tech/NCAA-Division-I-Men-2024/"; 
    
    // Get the index page
    let index_url = format!("{}/evtindex.htm", base_url);
    
    // Fetch and parse index page
    let index_html = fetch_html(&index_url).await?;
    let index_document = Html::parse_document(&index_html);
    
    // Select all links in the index
    let link_selector = Selector::parse("a").unwrap();
    
    // Map to store events: key is event number, value is SwimEvent
    let mut events: HashMap<String, SwimEvent> = HashMap::new();
    
    // Process each link
    for link in index_document.select(&link_selector) {
        if let Some(href) = link.value().attr("href") {
            if let Some(event_text) = link.text().next() {
                if let Some((event_key, event_type, event)) = parse_event_from_link(href, &event_text) {
                    process_event(&mut events, event_key, event_type, event);
                }
            }
        }
    }
    
    print_events(&events);
    
    // Process each event
    for (_, event) in events.iter() {
        if let Some(href) = &event.prelims_link {
            if let Some(page_html) = fetch_html(&format!("{}{}", base_url, href)).await? {
                let results = process_event_page(&page_html).await?;
                print_results(&results);
            }
        }
    }
    
    Ok(())
}


