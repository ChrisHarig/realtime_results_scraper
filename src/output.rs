use crate::event_handler::EventResults;
use crate::relay_handler::RelayResults;
use std::error::Error;
use std::fs::File;

const CSV_OUTPUT_FILE: &str = "results.csv";
const RELAY_CSV_OUTPUT_FILE: &str = "relay_results.csv";

// ============================================================================
// INDIVIDUAL CSV OUTPUT
// ============================================================================

/// Writes individual event results to results.csv
pub fn write_csv(results: &[EventResults], options: &OutputOptions) -> Result<(), Box<dyn Error>> {
    let max_splits = results.iter()
        .flat_map(|e| e.swimmers.iter())
        .map(|s| s.splits.len())
        .max()
        .unwrap_or(0);

    let file = File::create(CSV_OUTPUT_FILE)?;
    let mut writer = csv::Writer::from_writer(file);

    let mut header: Vec<&str> = vec![
        "event_name", "session", "event_number", "gender", "distance",
        "course", "stroke", "place", "name", "year", "school", "seed_time", "final_time", "reaction_time"
    ];

    let split_headers: Vec<String> = (1..=max_splits).map(|i| format!("split{}", i)).collect();
    let split_header_refs: Vec<&str> = split_headers.iter().map(|s| s.as_str()).collect();
    header.extend(split_header_refs);

    writer.write_record(&header)?;

    for event in results {
        let session = if event.session == 'P' { "Prelims" } else { "Finals" };

        let (event_number, gender, distance, course, stroke) = if let Some(ref info) = event.race_info {
            (
                info.event_number,
                info.gender.clone().unwrap_or_default(),
                info.distance.unwrap_or(0),
                info.course.clone().unwrap_or_default(),
                info.stroke.clone().unwrap_or_default(),
            )
        } else {
            (0, String::new(), 0, String::new(), String::new())
        };

        for swimmer in &event.swimmers {
            // Filter by placement if top_n is set
            if let Some(top_n) = options.top_n {
                if let Some(place) = swimmer.place {
                    if u32::from(place) > top_n {
                        continue;
                    }
                }
            }

            let place_str = match swimmer.place {
                Some(p) => p.to_string(),
                None => String::new(),
            };
            let mut row: Vec<String> = vec![
                event.event_name.clone(),
                session.to_string(),
                event_number.to_string(),
                gender.clone(),
                distance.to_string(),
                course.clone(),
                stroke.clone(),
                place_str,
                swimmer.name.clone(),
                swimmer.year.clone(),
                swimmer.school.clone(),
                swimmer.seed_time.clone().unwrap_or_default(),
                swimmer.final_time.clone(),
                swimmer.reaction_time.clone().unwrap_or_default(),
            ];

            for i in 0..max_splits {
                if i < swimmer.splits.len() {
                    row.push(swimmer.splits[i].time.clone());
                } else {
                    row.push(String::new());
                }
            }

            writer.write_record(&row)?;
        }
    }

    writer.flush()?;
    println!("Results written to {}", CSV_OUTPUT_FILE);
    Ok(())
}

// ============================================================================
// OUTPUT FORMATTING
// ============================================================================

/// Configuration for output display and filtering
#[derive(Debug, Clone)]
pub struct OutputOptions {
    pub metadata: bool,
    /// Maximum placement to include (None = all placements)
    pub top_n: Option<u32>,
}

impl Default for OutputOptions {
    fn default() -> Self {
        OutputOptions {
            metadata: true,
            top_n: None,
        }
    }
}

/// Prints individual results to stdout
pub fn print_results(results: &EventResults, options: &OutputOptions) {
    let session_str = if results.session == 'P' { "Prelims" } else { "Finals" };

    if options.metadata {
        if let Some(ref meta) = results.metadata {
            if let Some(ref venue) = meta.venue {
                println!("Venue: {}", venue);
            }
            if let Some(ref meet) = meta.meet_name {
                println!("Meet: {}", meet);
            }
            if !meta.records.is_empty() {
                println!("Records:");
                for record in &meta.records {
                    println!("  {}", record);
                }
            }
        }

        if let Some(ref info) = results.race_info {
            let gender = info.gender.as_deref().unwrap_or("?");
            let distance = info.distance.map(|d| d.to_string()).unwrap_or_else(|| "?".to_string());
            let stroke = info.stroke.as_deref().unwrap_or("?");
            let course = info.course.as_deref().unwrap_or("");
            let relay = if info.is_relay { "(Relay)" } else { "" };

            println!("Race: {} {} {} {} {}", gender, distance, course, stroke, relay);
        }
    }

    println!("\nEvent: {} {}", results.event_name, session_str);
    println!("{:-<80}", "");

    for swimmer in &results.swimmers {
        // Filter by placement if top_n is set
        if let Some(top_n) = options.top_n {
            if let Some(place) = swimmer.place {
                if u32::from(place) > top_n {
                    continue;
                }
            }
        }

        let place_str = match swimmer.place {
            Some(p) => format!("{:2}", p),
            None => "--".to_string(),
        };
        println!(
            "{}. {:25} {:2} {:20} {}",
            place_str,
            swimmer.name,
            swimmer.year,
            swimmer.school,
            swimmer.final_time
        );

        if !swimmer.splits.is_empty() {
            print!("    Splits:");
            for split in &swimmer.splits {
                print!(" {}={}", split.distance, split.time);
            }
            println!();
        }
    }
}

// ============================================================================
// RELAY CSV OUTPUT
// ============================================================================

