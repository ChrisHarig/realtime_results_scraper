use realtime_results_scraper::{
    parse, process_event, parse_meet_index, print_results, print_relay_results,
    write_csv, write_relay_csv, detect_url_type, validate_meet_url, validate_event_url,
    UrlType, ParsedEvent
};

const NCAA_D1_MEN_2024_URL: &str = "https://swimmeetresults.tech/NCAA-Division-I-Men-2024";
const EVENT_500_FREE_FINALS_URL: &str = "https://swimmeetresults.tech/NCAA-Division-I-Men-2024/240327F003.htm";
const RELAY_200_MEDLEY_URL: &str = "https://swimmeetresults.tech/NCAA-Division-I-Men-2024/240327F001.htm";

#[test]
fn test_url_detection() {
    // Meet URL
    assert_eq!(detect_url_type(NCAA_D1_MEN_2024_URL), UrlType::Meet);

    // Meet URL with trailing slash
    assert_eq!(detect_url_type(&format!("{}/", NCAA_D1_MEN_2024_URL)), UrlType::Meet);

    // Event URL
    assert_eq!(detect_url_type(EVENT_500_FREE_FINALS_URL), UrlType::Event);

    // Any .htm URL is detected as event (validation happens later)
    assert_eq!(detect_url_type("https://example.com/foo.htm"), UrlType::Event);
}

#[test]
fn test_validate_meet_url() {
    // Valid meet URLs
    assert!(validate_meet_url(NCAA_D1_MEN_2024_URL).is_ok());
    assert!(validate_meet_url(&format!("{}/", NCAA_D1_MEN_2024_URL)).is_ok());

    // Invalid: wrong domain
    let err = validate_meet_url("https://example.com/NCAA-Division-I-Men-2024").unwrap_err();
    assert!(err.to_string().contains("Invalid domain"));

    // Invalid: no meet name
    let err = validate_meet_url("https://swimmeetresults.tech/").unwrap_err();
    assert!(err.to_string().contains("Missing meet name"));

    // Invalid: event URL passed as meet
    let err = validate_meet_url(EVENT_500_FREE_FINALS_URL).unwrap_err();
    assert!(err.to_string().contains("should not end with .htm"));
}

#[test]
fn test_validate_event_url() {
    // Valid event URLs
    let session = validate_event_url(EVENT_500_FREE_FINALS_URL).unwrap();
    assert_eq!(session, 'F');

    let prelims_url = "https://swimmeetresults.tech/NCAA-Division-I-Men-2024/240327P003.htm";
    let session = validate_event_url(prelims_url).unwrap();
    assert_eq!(session, 'P');

    // Invalid: wrong domain
    let err = validate_event_url("https://example.com/Meet/240327F003.htm").unwrap_err();
    assert!(err.to_string().contains("Invalid domain"));

    // Invalid: missing .htm
    let err = validate_event_url("https://swimmeetresults.tech/Meet/240327F003").unwrap_err();
    assert!(err.to_string().contains("must end with .htm"));

    // Invalid: bad session type
    let err = validate_event_url("https://swimmeetresults.tech/Meet/240327X003.htm").unwrap_err();
    assert!(err.to_string().contains("Invalid session type"));

    // Invalid: filename too short
    let err = validate_event_url("https://swimmeetresults.tech/Meet/F003.htm").unwrap_err();
    assert!(err.to_string().contains("Invalid event filename"));
}

#[tokio::test]
async fn test_process_individual_event() {
    println!("\n========================================");
    println!("Testing: process_event (500 Free Finals)");
    println!("URL: {}", EVENT_500_FREE_FINALS_URL);
    println!("========================================\n");

    let session = validate_event_url(EVENT_500_FREE_FINALS_URL).unwrap();
    let result = process_event(EVENT_500_FREE_FINALS_URL, session).await;

    match result {
        Ok(ParsedEvent::Individual(event_results)) => {
            print_results(&event_results);
            println!("\n✓ Successfully parsed event with {} swimmers", event_results.swimmers.len());
            assert!(!event_results.swimmers.is_empty(), "Should have parsed swimmers");
        }
        Ok(ParsedEvent::Relay(_)) => {
            panic!("Expected individual event, got relay");
        }
        Err(e) => {
            panic!("Failed to process event: {}", e);
        }
    }
}

