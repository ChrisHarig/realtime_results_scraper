# Realtime Results Scraper

A Rust CLI tool to parse swimming meet results from [swimmeetresults.tech](https://swimmeetresults.tech/).

Works with any realtime results page - pass a meet URL to scrape all events, or a specific event URL.

**Expected format:** Standard HyTek meet results pages where the index displays all events, and each link is a `.htm` file containing one event's results.

## Usage

```bash
# Parse entire meet
realtime_results_scraper <MEET_URL>

# Parse single event
realtime_results_scraper <EVENT_URL>

# Output to stdout instead of CSV
realtime_results_scraper -o stdout <URL>

# Only include top N placements
realtime_results_scraper -t 8 <URL>

# Disable metadata files
realtime_results_scraper --no-metadata <URL>
```

## Output

**Meet URL** creates:
```
MeetName_datetime_random/
├── EventName_datetime_random/
│   ├── results_EventName_datetime_random.csv
│   └── metadata_EventName_datetime_random.csv
...
```

**Event URL** creates:
```
MeetName_datetime_random/
└── EventName_datetime_random/
    ├── results_EventName_datetime_random.csv
    └── metadata_EventName_datetime_random.csv
```

Each folder/file includes a unique timestamp and random suffix to prevent overwrites.

## Building

```bash
cargo build --release
```
