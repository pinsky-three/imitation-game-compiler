use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::process;
use std::time::Instant;

// --- Data Structures ---

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Event {
    #[serde(rename = "type")]
    event_type: i64,
    data: Value, // Using Value for flexibility initially
    timestamp: i64,
}

#[derive(Debug, Clone)]
struct NodeInfo {
    // Simplified for now, based on pseudocode
    tag_name: Option<String>,
    attributes: Option<HashMap<String, String>>,
    parent_id: Option<i64>,
    text_content: Option<String>,
    rrweb_id: i64, // Add rrweb_id here for easier reference
}

#[derive(Debug, Clone)]
enum ActionType {
    Click,
    Input,
    // Add other types like Scroll, Navigate, etc. later
}

#[derive(Debug, Clone)]
struct SimplifiedAction {
    action_type: ActionType,
    rrweb_id: i64,         // ID of the element interacted with
    value: Option<String>, // For input actions
    timestamp: i64,
}

#[derive(Debug, Clone)]
struct ActionWithSelector {
    action_type: ActionType,
    rrweb_id: i64,
    value: Option<String>,
    timestamp: i64,
    selector: String, // CSS or XPath
}

// --- Main Function ---

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "Usage: {} <rrweb_json_path> <target_language_framework>",
            args[0]
        );
        eprintln!("Example: {} recording.json python-playwright", args[0]);
        process::exit(1);
    }
    let rrweb_json_path = &args[1];
    let target_language_framework = &args[2]; // e.g., "python-playwright", "js-puppeteer"

    println!(
        "Starting conversion for '{}' targeting '{}'...",
        rrweb_json_path, target_language_framework
    );

    let start_time = Instant::now();

    let automation_script =
        convert_rrweb_to_script(rrweb_json_path, target_language_framework).await?;

    println!(
        "
--- Generated Automation Script ---"
    );
    println!("{}", automation_script);
    println!("---------------------------------");

    let duration = start_time.elapsed();
    println!("Conversion completed in {:?}", duration);

    Ok(())
}

async fn convert_rrweb_to_script(
    rrweb_json_path: &str,
    target_language_framework: &str,
) -> Result<String, Box<dyn Error>> {
    // Load the recording data
    println!("Step 1: Loading rrweb events...");
    let rrweb_events = load_json_from_file(rrweb_json_path)?;
    println!("Loaded {} events.", rrweb_events.len());

    // Extract initial metadata (like starting URL) - Placeholder
    // let meta_event = find_event_by_type(&rrweb_events, 4); // Type 4 is Meta
    let initial_url = "http://example.com"; // Placeholder URL
    println!("Initial URL (placeholder): {}", initial_url);

    // --- Stage 1: Pre-processing and Action Extraction ---
    println!("Step 2: Pre-processing and extracting actions...");
    let (dom_map, simplified_actions) = preprocess_rrweb_data(&rrweb_events)?;
    println!("Extracted {} simplified actions.", simplified_actions.len());

    // --- Stage 2: Selector Generation (LLM-Assisted) ---
    println!("Step 3: Generating selectors (placeholder)...");
    let actions_with_selectors =
        generate_selectors_for_actions(&simplified_actions, &dom_map).await?;

    // --- Stage 3: Code Generation ---
    println!("Step 4: Generating automation code (placeholder)...");
    let automation_script = generate_automation_code(
        &actions_with_selectors,
        target_language_framework,
        initial_url,
    )
    .await?;

    Ok(automation_script)
}

// --- Utility/Placeholder Functions ---

fn load_json_from_file(path: &str) -> Result<Vec<Event>, Box<dyn Error>> {
    let start_load = Instant::now();
    let content = fs::read_to_string(path)?;
    let load_duration = start_load.elapsed();
    println!("  Time to load file: {:?}", load_duration);

    let start_parse = Instant::now();
    let events: Vec<Event> = serde_json::from_str(&content)?;
    let parse_duration = start_parse.elapsed();
    println!("  Time to parse JSON: {:?}", parse_duration);

    Ok(events)
}

// Find the first event of a specific type
fn find_event_by_type(events: &[Event], event_type: i64) -> Option<&Event> {
    events.iter().find(|e| e.event_type == event_type)
}

// Placeholder for recursive DOM snapshot parsing
fn parse_dom_snapshot(
    node_data: &Value,
    dom_map: &mut HashMap<i64, NodeInfo>,
    parent_id: Option<i64>,
) {
    // TODO: Implement recursive parsing based on rrweb snapshot format
    // Needs to handle node structure, attributes, children, text content, etc.
    if let Some(id) = node_data.get("id").and_then(|v| v.as_i64()) {
        let info = NodeInfo {
            rrweb_id: id,
            tag_name: node_data
                .get("tagName")
                .and_then(|v| v.as_str())
                .map(String::from),
            // attributes: node_data.get("attributes")... parse into HashMap ...
            attributes: None, // Placeholder
            parent_id,
            text_content: node_data
                .get("textContent")
                .and_then(|v| v.as_str())
                .map(String::from),
        };
        dom_map.insert(id, info);

        if let Some(children) = node_data.get("childNodes").and_then(|v| v.as_array()) {
            for child_node in children {
                parse_dom_snapshot(child_node, dom_map, Some(id));
            }
        }
    }
}

