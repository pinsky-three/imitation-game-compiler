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
    rrweb_id: i64,
    tag_name: Option<String>,
    attributes: HashMap<String, String>,
    parent_id: Option<i64>,
    text_content: Option<String>,
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
    println!("Step 3: Generating selectors...");
    let actions_with_selectors = generate_selectors_for_actions(&simplified_actions, &dom_map)?;

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
    // Needs to handle node structure, attributes, children, text content, etc.
    if let Some(id) = node_data.get("id").and_then(|v| v.as_i64()) {
        let mut attributes_map = HashMap::new();
        if let Some(attrs) = node_data.get("attributes").and_then(|v| v.as_object()) {
            for (key, value) in attrs {
                if let Some(val_str) = value.as_str() {
                    attributes_map.insert(key.clone(), val_str.to_string());
                } else if value.is_number() || value.is_boolean() {
                    // Convert numbers/bools to string representation
                    attributes_map.insert(key.clone(), value.to_string());
                }
                // Ignore other value types for attributes for now
            }
        }

        let info = NodeInfo {
            rrweb_id: id,
            tag_name: node_data
                .get("tagName")
                .and_then(|v| v.as_str())
                .map(String::from),
            attributes: attributes_map, // Store parsed attributes
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

// --- Stage 2 Helper Function ---
fn generate_selectors_for_actions(
    simplified_actions: &[SimplifiedAction],
    dom_map: &HashMap<i64, NodeInfo>,
) -> Result<Vec<ActionWithSelector>, Box<dyn Error>> {
    let mut actions_with_selectors = Vec::new();

    for action in simplified_actions {
        let mut generated_selector = format!("TODO:selector_for_rrweb_id_{}", action.rrweb_id); // Default placeholder

        if let Some(node_info) = dom_map.get(&action.rrweb_id) {
            let mut selector_found = false;

            // Helper function to create attribute selectors, escaping quotes
            let create_attr_selector = |attr: &str, value: &str| -> String {
                format!("*[{} = \"{}\"]", attr, value.replace('"', "\\\""))
            };

            // Strategy 1: Use ID if available and valid
            if let Some(id_val) = node_info.attributes.get("id") {
                if !id_val.is_empty() && !id_val.contains(char::is_whitespace) {
                    generated_selector = format!("#{}", id_val);
                    selector_found = true;
                } else if !id_val.is_empty() {
                    // Use attribute selector for invalid IDs
                    generated_selector = create_attr_selector("id", id_val);
                    selector_found = true;
                }
            }

            // Strategy 2: Use data-testid if ID wasn't found/used
            if !selector_found {
                if let Some(test_id_val) = node_info.attributes.get("data-testid") {
                    if !test_id_val.is_empty() {
                        generated_selector = create_attr_selector("data-testid", test_id_val);
                        selector_found = true;
                    }
                }
            }

            // Strategy 3: Use data-cy if still not found
            if !selector_found {
                if let Some(cy_id_val) = node_info.attributes.get("data-cy") {
                    if !cy_id_val.is_empty() {
                        generated_selector = create_attr_selector("data-cy", cy_id_val);
                        selector_found = true;
                    }
                }
            }

            // Strategy 4: Use name attribute if still not found (often useful for form elements)
            if !selector_found {
                if let Some(name_val) = node_info.attributes.get("name") {
                    if !name_val.is_empty() {
                        // Optionally, be more specific for form elements
                        // if let Some(tag) = &node_info.tag_name {
                        //    if ["input", "button", "select", "textarea"].contains(&tag.to_lowercase().as_str()) {
                        //        generated_selector = format!("{}[name=\"{}\"]", tag, name_val.replace('"', "\\\""));
                        //        selector_found = true;
                        //    }
                        // }
                        // For simplicity, use the general attribute selector for now
                        if !selector_found {
                            // Check again in case the more specific one above was used
                            generated_selector = create_attr_selector("name", name_val);
                            selector_found = true;
                        }
                    }
                }
            }

            // Strategy 5: Use the first class name if still not found
            if !selector_found {
                if let Some(class_val) = node_info.attributes.get("class") {
                    if !class_val.trim().is_empty() {
                        // Split by whitespace and take the first non-empty class
                        if let Some(first_class) = class_val.split_whitespace().next() {
                            if !first_class.is_empty() {
                                // Generate selector like tagname.classname
                                // Escape the class name if it contains special CSS characters (simplistic check)
                                let mut escaped_class = first_class.to_string();
                                // Basic escaping - might need refinement for full CSS spec
                                escaped_class =
                                    escaped_class.replace(':', "\\:").replace('.', "\\.");

                                if let Some(tag) = &node_info.tag_name {
                                    generated_selector = format!("{}.{}", tag, escaped_class);
                                    selector_found = true;
                                } else {
                                    // Fallback to attribute selector if tag name is missing
                                    generated_selector = format!("*.{}", escaped_class);
                                    selector_found = true;
                                }
                            }
                        }
                    }
                }
            }

            // Fallback Strategy: If no preferred selector found, use tag + rrweb_id
            if !selector_found {
                if let Some(tag) = &node_info.tag_name {
                    generated_selector = format!("{}[rrweb_id=\"{}\"]", tag, action.rrweb_id);
                } else {
                    generated_selector = format!("*[rrweb_id=\"{}\"]", action.rrweb_id);
                    // If even tag is missing
                }
            }
        } // If node_info is None, keep the default placeholder

        actions_with_selectors.push(ActionWithSelector {
            action_type: action.action_type.clone(),
            rrweb_id: action.rrweb_id,
            value: action.value.clone(),
            timestamp: action.timestamp,
            selector: generated_selector, // Use the generated or placeholder selector
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