#[tokio::test]
async fn test_process_relay_event() {
    println!("\n========================================");
    println!("Testing: process_event (200 Medley Relay)");
    println!("URL: {}", RELAY_200_MEDLEY_URL);
    println!("========================================\n");

    let session = validate_event_url(RELAY_200_MEDLEY_URL).unwrap();
    let result = process_event(RELAY_200_MEDLEY_URL, session).await;

    match result {
        Ok(ParsedEvent::Relay(relay_results)) => {
            print_relay_results(&relay_results);
            println!("\n✓ Successfully parsed relay with {} teams", relay_results.teams.len());
            assert!(!relay_results.teams.is_empty(), "Should have parsed teams");
        }
        Ok(ParsedEvent::Individual(_)) => {
            panic!("Expected relay event, got individual");
        }
        Err(e) => {
            panic!("Failed to process relay: {}", e);
        }
    }
}

#[tokio::test]
async fn test_parse_meet_index() {
    println!("\n========================================");
    println!("Testing: parse_meet_index (NCAA D1 Men 2024)");
    println!("URL: {}", NCAA_D1_MEN_2024_URL);
    println!("========================================\n");

    let meet = parse_meet_index(NCAA_D1_MEN_2024_URL).await
        .expect("Failed to parse meet index");

    println!("Found {} events in the meet:\n", meet.events.len());
    meet.print_events();

    assert!(!meet.events.is_empty(), "Should have found events");
    println!("\n✓ Successfully parsed meet index with {} events", meet.events.len());
}

#[tokio::test]
async fn test_parse_event_url() {
    println!("\n========================================");
    println!("Testing: parse() with event URL");
    println!("========================================\n");

    let (individual, relay) = parse(EVENT_500_FREE_FINALS_URL).await
        .expect("Failed to parse event");

    assert_eq!(individual.len(), 1, "Should return exactly one individual event");
    assert!(relay.is_empty(), "Should return no relay events");
    print_results(&individual[0]);
    println!("\n✓ parse correctly handled individual event URL");
}

#[tokio::test]
async fn test_parse_relay_url() {
    println!("\n========================================");
    println!("Testing: parse() with relay URL");
    println!("========================================\n");

    let (individual, relay) = parse(RELAY_200_MEDLEY_URL).await
        .expect("Failed to parse relay");

    assert!(individual.is_empty(), "Should return no individual events");
    assert_eq!(relay.len(), 1, "Should return exactly one relay event");
    print_relay_results(&relay[0]);
    println!("\n✓ parse correctly handled relay event URL");
}

#[tokio::test]
async fn test_parse_meet_url() {
    println!("\n========================================");
    println!("Testing: parse() with meet URL");
    println!("========================================\n");

    let (individual, relay) = parse(NCAA_D1_MEN_2024_URL).await
        .expect("Failed to parse meet");

    println!("Parsed {} individual events, {} relay events", individual.len(), relay.len());

    assert!(!individual.is_empty(), "Should have parsed individual events");
    assert!(!relay.is_empty(), "Should have parsed relay events");
    println!("\n✓ parse correctly handled meet URL");
}

#[tokio::test]
async fn test_write_csv() {
    println!("\n========================================");
    println!("Testing: write_csv");
    println!("========================================\n");

    let (individual, relay) = parse(EVENT_500_FREE_FINALS_URL).await
        .expect("Failed to parse event");

    write_csv(&individual).expect("Failed to write CSV");

    // Verify file exists
    assert!(std::path::Path::new("results.csv").exists(), "CSV file should exist");
    println!("\n✓ CSV written successfully");

    // Clean up relay CSV test
    if !relay.is_empty() {
        write_relay_csv(&relay).expect("Failed to write relay CSV");
        assert!(std::path::Path::new("relay_results.csv").exists(), "Relay CSV file should exist");
    }
}
