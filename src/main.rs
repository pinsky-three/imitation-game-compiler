use fs_extra::dir::{copy, CopyOptions};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process;
use std::time::Instant;

// --- Type Aliases ---
type DomMap = HashMap<i64, NodeInfo>;
type SimplifiedActionList = Vec<SimplifiedAction>;
type PreprocessingResultData = (DomMap, SimplifiedActionList);

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
    _rrweb_id: i64,
    tag_name: Option<String>,
    attributes: HashMap<String, String>,
    _parent_id: Option<i64>,
    _text_content: Option<String>,
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
    _rrweb_id: i64,
    value: Option<String>,
    timestamp: i64,
    selector: String, // CSS or XPath
}

// --- Main Function ---

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <rrweb_json_path>", args[0]);
        eprintln!("Example: {} recording.json", args[0]);
        process::exit(1);
    }
    let rrweb_json_path = &args[1];

    println!(
        "Starting conversion for '{}' to Stagehand project...",
        rrweb_json_path
    );

    let start_time = Instant::now();

    // Extract initial URL and generate action sequence string
    let (initial_url, action_sequence) = convert_rrweb_to_script(rrweb_json_path).await?;

    // --- Determine Paths ---

    // Find the executable's path to locate the template directory reliably
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path
        .parent()
        .ok_or("Could not get executable's parent directory")?;
    // Adjust based on where the executable is relative to the project root
    // Assuming executable is in target/release/ or target/debug/, go up 3 levels.
    // If run via `cargo run`, cwd might be project root, adjust accordingly.
    // Let's try finding project root by looking for Cargo.toml
    let mut project_root = exe_dir.to_path_buf();
    loop {
        if project_root.join("Cargo.toml").is_file() {
            break;
        }
        if !project_root.pop() {
            return Err("Could not find project root (Cargo.toml) from executable path".into());
        }
    }

    let template_dir = project_root.join("templates/initial_state");
    if !template_dir.is_dir() {
        return Err(format!("Template directory not found at {:?}", template_dir).into());
    }

    let output_base_dir = Path::new("./output"); // Output relative to CWD

    let input_path = Path::new(rrweb_json_path);
    let input_filename_stem = input_path
        .file_stem()
        .ok_or_else(|| {
            format!(
                "Could not get file stem from input path: {}",
                rrweb_json_path
            )
        })?
        .to_str()
        .unwrap(); // Using unwrap based on user change

    // Output project directory (e.g., ./output/rrweb-recording-xyz/)
    let output_project_dir = output_base_dir.join(input_filename_stem);

    println!(
        "Step 5: Preparing output project directory '{:?}'...",
        output_project_dir
    );
    fs::create_dir_all(&output_project_dir)?;

    // --- Copy Template Files ---
    println!(
        "Step 6: Copying Stagehand template files from {:?}...",
        template_dir
    );
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    copy_options.content_only = true; // Copy contents, not the 'initial_state' folder itself

    // Remove unused skip_items variable
    // let skip_items = vec![ ... ];

    // --- TEMPORARY REPLACEMENT FOR fs_extra::copy with filtering ---
    // Manual selective copy (more robust for skipping)
    let entries = fs::read_dir(&template_dir) // Use reference to template_dir PathBuf
        .map_err(|e| {
            format!(
                "Failed to read template directory {:?}: {}",
                template_dir, e
            )
        })?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();

        // Skip unwanted items
        if file_name == "node_modules"
            || file_name == "downloads"
            || file_name == "cache.json"
            || file_name.starts_with(".")
        {
            // Also skip dotfiles for now unless explicitly needed
            continue;
        }

        let dest_path = output_project_dir.join(file_name);

        if path.is_dir() {
            fs::create_dir_all(&dest_path)?;
            // Use fs_extra::dir::copy for recursive directory copying
            // Ensure options are set correctly for subdirectories if needed
            let mut sub_copy_options = CopyOptions::new();
            sub_copy_options.overwrite = true;
            copy(&path, &dest_path, &sub_copy_options).map_err(|e| {
                format!(
                    "Failed to copy directory {:?} to {:?}: {}",
                    path, dest_path, e
                )
            })?;
            println!("  Copied directory: {:?} -> {:?}", path, dest_path);
        } else {
            fs::copy(&path, &dest_path)
                .map_err(|e| format!("Failed to copy file {:?} to {:?}: {}", path, dest_path, e))?;
        }
    }
    // --- END TEMPORARY REPLACEMENT ---

    // --- Fill Template ---
    println!(
        "Step 7: Populating template '{:?}'...",
        output_project_dir.join("index.ts")
    );
    let template_index_path = output_project_dir.join("index.ts");
    let template_content = fs::read_to_string(&template_index_path)?;

    // Replace placeholders
    let final_content = template_content
        .replace("__START_URL__", &initial_url)
        .replace("// __ACTION_SEQUENCE__", &action_sequence); // Replace the comment line

    fs::write(&template_index_path, final_content)?;

    let duration = start_time.elapsed();
    println!("Conversion completed in {:?}", duration);
    println!("Stagehand project created at: {:?}", output_project_dir);
    println!(
        "To run: cd {:?} && npm install && npm run start",
        output_project_dir
    );

    Ok(())
}

