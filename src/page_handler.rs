use scraper::{Html, Selector};
use std::error::Error;
use std::collections::HashMap;

// A race split
#[derive(Debug, Clone)]
pub struct Split {
    distance: u16,
    time: String,
}

// Each swimmer contains all the information about their performance, 
// and then is stored with other swimmers in an Event
#[derive(Debug, Clone)]
pub struct Swimmer {
    place: u8,
    name: String,
    year: String,
    school: String,
    seed_time: Option<String>,
    final_time: String,
    splits: Vec<Split>,
}

// Events store a list of swimmers
#[derive(Debug)]
pub struct EventResults {
    event_name: String,
    swimmers: Vec<Swimmer>,
}

pub async fn process_event_page(base_url: &str,event_name: &str,page_url: &str,) -> Result<EventResults, Box<dyn Error>> {
    let full_url = format!("{}/{}", base_url, page_url);
    let html = fetch_page(&full_url).await?;
    let document = Html::parse_document(&html);
    
    let mut swimmers = Vec::new();
    
    // Select the results table
    let table_selector = Selector::parse("table.event-table").unwrap_or(
        Selector::parse("table").unwrap()
    );
    
    let row_selector = Selector::parse("tr").unwrap();
    
    if let Some(table) = document.select(&table_selector).next() {
        for row in table.select(&row_selector).skip(1) { // Skip header row
            if let Some(swimmer) = parse_swimmer_row(row) {
                swimmers.push(swimmer);
            }
            
            if swimmers.len() >= 16 {
                break; // Only store top 16
            }
        }
    }
    
    Ok(EventResults {
        event_name: event_name.to_string(),
        swimmers,
    })
}

async fn fetch_page(url: &str) -> Result<String, Box<dyn Error>> {
    let response = reqwest::get(url).await?;
    Ok(response.text().await?)
}

fn parse_swimmer_row(row: scraper::element_ref::ElementRef) -> Option<Swimmer> {
    let cell_selector = Selector::parse("td").unwrap();
    let mut cells = row.select(&cell_selector);
    
    // Expected column order: Place, Name, Year, School, Seed Time, Final Time, Splits...
    let place = cells.next()?.text().next()?
        .trim().parse().ok()?;
    
    let name = cells.next()?.text().next()?
        .trim().to_string();
    
    let year = cells.next()?.text().next()?
        .trim().to_string();
    
    let school = cells.next()?.text().next()?
        .trim().to_string();
    
    let seed_time = cells.next()?.text().next()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    
    let final_time = cells.next()?.text().next()?
        .trim().to_string();
    
    // Parse splits from remaining cells
    let mut splits = Vec::new();
    let mut distance = 50;
    
    for cell in cells {
        if let Some(split_time) = cell.text().next() {
            let split_time = split_time.trim();
            if !split_time.is_empty() {
                splits.push(Split {
                    distance,
                    time: split_time.to_string(),
                });
                distance += 50;
            }
        }
    }
    
    Some(Swimmer {
        place,
        name,
        year,
        school,
        seed_time,
        final_time,
        splits,
    })
}

pub fn print_results(results: &EventResults) {
    println!("\nEvent: {}", results.event_name);
    println!("{:-<80}", "");
    
    for swimmer in &results.swimmers {
        println!(
            "{:2}. {:25} {:2} {:20} {}",
            swimmer.place,
            swimmer.name,
            swimmer.year,
            swimmer.school,
            swimmer.final_time
        );
        
        if !swimmer.splits.is_empty() {
            print!("   Splits:");
            for split in &swimmer.splits {
                print!(" {}={}", split.distance, split.time);
            }
            println!();
        }
    }
} 