/// Writes relay results to relay_results.csv
pub fn write_relay_csv(results: &[RelayResults], options: &OutputOptions) -> Result<(), Box<dyn Error>> {
    if results.is_empty() {
        return Ok(());
    }

    let max_splits = results.iter()
        .flat_map(|e| e.teams.iter())
        .map(|t| t.splits.len())
        .max()
        .unwrap_or(0);

    let file = File::create(RELAY_CSV_OUTPUT_FILE)?;
    let mut writer = csv::Writer::from_writer(file);

    let mut header: Vec<&str> = vec![
        "event_name", "session", "event_number", "gender", "distance", "course", "stroke",
        "place", "team_name", "seed_time", "final_time", "dq_description",
        "swimmer1_name", "swimmer1_year", "swimmer2_name", "swimmer2_year",
        "swimmer3_name", "swimmer3_year", "swimmer4_name", "swimmer4_year",
        "swimmer1_reaction", "swimmer2_reaction", "swimmer3_reaction", "swimmer4_reaction"
    ];

    let split_headers: Vec<String> = (1..=max_splits).map(|i| format!("split{}", i)).collect();
    let split_header_refs: Vec<&str> = split_headers.iter().map(|s| s.as_str()).collect();
    header.extend(split_header_refs);

    writer.write_record(&header)?;

    for event in results {
        let session = if event.session == 'P' { "Prelims" } else { "Finals" };

        let (event_number, gender, distance, course, stroke) = if let Some(ref info) = event.race_info {
            (
                info.event_number,
                info.gender.clone().unwrap_or_default(),
                info.distance.unwrap_or(0),
                info.course.clone().unwrap_or_default(),
                info.stroke.clone().unwrap_or_default(),
            )
        } else {
            (0, String::new(), 0, String::new(), String::new())
        };

        for team in &event.teams {
            // Filter by placement if top_n is set
            if let Some(top_n) = options.top_n {
                if let Some(place) = team.place {
                    if u32::from(place) > top_n {
                        continue;
                    }
                }
            }

            let place_str = match team.place {
                Some(p) => p.to_string(),
                None => String::new(),
            };
            let mut row: Vec<String> = vec![
                event.event_name.clone(),
                session.to_string(),
                event_number.to_string(),
                gender.clone(),
                distance.to_string(),
                course.clone(),
                stroke.clone(),
                place_str,
                team.team_name.clone(),
                team.seed_time.clone().unwrap_or_default(),
                team.final_time.clone(),
                team.dq_description.clone().unwrap_or_default(),
            ];

            for i in 0..4 {
                if i < team.swimmers.len() {
                    row.push(team.swimmers[i].name.clone());
                    row.push(team.swimmers[i].year.clone());
                } else {
                    row.push(String::new());
                    row.push(String::new());
                }
            }

            for i in 0..4 {
                if i < team.swimmers.len() {
                    row.push(team.swimmers[i].reaction_time.clone().unwrap_or_default());
                } else {
                    row.push(String::new());
                }
            }

            for i in 0..max_splits {
                if i < team.splits.len() {
                    row.push(team.splits[i].time.clone());
                } else {
                    row.push(String::new());
                }
            }

            writer.write_record(&row)?;
        }
    }

    writer.flush()?;
    println!("Relay results written to {}", RELAY_CSV_OUTPUT_FILE);
    Ok(())
}

// ============================================================================
// RELAY OUTPUT FORMATTING
// ============================================================================

/// Prints relay results to stdout
pub fn print_relay_results(results: &RelayResults, options: &OutputOptions) {
    let session_str = if results.session == 'P' { "Prelims" } else { "Finals" };

    if options.metadata {
        if let Some(ref meta) = results.metadata {
            if let Some(ref venue) = meta.venue {
                println!("Venue: {}", venue);
            }
            if let Some(ref meet) = meta.meet_name {
                println!("Meet: {}", meet);
            }
            if !meta.records.is_empty() {
                println!("Records:");
                for record in &meta.records {
                    println!("  {}", record);
                }
            }
        }

        if let Some(ref info) = results.race_info {
            let gender = info.gender.as_deref().unwrap_or("?");
            let distance = info.distance.map(|d| d.to_string()).unwrap_or_else(|| "?".to_string());
            let stroke = info.stroke.as_deref().unwrap_or("?");
            let course = info.course.as_deref().unwrap_or("");

            println!("Race: {} {} {} {} Relay", gender, distance, course, stroke);
        }
    }

    println!("\nEvent: {} {}", results.event_name, session_str);
    println!("{:-<80}", "");

    for team in &results.teams {
        // Filter by placement if top_n is set
        if let Some(top_n) = options.top_n {
            if let Some(place) = team.place {
                if u32::from(place) > top_n {
                    continue;
                }
            }
        }

        let place_str = match team.place {
            Some(p) => format!("{:2}", p),
            None => "--".to_string(),
        };
        println!(
            "{}. {:25} {}",
            place_str,
            team.team_name,
            team.final_time
        );

        if let Some(ref desc) = team.dq_description {
            println!("    {}", desc);
        }

        for (i, swimmer) in team.swimmers.iter().enumerate() {
            let reaction = swimmer.reaction_time.as_deref().unwrap_or("");
            println!(
                "    {}) {:25} {:2} {}",
                i + 1,
                swimmer.name,
                swimmer.year,
                reaction
            );
        }

        if !team.splits.is_empty() {
            print!("    Splits:");
            for split in &team.splits {
                print!(" {}={}", split.distance, split.time);
            }
            println!();
        }
    }
}