async fn convert_rrweb_to_script(
    rrweb_json_path: &str,
) -> Result<(String, String), Box<dyn Error>> {
    // Returns (initial_url, action_sequence_string)
    // Load the recording data
    println!("Step 1: Loading rrweb events...");
    let rrweb_events = load_json_from_file(rrweb_json_path)?;
    println!("Loaded {} events.", rrweb_events.len());

    // Extract initial metadata (like starting URL)
    let initial_url = find_event_by_type(&rrweb_events, 4) // Type 4 is Meta
        .and_then(|event| event.data.get("href"))
        .and_then(|href| href.as_str())
        .map(String::from)
        .unwrap_or_else(|| {
            eprintln!("Warning: Could not find initial URL (Meta event type 4 with href). Using placeholder.");
            "http://example.com".to_string()
        });
    println!("Initial URL: {}", initial_url);

    // --- Stage 1: Pre-processing and Action Extraction ---
    println!("Step 2: Pre-processing and extracting actions...");
    let (dom_map, simplified_actions) = preprocess_rrweb_data(&rrweb_events)?;
    println!("Extracted {} simplified actions.", simplified_actions.len());

    // --- Stage 2: Selector Generation (LLM-Assisted) ---
    println!("Step 3: Generating selectors...");
    let actions_with_selectors = generate_selectors_for_actions(&simplified_actions, &dom_map)?;

    // --- Stage 3: Code Generation ---
    println!("Step 4: Generating action sequence code...");
    let action_sequence = generate_action_sequence_code(&actions_with_selectors).await?;

    Ok((initial_url, action_sequence)) // Return both URL and action string
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
            _rrweb_id: id,
            tag_name: node_data
                .get("tagName")
                .and_then(|v| v.as_str())
                .map(String::from),
            attributes: attributes_map, // Store parsed attributes
            _parent_id: parent_id,
            _text_content: node_data
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
// fn update_dom_map(dom_map: &mut HashMap<i64, NodeInfo>, mutation_data: &Value) {
//     // TODO: Implement logic based on rrweb mutation data format
//     // Needs to handle additions, removals, attribute changes, text changes
//     // Example: Handle added nodes
//     // if let Some(adds) = mutation_data.get("adds") {
//     //     for addition in adds.as_array().unwrap_or(&vec![]) {
//     //          let parent_id = addition.get("parentId").and_then(|v| v.as_i64());
//     //          let node_data = addition.get("node");
//     //          if let (Some(p_id), Some(n_data)) = (parent_id, node_data) {
//     //               parse_dom_snapshot(n_data, dom_map, Some(p_id)); // Need to handle nextId correctly too
//     //          }
//     //     }
//     // }
//     // ... handle removals, attribute changes etc.
// }

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
) -> Result<PreprocessingResultData, Box<dyn Error>> {
    let mut dom_map: DomMap = HashMap::new();
    let mut simplified_actions: SimplifiedActionList = Vec::new();
    let mut current_input_buffer: HashMap<i64, (String, i64)> = HashMap::new();

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

            // Fallback Strategy: If no preferred selector found, mark as failed.
            if !selector_found {
                let tag_name = node_info.tag_name.as_deref().unwrap_or("unknown");
                // Use a specific prefix to identify failed selectors later
                generated_selector = format!("SELECTOR_GENERATION_FAILED::{}", tag_name);
            }
        } else {
            // If node_info is None (shouldn't happen often if preprocessing is robust)
            generated_selector = format!(
                "SELECTOR_GENERATION_FAILED::node_not_found_id_{}",
                action.rrweb_id
            );
        } // Keep the default placeholder if node_info is None

        actions_with_selectors.push(ActionWithSelector {
            action_type: action.action_type.clone(),
            _rrweb_id: action.rrweb_id,
            value: action.value.clone(),
            timestamp: action.timestamp,
            selector: generated_selector, // Use the generated or placeholder selector
        });
    }

    Ok(actions_with_selectors)
}

