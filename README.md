# Realtime Results Scraper

A Rust CLI tool to parse swimming meet results from HY-TEK Realtime Results pages. 
(Ex: https://swimmeetresults.tech/NCAA-Division-I-Men-2025/)

Pass a meet URL to scrape all events, or a specific event URL. 

**Expected format:** Standard HyTek meet results pages where the index displays all events, and each link is a `.htm` file containing one event's results.

Some pages that contain formatting like US masters results, where each link contains results from multiple events, will not work with this package.

## Prerequisites

Requires Rust. Install via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Install

From crates.io:
```bash
cargo install realtime_results_scraper
```

Or build from source:
```bash
git clone https://github.com/ChrisHarig/realtime_results_scraper
cd realtime_results_scraper
cargo build --release
```

## Usage

To scrape a meet just copy the URL and paste in the command line. 

To grab a specific event's URL, go to the index on the left side of a meet page, ctrl+click and select "copy link address', then paste in to the command line.  

There are three optional flags detailed below.

```bash
# Parse entire meet
realtime_results_scraper <MEET_URL>

# Parse single event
realtime_results_scraper <EVENT_URL>

# Output to stdout instead of CSV
realtime_results_scraper -o stdout <URL>

# Only include top N placements
realtime_results_scraper -t 8 <URL>

# Disable metadata output
realtime_results_scraper --no-metadata <URL>

# Show help
realtime_results_scraper --help
```

## Output

The default ouput format is csv, but this can be changed to stdout. 

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
