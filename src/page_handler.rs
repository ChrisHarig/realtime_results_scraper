use scraper::{Html, Selector};
use std::error::Error;

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
    seed_time: Option<String>, //prelims time if the event is finals
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
    
    // First check for reaction time in the first line after the main line
    if lines.len() > 1 {
        let first_line = lines[1].trim();
        if first_line.starts_with("r:") {
            // Extract reaction time
            let reaction_parts: Vec<&str> = first_line.split_whitespace().collect();
            if !reaction_parts.is_empty() {
                splits.push(Split {
                    distance: 0,
                    time: reaction_parts[0].to_string(),
                });
                
                // Extract first split if it's on the same line as reaction time
                if reaction_parts.len() > 1 {
                    splits.push(Split {
                        distance: 50,
                        time: reaction_parts[1].to_string(),
                    });
                }
            }
        }
    }
    
    // Process all lines for additional splits
    for line_idx in 1..lines.len() {
        let line = lines[line_idx].trim();
        if line.is_empty() {
            continue;
        }

        // Extract all splits from the line
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        for (i, part) in parts.iter().enumerate() {
            // Skip reaction time which we already processed
            if i == 0 && line_idx == 1 && part.starts_with("r:") {
                continue;
            }
            
            // Check if this part contains a time with parentheses
            if part.contains('(') && part.contains(')') {
                let time_start = part.find('(').unwrap() + 1;
                let time_end = part.find(')').unwrap();
                if time_start < time_end {
                    let split_time = &part[time_start..time_end];
                    
                    // Calculate the distance based on the number of existing splits
                    // Skip the reaction time when calculating distance
                    let distance = if splits.is_empty() {
                        50
                    } else if splits[0].distance == 0 {
                        // If we have a reaction time, adjust accordingly
                        (splits.len() as u16) * 50
                    } else {
                        (splits.len() as u16 + 1) * 50
                    };
                    
                    splits.push(Split {
                        distance,
                        time: split_time.to_string(),
                    });
                }
            }
            // Check if this is a standalone time (not in parentheses)
            else if !part.starts_with("(") && !part.ends_with(")") && 
                    part.contains(':') || (part.contains('.') && part.chars().filter(|&c| c == '.').count() == 1) {
                // This looks like a time without parentheses (e.g., "21.26" or "1:08.01")
                // Only add if we don't already have a split at this position
                let distance = if splits.is_empty() {
                    50
                } else if splits[0].distance == 0 {
                    (splits.len() as u16) * 50
                } else {
                    (splits.len() as u16 + 1) * 50
                };
                
                splits.push(Split {
                    distance,
                    time: part.to_string(),
                });
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