// Placeholder for applying incremental mutations to the dom_map
fn update_dom_map(dom_map: &mut HashMap<i64, NodeInfo>, mutation_data: &Value) {
    // TODO: Implement logic based on rrweb mutation data format
    // Needs to handle additions, removals, attribute changes, text changes
    // Example: Handle added nodes
    // if let Some(adds) = mutation_data.get("adds") {
    //     for addition in adds.as_array().unwrap_or(&vec![]) {
    //          let parent_id = addition.get("parentId").and_then(|v| v.as_i64());
    //          let node_data = addition.get("node");
    //          if let (Some(p_id), Some(n_data)) = (parent_id, node_data) {
    //               parse_dom_snapshot(n_data, dom_map, Some(p_id)); // Need to handle nextId correctly too
    //          }
    //     }
    // }
    // ... handle removals, attribute changes etc.
}

// Placeholder: Flush buffered input actions
fn flush_input_buffer(
    current_input_buffer: &mut HashMap<i64, (String, i64)>, // Map rrweb_id -> (text, last_timestamp)
    simplified_actions: &mut Vec<SimplifiedAction>,
) {
    for (rrweb_id, (text, last_timestamp)) in current_input_buffer.drain() {
        let action = SimplifiedAction {
            action_type: ActionType::Input,
            rrweb_id,
            value: Some(text),
            timestamp: last_timestamp,
        };
        add_action(simplified_actions, action);
    }
}

// Placeholder: Add action (potentially with simplification logic later)
fn add_action(action_list: &mut Vec<SimplifiedAction>, new_action: SimplifiedAction) {
    action_list.push(new_action);
}

// --- Stage 1 Helper Function ---
fn preprocess_rrweb_data(
    rrweb_events: &[Event],
) -> Result<(HashMap<i64, NodeInfo>, Vec<SimplifiedAction>), Box<dyn Error>> {
    let mut dom_map: HashMap<i64, NodeInfo> = HashMap::new();
    let mut simplified_actions: Vec<SimplifiedAction> = Vec::new();
    let mut current_input_buffer: HashMap<i64, (String, i64)> = HashMap::new(); // Maps rrweb_id -> (text, last_timestamp)

    // Process initial snapshot to build the first dom_map
    if let Some(initial_snapshot_event) = find_event_by_type(rrweb_events, 2) {
        // Type 2 is Full Snapshot
        println!("  Processing initial DOM snapshot...");
        if let Some(node_data) = initial_snapshot_event.data.get("node") {
            parse_dom_snapshot(node_data, &mut dom_map, None);
            println!("  Initial dom_map contains {} nodes.", dom_map.len());
        } else {
            eprintln!("Warning: Full snapshot event found but missing 'node' data.");
        }
    } else {
        return Err("Error: No initial full snapshot (type 2) event found in recording.".into());
    }

    // Process incremental events
    println!("  Processing incremental events...");
    for event in rrweb_events.iter() {
        if event.event_type == 3 {
            // Incremental Snapshot
            if let Some(source_type) = event.data.get("source").and_then(|v| v.as_i64()) {
                match source_type {
                    0 => { // Mutation
                         // update_dom_map(&mut dom_map, &event.data); // TODO: Implement this
                    }
                    2 => {
                        // Mouse Interaction
                        if let Some(interaction_type) =
                            event.data.get("type").and_then(|v| v.as_i64())
                        {
                            if interaction_type == 2 {
                                // Click
                                if let Some(target_id) =
                                    event.data.get("id").and_then(|v| v.as_i64())
                                {
                                    flush_input_buffer(
                                        &mut current_input_buffer,
                                        &mut simplified_actions,
                                    ); // Flush inputs before click
                                    let action = SimplifiedAction {
                                        action_type: ActionType::Click,
                                        rrweb_id: target_id,
                                        value: None,
                                        timestamp: event.timestamp,
                                    };
                                    add_action(&mut simplified_actions, action);
                                }
                            }
                            // TODO: Handle other mouse interactions if needed (e.g., MouseUp)
                        }
                    }
                    5 => {
                        // Input
                        if let (Some(target_id), Some(text)) = (
                            event.data.get("id").and_then(|v| v.as_i64()),
                            event.data.get("text").and_then(|v| v.as_str()),
                        ) {
                            // Buffer input: store last text value and timestamp for this element ID
                            current_input_buffer
                                .insert(target_id, (text.to_string(), event.timestamp));
                        }
                    }
                    // TODO: Handle other source types (Scroll, etc.) if needed
                    _ => {} // Ignore other incremental sources for now
                }
            }
        }
        // TODO: Handle Meta events (type 4) for URL changes mid-recording?
    }

    // Flush any remaining inputs at the end
    flush_input_buffer(&mut current_input_buffer, &mut simplified_actions);

    Ok((dom_map, simplified_actions))
}

