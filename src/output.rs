use crate::event_handler::EventResults;
use crate::relay_handler::RelayResults;
use crate::utils::{generate_unique_id, sanitize_name};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::path::PathBuf;

const CSV_OUTPUT_FILE: &str = "results.csv";
const RELAY_CSV_OUTPUT_FILE: &str = "relay_results.csv";
const METADATA_CSV_OUTPUT_FILE: &str = "metadata.csv";

// ============================================================================
// METADATA CSV OUTPUT
// ============================================================================

/// Writes event metadata to metadata.csv
pub fn write_metadata_csv(
    individual_results: &[EventResults],
    relay_results: &[RelayResults],
) -> Result<(), Box<dyn Error>> {
    let file = File::create(METADATA_CSV_OUTPUT_FILE)?;
    let mut writer = csv::Writer::from_writer(file);

    writer.write_record(["event_name", "session", "venue", "meet_name", "records"])?;

    for event in individual_results {
        let session = if event.session == 'P' { "Prelims" } else { "Finals" };
        let (venue, meet_name, records) = if let Some(ref meta) = event.metadata {
            (
                meta.venue.clone().unwrap_or_default(),
                meta.meet_name.clone().unwrap_or_default(),
                meta.records.iter()
                    .map(|r| r.trim_matches('=').trim())
                    .collect::<Vec<_>>()
                    .join(" | "),
            )
        } else {
            (String::new(), String::new(), String::new())
        };

        writer.write_record([
            &event.event_name,
            session,
            &venue,
            &meet_name,
            &records,
        ])?;
    }

    for event in relay_results {
        let session = if event.session == 'P' { "Prelims" } else { "Finals" };
        let (venue, meet_name, records) = if let Some(ref meta) = event.metadata {
            (
                meta.venue.clone().unwrap_or_default(),
                meta.meet_name.clone().unwrap_or_default(),
                meta.records.iter()
                    .map(|r| r.trim_matches('=').trim())
                    .collect::<Vec<_>>()
                    .join(" | "),
            )
        } else {
            (String::new(), String::new(), String::new())
        };

        writer.write_record([
            &event.event_name,
            session,
            &venue,
            &meet_name,
            &records,
        ])?;
    }

    writer.flush()?;
    println!("Metadata written to {}", METADATA_CSV_OUTPUT_FILE);
    Ok(())
}

// ============================================================================
// INDIVIDUAL CSV OUTPUT
// ============================================================================

