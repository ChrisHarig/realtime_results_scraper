use std::io::{self, Write};
use scraper::{Html, Selector};
use reqwest;
use tokio;
use std::collections::HashMap;

// Represents a single swimming event
struct SwimEvent {
    name: String,
    prelims_link: Option<String>,  // Some events might not have prelims
    finals_link: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Base URL for the meet results
    let base_url = "https://swimmeetresults.tech/NCAA-Division-I-Men-2024/"; 
    
    // Get the index page
    let index_url = format!("{}/evtindex.htm", base_url);
    let index_response = reqwest::get(&index_url).await?;
    let index_html = index_response.text().await?;
    
    // Parse the index page
    let index_document = Html::parse_document(&index_html);
    
    // Select all links in the index
    let link_selector = Selector::parse("a").unwrap();
    
    // Map to store events: key is event number, value is SwimEvent
    let mut events: HashMap<String, SwimEvent> = HashMap::new();
    
    // Process each link
    for link in index_document.select(&link_selector) {
        if let Some(href) = link.value().attr("href") {
            if let Some(event_text) = link.text().next() {
                // Check if link ends with P001, F001 pattern
                if href.ends_with(".htm") {
                    let event_code = href.trim_end_matches(".htm");
                    
                    // Extract event number and type (P for prelims, F for finals)
                    if let (Some(event_type), Some(event_num)) = (
                        event_code.chars().nth(event_code.len() - 4),
                        event_code[..event_code.len() - 4].chars().last()
                    ) {
                        let event_key = event_num.to_string();
                        
                        // Create or update event in HashMap
                        events.entry(event_key.clone())
                            .and_modify(|e| {
                                match event_type {
                                    'P' => e.prelims_link = Some(href.to_string()),
                                    'F' => e.finals_link = Some(href.to_string()),
                                    _ => {}
                                }
                            })
                            .or_insert_with(|| SwimEvent {
                                name: event_text.trim().to_string(),
                                prelims_link: if event_type == 'P' { Some(href.to_string()) } else { None },
                                finals_link: if event_type == 'F' { Some(href.to_string()) } else { None },
                            });
                    }
                }
            }
        }
    }
    
    // Print organized events
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
    
    Ok(())
}