// --- Stage 2 Helper Function (Placeholder) ---
async fn generate_selectors_for_actions(
    simplified_actions: &[SimplifiedAction],
    dom_map: &HashMap<i64, NodeInfo>,
) -> Result<Vec<ActionWithSelector>, Box<dyn Error>> {
    let mut actions_with_selectors = Vec::new();

    for action in simplified_actions {
        // TODO: Implement LLM call or other selector generation logic
        // 1. Get node_info from dom_map using action.rrweb_id
        // 2. Get parent_info if available
        // 3. Format context for LLM
        // 4. Call LLM API (placeholder)
        // 5. Create ActionWithSelector

        let placeholder_selector = format!("placeholder_selector_for_id_{}", action.rrweb_id); // Simple placeholder

        actions_with_selectors.push(ActionWithSelector {
            action_type: action.action_type.clone(),
            rrweb_id: action.rrweb_id,
            value: action.value.clone(),
            timestamp: action.timestamp,
            selector: placeholder_selector, // Placeholder
        });
    }

    Ok(actions_with_selectors)
}

// --- Stage 3 Helper Function (Placeholder) ---
async fn generate_automation_code(
    actions_with_selectors: &[ActionWithSelector],
    target_language_framework: &str,
    initial_url: &str,
) -> Result<String, Box<dyn Error>> {
    // TODO: Implement LLM call or template-based code generation
    // For now, just return a summary string

    let mut generated_code = format!(
        "# Automation script generated for target: {}
# Starting URL: {}

",
        target_language_framework, initial_url
    );

    // Add boilerplate start (example for python-playwright)
    if target_language_framework == "python-playwright" {
        generated_code.push_str(
            "from playwright.sync_api import sync_playwright

",
        );
        generated_code.push_str(
            "with sync_playwright() as p:
",
        );
        generated_code.push_str(
            "    browser = p.chromium.launch()
",
        );
        generated_code.push_str(
            "    page = browser.new_page()
",
        );
        generated_code.push_str(&format!("    page.goto(\"{}\")", initial_url));
    } else {
        generated_code.push_str(&format!(
            "# Code generation for '{}' not fully implemented.

",
            target_language_framework
        ));
    }

    for action in actions_with_selectors {
        generated_code.push_str(&format!(
            "    # Action: {:?}, Selector: '{}'",
            action.action_type, action.selector
        ));
        if let Some(val) = &action.value {
            generated_code.push_str(&format!(", Value: '{}'", val));
        }
        generated_code.push('\n');

        // Add actual code generation based on type and target (example for python-playwright)
        if target_language_framework == "python-playwright" {
            match action.action_type {
                ActionType::Click => {
                    let escaped_selector = action.selector.replace('"', "\\\""); // Escape only double quotes for Python
                                                                                 // Use format! directly with escaped selector
                    generated_code.push_str(&format!(
                        r#"    page.locator("{}").click()"#,
                        escaped_selector
                    ));
                    generated_code.push('\n');
                }
                ActionType::Input => {
                    if let Some(val) = &action.value {
                        let escaped_selector = action.selector.replace('"', "\\\"");
                        // Escape backslashes first, then double quotes for the value string literal in Python
                        let escaped_value = val.replace('\\', "\\\\").replace('"', "\\\"");
                        // Use format! directly with escaped selector and value
                        generated_code.push_str(&format!(
                            r#"    page.locator("{}").fill("{}")"#,
                            escaped_selector, escaped_value
                        ));
                        generated_code.push('\n');
                    }
                }
            }
            generated_code.push('\n'); // Add blank line between actions
        }
    }

    // Add boilerplate end (example for python-playwright)
    if target_language_framework == "python-playwright" {
        generated_code.push_str(
            "
    # Example: Add a pause or screenshot
",
        );
        generated_code.push_str(
            "    # page.pause()
",
        );
        generated_code.push_str(
            "    browser.close()
",
        );
    }

    Ok(generated_code)
}

// --- Other Utility Placeholders ---
// fn get_node_info(map: &HashMap<i64, NodeInfo>, id: i64) -> Option<&NodeInfo> { map.get(&id) }
// fn format_node_context_for_llm(node: &NodeInfo, parent: Option<&NodeInfo>) -> String { /* ... */ String::new() }
// async fn call_llm_selector_api(prompt: &str) -> Result<String, Box<dyn Error>> { Ok("llm_generated_selector".to_string()) }
// async fn call_llm_code_generation_api(prompt: &str) -> Result<String, Box<dyn Error>> { Ok("llm_generated_code".to_string()) }