/// Writes individual event results to results.csv
pub fn write_individual_csv(results: &[EventResults], options: &OutputOptions) -> Result<(), Box<dyn Error>> {
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
            // Filter by placement if top_n is set (skip DQ/no-place swimmers)
            if let Some(top_n) = options.top_n {
                match swimmer.place {
                    Some(place) if u32::from(place) > top_n => continue,
                    None => continue,
                    _ => {}
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
pub fn print_individual_results(results: &EventResults, options: &OutputOptions) {
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
        // Filter by placement if top_n is set (skip DQ/no-place swimmers)
        if let Some(top_n) = options.top_n {
            match swimmer.place {
                Some(place) if u32::from(place) > top_n => continue,
                None => continue,
                _ => {}
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
            for (i, split) in swimmer.splits.iter().enumerate() {
                print!(" split{}={}", i + 1, split.time);
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
            // Filter by placement if top_n is set (skip DQ/no-place teams)
            if let Some(top_n) = options.top_n {
                match team.place {
                    Some(place) if u32::from(place) > top_n => continue,
                    None => continue,
                    _ => {}
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
        // Filter by placement if top_n is set (skip DQ/no-place teams)
        if let Some(top_n) = options.top_n {
            match team.place {
                Some(place) if u32::from(place) > top_n => continue,
                None => continue,
                _ => {}
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
            for (i, split) in team.splits.iter().enumerate() {
                print!(" split{}={}", i + 1, split.time);
            }
            println!();
        }
    }
}

// ============================================================================
// FOLDER-BASED CSV OUTPUT
// ============================================================================

/// Writes results to organized folder structure
/// Creates: MeetName_datetime_random/EventName_datetime_random/files.csv
pub fn write_results_to_folders(
    individual_results: &[EventResults],
    relay_results: &[RelayResults],
    meet_title: Option<&str>,
    options: &OutputOptions,
) -> Result<PathBuf, Box<dyn Error>> {
    let meet_id = generate_unique_id();

    // Create meet folder name
    let meet_name = meet_title
        .map(|t| sanitize_name(t))
        .unwrap_or_else(|| "UnknownMeet".to_string());
    let meet_folder_name = format!("{}_{}", meet_name, meet_id);
    let meet_path = PathBuf::from(&meet_folder_name);

    fs::create_dir_all(&meet_path)?;
    println!("Created meet folder: {}", meet_folder_name);

    // Group results by event name (combining individual and relay)
    let mut event_groups: HashMap<String, (Vec<&EventResults>, Vec<&RelayResults>)> = HashMap::new();

    for result in individual_results {
        let event_name = &result.event_name;
        event_groups
            .entry(event_name.clone())
            .or_insert_with(|| (Vec::new(), Vec::new()))
            .0
            .push(result);
    }

    for result in relay_results {
        let event_name = &result.event_name;
        event_groups
            .entry(event_name.clone())
            .or_insert_with(|| (Vec::new(), Vec::new()))
            .1
            .push(result);
    }

    // Process each event
    for (event_name, (ind_results, rel_results)) in &event_groups {
        let event_id = generate_unique_id();
        let sanitized_event = sanitize_name(event_name);
        let event_folder_name = format!("{}_{}", sanitized_event, event_id);
        let event_path = meet_path.join(&event_folder_name);

        fs::create_dir_all(&event_path)?;

        let file_suffix = format!("{}_{}", sanitized_event, event_id);

        // Write individual results if present
        if !ind_results.is_empty() {
            let ind_file = event_path.join(format!("results_{}.csv", file_suffix));
            write_individual_csv_to_file(ind_results, options, &ind_file)?;
        }

        // Write relay results if present
        if !rel_results.is_empty() {
            let relay_file = event_path.join(format!("results_{}.csv", file_suffix));
            write_relay_csv_to_file(rel_results, options, &relay_file)?;
        }

        // Write metadata if enabled
        if options.metadata {
            let meta_file = event_path.join(format!("metadata_{}.csv", file_suffix));
            write_metadata_csv_to_file(ind_results, rel_results, &meta_file)?;
        }

        println!("  Created event folder: {}", event_folder_name);
    }

    Ok(meet_path)
}

/// Writes individual results to a specific file path
fn write_individual_csv_to_file(
    results: &[&EventResults],
    options: &OutputOptions,
    path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    let max_splits = results.iter()
        .flat_map(|e| e.swimmers.iter())
        .map(|s| s.splits.len())
        .max()
        .unwrap_or(0);

    let file = File::create(path)?;
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
            // Filter by placement if top_n is set (skip DQ/no-place swimmers)
            if let Some(top_n) = options.top_n {
                match swimmer.place {
                    Some(place) if u32::from(place) > top_n => continue,
                    None => continue,
                    _ => {}
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
    Ok(())
}

/// Writes relay results to a specific file path
fn write_relay_csv_to_file(
    results: &[&RelayResults],
    options: &OutputOptions,
    path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    if results.is_empty() {
        return Ok(());
    }

    let max_splits = results.iter()
        .flat_map(|e| e.teams.iter())
        .map(|t| t.splits.len())
        .max()
        .unwrap_or(0);

    let file = File::create(path)?;
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
            // Filter by placement if top_n is set (skip DQ/no-place teams)
            if let Some(top_n) = options.top_n {
                match team.place {
                    Some(place) if u32::from(place) > top_n => continue,
                    None => continue,
                    _ => {}
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
    Ok(())
}

/// Writes metadata to a specific file path
fn write_metadata_csv_to_file(
    individual_results: &[&EventResults],
    relay_results: &[&RelayResults],
    path: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    let file = File::create(path)?;
    let mut writer = csv::Writer::from_writer(file);

    writer.write_record(["event_name", "session", "venue", "meet_name", "records"])?;

    for event in individual_results {
        let session = if event.session == 'P' { "Prelims" } else { "Finals" };
        let (venue, meet_name, records) = if let Some(ref meta) = event.metadata {
            (
                meta.venue.clone().unwrap_or_default(),
                meta.meet_name.clone().unwrap_or_default(),
                meta.records.iter()
                    .map(|r| r.trim_matches('=').trim())
                    .collect::<Vec<_>>()
                    .join(" | "),
            )
        } else {
            (String::new(), String::new(), String::new())
        };

        writer.write_record([
            &event.event_name,
            session,
            &venue,
            &meet_name,
            &records,
        ])?;
    }

    for event in relay_results {
        let session = if event.session == 'P' { "Prelims" } else { "Finals" };
        let (venue, meet_name, records) = if let Some(ref meta) = event.metadata {
            (
                meta.venue.clone().unwrap_or_default(),
                meta.meet_name.clone().unwrap_or_default(),
                meta.records.iter()
                    .map(|r| r.trim_matches('=').trim())
                    .collect::<Vec<_>>()
                    .join(" | "),
            )
        } else {
            (String::new(), String::new(), String::new())
        };

        writer.write_record([
            &event.event_name,
            session,
            &venue,
            &meet_name,
            &records,
        ])?;
    }

    writer.flush()?;
    Ok(())
}
