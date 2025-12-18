use clap::{Parser, ValueEnum};
use realtime_results_scraper::{
    parse, print_results_with_options, print_relay_results_with_options,
    write_csv, write_relay_csv, OutputOptions
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
    /// Meet or event URL to parse
    url: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "csv")]
    output: OutputFormat,

    /// Disable metadata output (venue, meet name, records, race info)
    #[arg(long)]
    no_metadata: bool,
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

    match args.output {
        OutputFormat::Csv => {
            if !individual_results.is_empty() {
                write_csv(&individual_results)?;
            }
            if !relay_results.is_empty() {
                write_relay_csv(&relay_results)?;
            }
        }
        OutputFormat::Stdout => {
            let options = OutputOptions { metadata: !args.no_metadata };

            for event_results in &individual_results {
                print_results_with_options(event_results, &options);
            }
            for relay_event in &relay_results {
                print_relay_results_with_options(relay_event, &options);
            }
        }
    }

    let total = individual_results.len() + relay_results.len();
    println!("\nParsed {} event(s) ({} individual, {} relay)",
             total, individual_results.len(), relay_results.len());

    Ok(())
}
