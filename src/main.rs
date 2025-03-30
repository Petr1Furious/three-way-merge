use clap::{Arg, Command};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

mod merge;

fn parse_path(v: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(v);
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("File '{}' does not exist", v))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    let matches = Command::new("three-way-merge")
        .about("Performs a three-way merge of JSON files")
        .arg(
            Arg::new("base")
                .help("Base version of the file")
                .short('b')
                .long("base")
                .value_parser(parse_path)
                .default_value("base.json"),
        )
        .arg(
            Arg::new("branch_a")
                .help("Branch A version of the file")
                .short('a')
                .long("branch-a")
                .value_parser(parse_path)
                .default_value("branch_a.json"),
        )
        .arg(
            Arg::new("branch_b")
                .help("Branch B version of the file")
                .short('c')
                .long("branch-b")
                .value_parser(parse_path)
                .default_value("branch_b.json"),
        )
        .arg(
            Arg::new("output")
                .help("Output file path")
                .short('o')
                .long("output")
                .default_value("merged.json"),
        )
        .arg(
            Arg::new("verbose")
                .help("Enable verbose logging")
                .short('v')
                .long("verbose")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let base_path = matches.get_one::<PathBuf>("base").unwrap();
    let branch_a_path = matches.get_one::<PathBuf>("branch_a").unwrap();
    let branch_b_path = matches.get_one::<PathBuf>("branch_b").unwrap();
    let output_path = matches.get_one::<String>("output").unwrap();
    let verbose = matches.get_flag("verbose");

    if verbose {
        log::set_max_level(log::LevelFilter::Debug);
    } else {
        log::set_max_level(log::LevelFilter::Info);
    }

    let base_str =
        fs::read_to_string(base_path).map_err(|e| format!("Failed to read base file: {}", e))?;

    let branch_a_str = fs::read_to_string(branch_a_path)
        .map_err(|e| format!("Failed to read branch A file: {}", e))?;

    let branch_b_str = fs::read_to_string(branch_b_path)
        .map_err(|e| format!("Failed to read branch B file: {}", e))?;

    let base_json: Value =
        serde_json::from_str(&base_str).map_err(|e| format!("Failed to parse base JSON: {}", e))?;

    let branch_a_json: Value = serde_json::from_str(&branch_a_str)
        .map_err(|e| format!("Failed to parse branch A JSON: {}", e))?;

    let branch_b_json: Value = serde_json::from_str(&branch_b_str)
        .map_err(|e| format!("Failed to parse branch B JSON: {}", e))?;

    let (merged, had_conflicts) =
        merge::three_way_merge(&base_json, &branch_a_json, &branch_b_json);

    if had_conflicts {
        log::error!("Merge completed with conflicts. See log for details.");
    } else {
        log::info!("Merge completed successfully with no conflicts.");
    }

    let merged_str = serde_json::to_string_pretty(&merged)
        .map_err(|e| format!("Failed to serialize merged JSON: {}", e))?;

    log::info!("Writing output to {}", output_path);
    fs::write(output_path, merged_str)
        .map_err(|e| format!("Failed to write merged output: {}", e))?;

    println!("Merge completed. Output written to {}", output_path);
    if had_conflicts {
        println!("Note: Conflicts occurred during merge. See logs for details.");
        return Err("Merge conflicts detected".into());
    }

    Ok(())
}
