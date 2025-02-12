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
    event_type: char,  // 'P' for prelims, 'F' for finals
    swimmers: Vec<Swimmer>,
}

// Adds the top 16 swimmers with all relevant information to the EventResults struct
pub async fn process_event_page(event_name: &str, page_url: &str, event_type: char) -> Result<EventResults, Box<dyn Error>> {
    let html = fetch_page(page_url).await?;
    let document = Html::parse_document(&html);
    
    let mut swimmers = Vec::new();
    
    // Select the pre tag containing results
    let pre_selector = Selector::parse("pre").unwrap();
    if let Some(pre) = document.select(&pre_selector).next() {
        let content = pre.text().collect::<String>();
        let lines: Vec<&str> = content.lines().collect();

        if !event_name.contains("Relay") {
            let mut i = 0;
            while i < lines.len() {
                let current_line = lines[i].trim();
                
                // Check if this is a main swimmer line (starts with place number)
                if let Some(first_char) = current_line.chars().next() {
                    if first_char.is_ascii_digit() {
                        // Find the next main line or end of content
                        let mut next_main_line_idx = i + 1;
                        while next_main_line_idx < lines.len() {
                            let next_line = lines[next_main_line_idx].trim();
                            if !next_line.is_empty() && next_line.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                                break;
                            }
                            next_main_line_idx += 1;
                        }

                        // Parse all lines between current and next main line
                        if let Some(swimmer) = parse_swimmer_section(&lines[i..next_main_line_idx]) {
                            swimmers.push(swimmer);
                            if swimmers.len() >= 16 {
                                break;
                            }
                        }
                        
                        i = next_main_line_idx;
                        continue;
                    }
                }
                i += 1;
            }
        } else {
            return Err("Relay events are not currently supported".into());
        }
    }
    
    Ok(EventResults {
        event_name: event_name.to_string(),
        event_type,
        swimmers,
    })
}

async fn fetch_page(url: &str) -> Result<String, Box<dyn Error>> {
    let response = reqwest::get(url).await?;
    Ok(response.text().await?)
}

fn parse_swimmer_section(lines: &[&str]) -> Option<Swimmer> {
    let main_line = lines[0].trim();
    
    // Parse main line which contains: place, name, year, school, seed time, final time, points
    let parts: Vec<&str> = main_line.split_whitespace().collect();
    
    let place: u8 = parts[0].parse().ok()?;
    
    // Find the year and school by looking backwards from the end
    let _points = parts.last()?.parse::<u8>().ok()?;
    let final_time = parts[parts.len()-3];
    let seed_time = Some(parts[parts.len()-4].to_string());
    
    // School might be multiple words, so work backwards from known positions
    let year = parts[parts.len()-5];
    
    // Name is everything between place and year
    let name_end = parts.len() - 5;
    let name = parts[1..name_end].join(" ");
    let school = parts[name_end-1].to_string();

    // Parse all splits from remaining lines
    let mut splits = Vec::new();
    
    // Process all lines after the main line for splits
    for line in &lines[1..] {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Extract all content within parentheses
        let mut in_parentheses = false;
        let mut current_split = String::new();
        
        for c in line.chars() { //might need to add back in reaction time 
            match c {
                '(' => {
                    in_parentheses = true;
                    current_split.clear();
                }
                ')' => {
                    if in_parentheses && !current_split.is_empty() {
                        let split_time = current_split.trim().to_string();
                        // Store split with temporary distance of 0, we'll calculate actual distances later
                        splits.push(Split {
                            distance: 0,
                            time: split_time,
                        });
                    }
                    in_parentheses = false;
                }
                _ => {
                    if in_parentheses {
                        current_split.push(c);
                    }
                }
            }
        }
    }

    // Calculate and assign proper distances to splits
    let total_splits = splits.len();
    if total_splits > 0 {
        // First split is usually reaction time
        if splits[0].time.starts_with("r:") {
            splits[0].distance = 0;
            
            // Remaining splits are usually at equal intervals
            let remaining_splits = total_splits - 1;
            if remaining_splits > 0 {
                for i in 1..total_splits {
                    splits[i].distance = (i as u16) * 50;
                }
            }
        } else {
            // No reaction time, all splits are distance splits
            for i in 0..total_splits {
                splits[i].distance = ((i + 1) as u16) * 50;
            }
        }
    }

    Some(Swimmer {
        place,
        name,
        year: year.to_string(),
        school,
        seed_time,
        final_time: final_time.to_string(),
        splits,
    })
}

pub fn print_results(results: &EventResults) {
    let event_type_str = if results.event_type == 'P' { "Prelims" } else { "Finals" };
    println!("\nEvent: {} {}", results.event_name, event_type_str);
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