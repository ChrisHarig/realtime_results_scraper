use clap::{Parser, ValueEnum};
use realtime_results_scraper::{
    parse, print_individual_results, print_relay_results,
    write_results_to_folders, OutputOptions
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
#[command(next_line_help = true)]
struct Args {
    /// Realtime-results meet or event URL to parse
    url: Option<String>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "csv")]
    output: OutputFormat,

    /// Disable metadata output
    #[arg(long, default_value = "false")]
    no_metadata: bool,

    /// Number of swimmers to include per event [default: all]
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
    let results = parse(url).await?;

    // Build options from args (None = all participants, Some(n) = top n placements)
    let options = OutputOptions {
        metadata: !args.no_metadata,
        top_n: args.top,
    };

    match args.output {
        OutputFormat::Csv => {
            write_results_to_folders(
                &results.individual_results,
                &results.relay_results,
                results.meet_title.as_deref(),
                &options,
            )?;
        }
        OutputFormat::Stdout => {
            for event_results in &results.individual_results {
                print_individual_results(event_results, &options);
            }
            for relay_event in &results.relay_results {
                print_relay_results(relay_event, &options);
            }
        }
    }

    let total = results.individual_results.len() + results.relay_results.len();
    println!("\nParsed {} event(s) ({} individual, {} relay)",
             total, results.individual_results.len(), results.relay_results.len());

    Ok(())
}
