use clap::{Parser, ValueEnum};
use realtime_results_scraper::{
    parse, print_individual_results, print_relay_results,
    write_individual_csv, write_relay_csv, write_metadata_csv, OutputOptions
};
use std::io::{self, BufRead};

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Csv,
    Stdout,
}

#[derive(Parser, Debug)]
#[command(name = "realtime_results_scraper")]
#[command(about = "Parse swimming meet results from URLs")]
struct Args {
    /// realtime-results meet or event URL to parse
    url: Option<String>,

    /// output format, csv or stdout
    #[arg(short, long, value_enum, default_value = "csv")]
    output: OutputFormat,

    /// disable metadata output (venue, meet name, records, race info)
    #[arg(long)]
    no_metadata: bool,

    /// maximum placement to include (e.g., 16 = top 16 places, ties included)
    /// 0 will output only meet metadata
    /// default to all participants
    #[arg(short, long)]
    top: Option<u32>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Get URL from args or stdin
    let url = match args.url {
        Some(url) => url,
        None => {
            println!("Enter meet or event URL:");
            let stdin = io::stdin();
            stdin.lock().lines().next()
                .ok_or("No input provided")??
        }
    };

    let url = url.trim();
    println!("Parsing: {}\n", url);

    // Enter parse flow
    let (individual_results, relay_results) = parse(url).await?;

    // Build options from args (None = all participants, Some(n) = top n placements)
    let options = OutputOptions {
        metadata: !args.no_metadata,
        top_n: args.top,
    };

    match args.output {
        OutputFormat::Csv => {
            if !individual_results.is_empty() {
                write_individual_csv(&individual_results, &options)?;
            }
            if !relay_results.is_empty() {
                write_relay_csv(&relay_results, &options)?;
            }
            if options.metadata {
                write_metadata_csv(&individual_results, &relay_results)?;
            }
        }
        OutputFormat::Stdout => {
            for event_results in &individual_results {
                print_individual_results(event_results, &options);
            }
            for relay_event in &relay_results {
                print_relay_results(relay_event, &options);
            }
        }
    }

    let total = individual_results.len() + relay_results.len();
    println!("\nParsed {} event(s) ({} individual, {} relay)",
             total, individual_results.len(), relay_results.len());

    Ok(())
}
