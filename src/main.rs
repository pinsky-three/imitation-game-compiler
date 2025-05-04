use std::env;
use std::fs;
use std::process;
use std::time::Instant;

fn main() {
    // Get file path from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <json_file_path>", args[0]);
        process::exit(1);
    }
    let file_path = &args[1];

    // Read the file content
    let start_load = Instant::now();
    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", file_path, e);
            process::exit(1);
        }
    };
    let load_duration = start_load.elapsed();
    println!("Time to load file: {:?}", load_duration);

    // Deserialize the JSON content into a generic Value first
    // Assuming the root is an array (like rrweb events)
    let data: Result<serde_json::Value, _> = serde_json::from_str(&content);

    match data {
        Ok(serde_json::Value::Array(arr)) => {
            println!("Successfully deserialized JSON.");
            println!("Number of elements in the root array: {}", arr.len());
        }
        Ok(_) => {
            eprintln!("Error: Expected JSON root to be an array, but found something else.");
            process::exit(1);
        }
        Err(e) => {
            eprintln!("Error deserializing JSON from '{}': {}", file_path, e);
            process::exit(1);
        }
    }
}