// --- Stage 3 Helper Function ---
// Generates the sequence of TypeScript Playwright/Stagehand action lines
async fn generate_action_sequence_code(
    actions_with_selectors: &[ActionWithSelector],
) -> Result<String, Box<dyn Error>> {
    let mut action_sequence_code = String::new();

    // Prepend standard cookie consent dismissal
    action_sequence_code.push_str("  // Attempt to dismiss common cookie banners first\n");
    // Use getByRole with a regex for common button texts and a short timeout.
    // Add a .catch() to ignore errors if the button isn't found.
    action_sequence_code.push_str("  await page.getByRole('button', { name: /Accept|Agree|Allow|Got it/i }).click({ timeout: 5000 }).catch(() => { console.log('Cookie banner not found or dismissed already within 5s.'); });\n\n");

    // Loop through actions and generate code lines
    for action in actions_with_selectors {
        // Add timestamp comment (indented)
        action_sequence_code.push_str(&format!("  // Timestamp: {}\n", action.timestamp));

        // Add comment describing the action (indented)
        action_sequence_code.push_str(&format!(
            "  // Action: {:?}, Selector: '{}'",
            action.action_type, action.selector
        ));
        if let Some(val) = &action.value {
            action_sequence_code.push_str(&format!(", Value: '{}'", val));
        }
        action_sequence_code.push('\n');

        // Add actual code generation (indented)
        if action.selector.starts_with("SELECTOR_GENERATION_FAILED::") {
            // If selector generation failed, add a comment and skip the action command
            let failed_tag = action
                .selector
                .split("::")
                .nth(1)
                .unwrap_or("unknown_element");
            action_sequence_code.push_str(&format!(
                "  // Action skipped: Could not generate stable selector for <{}> element.\n",
                failed_tag
            ));
        } else {
            // If selector exists, generate the command wrapped in try...catch
            match action.action_type {
                ActionType::Click => {
                    let escaped_selector = action.selector.replace('`', "\\`").replace('"', "\\\"");
                    // Remove the explicit waitForSelector, rely on action timeout + try/catch
                    action_sequence_code.push_str("  try {\n");
                    action_sequence_code.push_str(&format!(
                        "    await page.locator(\"{}\").click();\n",
                        escaped_selector
                    ));
                    action_sequence_code.push_str("  } catch (error) {\n");
                    action_sequence_code.push_str(&format!(
                        "    console.warn('Action failed for selector [{}]:', (error as Error).message);\n",
                        escaped_selector.replace('\\', " ")
                    )); // Log cleaner selector
                    action_sequence_code.push_str("  }\n");
                }
                ActionType::Input => {
                    if let Some(val) = &action.value {
                        let escaped_selector =
                            action.selector.replace('`', "\\`").replace('"', "\\\"");
                        let escaped_value = val
                            .replace('\\', "\\\\")
                            .replace('`', "\\`")
                            .replace('"', "\\\"");

                        // Remove the explicit waitForSelector

                        let is_obscured = val.len() > 20
                            && val.chars().all(|c| {
                                c.is_ascii_alphanumeric() || c == '=' || c == '+' || c == '/'
                            });

                        action_sequence_code.push_str("  try {\n");
                        if is_obscured {
                            action_sequence_code.push_str(
                                "    // Input value seems obscured/masked, using placeholder:\n",
                            );
                            action_sequence_code.push_str(&format!("    await page.locator(\"{}\").fill(\"TODO: Add realistic test data\");\n", escaped_selector));
                        } else {
                            action_sequence_code.push_str(&format!(
                                "    await page.locator(\"{}\").fill(\"{}\");\n",
                                escaped_selector, escaped_value
                            ));
                        }
                        action_sequence_code.push_str("  } catch (error) {\n");
                        action_sequence_code.push_str(&format!("    console.warn('Action failed for selector [{}]:', (error as Error).message);\n", escaped_selector.replace('\\', ""))); // Log cleaner selector
                        action_sequence_code.push_str("  }\n");
                    }
                }
            }
        }
        action_sequence_code.push('\n'); // Add blank line between actions
    }

    Ok(action_sequence_code.trim_end().to_string()) // Trim trailing whitespace/newlines
}

// --- Other Utility Placeholders ---
// fn get_node_info(map: &HashMap<i64, NodeInfo>, id: i64) -> Option<&NodeInfo> { map.get(&id) }
// fn format_node_context_for_llm(node: &NodeInfo, parent: Option<&NodeInfo>) -> String { /* ... */ String::new() }
// async fn call_llm_selector_api(prompt: &str) -> Result<String, Box<dyn Error>> { Ok("llm_generated_selector".to_string()) }
// async fn call_llm_code_generation_api(prompt: &str) -> Result<String, Box<dyn Error>> { Ok("llm_generated_code".to_string()) }
