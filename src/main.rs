use axum::{
    Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, Uri, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use lol_html::html_content::ContentType;
use lol_html::{ElementContentHandlers, HtmlRewriter, Settings, element};
use reqwest::Client;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::fs;
use url::Url;
use urlencoding;

#[derive(Deserialize)]
struct ProxyParams {
    url: String,
}

struct AppState {
    http_client: Client,
}

#[tokio::main]
async fn main() {
    let shared_state = Arc::new(AppState {
        http_client: Client::builder()
            .user_agent("rrweb-recorder-proxy/1.0") // Be polite with User-Agent
            .build()
            .expect("Failed to build reqwest client"),
    });

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/proxy", get(proxy_handler))
        .with_state(shared_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn serve_index() -> impl IntoResponse {
    match fs::read_to_string("index.html").await {
        Ok(content) => Html(content).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to read index.html",
        )
            .into_response(),
    }
}

async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ProxyParams>,
) -> impl IntoResponse {
    // Thin wrapper that immediately delegates to a helper function
    process_proxy_request(state, params).await
}

// Move all the complex processing to this helper function
async fn process_proxy_request(state: Arc<AppState>, params: ProxyParams) -> Response {
    // Validate and parse the target URL
    let target_url = match Url::parse(&params.url) {
        Ok(url) => {
            if url.scheme() != "http" && url.scheme() != "https" {
                return (StatusCode::BAD_REQUEST, "URL scheme must be http or https")
                    .into_response();
            }
            url
        }
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid URL").into_response(),
    };

    // Fetch the target page
    let fetch_res = match state.http_client.get(target_url.clone()).send().await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Failed to fetch {}: {}", target_url, e);
            return (
                StatusCode::BAD_GATEWAY,
                Html(format!("Failed to fetch upstream URL: {}", e)),
            )
                .into_response();
        }
    };

    // Check content type - only process HTML
    let content_type = fetch_res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    if !content_type.contains("text/html") {
        // This path already builds and returns a Response directly.
        let original_headers = fetch_res.headers().clone();
        let status = fetch_res.status();
        let body = fetch_res.bytes().await.unwrap_or_default();
        let mut filtered_headers = HeaderMap::new();
        for (key, value) in original_headers.iter() {
            let lower_key = key.as_str().to_lowercase();
            if lower_key != "x-frame-options"
                && lower_key != "content-security-policy"
                && lower_key != "content-encoding"
                && lower_key != "transfer-encoding"
            {
                filtered_headers.insert(key.clone(), value.clone());
            }
        }
        if let Some(ct) = original_headers.get(header::CONTENT_TYPE) {
            filtered_headers.insert(header::CONTENT_TYPE, ct.clone());
        }
        filtered_headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());

        let mut response = Response::builder()
            .status(status)
            .body(axum::body::Body::from(body))
            .unwrap_or_else(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to build response",
                )
                    .into_response()
            }); // Keep .into_response() inside closure
        *response.headers_mut() = filtered_headers;
        println!("--- Sending non-HTML Response Headers ---");
        for (key, value) in response.headers() {
            println!(
                "{}: {}",
                key,
                value.to_str().unwrap_or("[invalid header value]")
            );
        }
        println!("---------------------------------------");
        return response; // Return the built Response
    }

    // --- HTML Processing ---
    // rewrite_html_response returns Response. Await and return it directly.
    rewrite_html_response(fetch_res, target_url).await
}

// Helper function to rewrite URLs
fn rewrite_url(url_str: &str, base_url: &Url) -> Result<String, url::ParseError> {
    // Trim whitespace
    let trimmed_url = url_str.trim();

    // Ignore data URLs, javascript: URIs, and empty URLs
    if trimmed_url.starts_with("data:")
        || trimmed_url.starts_with("javascript:")
        || trimmed_url.is_empty()
    {
        return Ok(trimmed_url.to_string());
    }

    // Try to parse the URL relative to the base URL
    match base_url.join(trimmed_url) {
        Ok(abs_url) => {
            // If successful, rewrite it to go through the proxy
            Ok(format!(
                "/proxy?url={}",
                urlencoding::encode(abs_url.as_str())
            ))
        }
        Err(e) => {
            // If parsing fails, return the original string (might be malformed)
            // or handle the error differently if needed. Log the error.
            eprintln!(
                "Failed to parse/join URL '{}' relative to base '{}': {}",
                trimmed_url, base_url, e
            );
            Err(e) // Propagate the error
            // Alternatively, return original: Ok(trimmed_url.to_string())
        }
    }
}

// --- New function to handle HTML rewriting ---
async fn rewrite_html_response(fetch_res: reqwest::Response, target_url: Url) -> Response {
    // First, read the full HTML text from the response (this is the only await inside the function)
    let html_content = match fetch_res.text().await {
        Ok(text) => text,
        Err(e) => {
            eprintln!("Failed to read text from {}: {}", target_url, e);
            return (
                StatusCode::BAD_GATEWAY,
                Html(format!("Failed to read upstream response body: {}", e)),
            )
                .into_response();
        }
    };

    // After this point, there will be NO further `.await`, so non-Send types won't cross await boundaries

    let mut rewritten_html_bytes = Vec::new();

    // Inject rrweb script
    let rrweb_script =
        r#"<script src="https://cdn.jsdelivr.net/npm/rrweb@latest/dist/rrweb.min.js"></script>
           <script>
             window.addEventListener('load', () => {
               if (typeof rrweb !== 'undefined') {
                 console.log('rrweb loaded in iframe, starting recording...');
                 rrweb.record({
                   emit(event) {
                     // Send event to parent window
                     window.parent.postMessage({ type: 'rrwebEvent', event: event }, '*');
                   },
                   recordCanvas: false,
                 });
               } else {
                 console.error('rrweb failed to load inside iframe.');
               }
               // --- Navigation Interception ---
               document.addEventListener('click', (event) => {
                 let target = event.target;
                 while (target && target.tagName !== 'A') { target = target.parentElement; }
                 if (target && target.href) {
                   event.preventDefault();
                   const targetUrl = new URL(target.href, window.location.href).href;
                   console.log('Intercepted navigation to:', targetUrl);
                   window.parent.postMessage({ type: 'navigateProxy', url: targetUrl }, '*');
                 }
               }, true);
             });
           </script>
        "#
        .to_string();

    let base_href = target_url.origin().unicode_serialization();
    let base_tag = format!("<base href=\"{}/\">\n", base_href);

    let target_url_clone = target_url.clone();

    let element_content_handlers = vec![
        element!("head", |el| {
            el.prepend(&base_tag, ContentType::Html);
            el.append(&rrweb_script, ContentType::Html);
            Ok(())
        }),
        element!("[href]", |el| {
            if let Some(href) = el.get_attribute("href") {
                if let Ok(rewritten) = rewrite_url(&href, &target_url_clone) {
                    el.set_attribute("href", &rewritten)?;
                }
            }
            Ok(())
        }),
        element!("[src]", |el| {
            if let Some(src) = el.get_attribute("src") {
                if let Ok(rewritten) = rewrite_url(&src, &target_url_clone) {
                    el.set_attribute("src", &rewritten)?;
                }
            }
            Ok(())
        }),
        element!("[srcset]", |el| {
            if let Some(srcset) = el.get_attribute("srcset") {
                // Full srcset handling
                let rewritten_srcset = srcset
                    .split(',')
                    .map(|part| {
                        let trimmed_part = part.trim();
                        if let Some(url_end) = trimmed_part.find(' ') {
                            let url_part = &trimmed_part[..url_end];
                            let descriptor = &trimmed_part[url_end..];
                            match rewrite_url(url_part, &target_url_clone) {
                                Ok(rewritten_url) => format!("{} {}", rewritten_url, descriptor),
                                Err(_) => trimmed_part.to_string(),
                            }
                        } else {
                            match rewrite_url(trimmed_part, &target_url_clone) {
                                Ok(rewritten_url) => rewritten_url,
                                Err(_) => trimmed_part.to_string(),
                            }
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(", ");
                el.set_attribute("srcset", &rewritten_srcset)?;
            }
            Ok(())
        }),
        // TODO: Add more handlers
    ];

    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers,
            ..Settings::default()
        },
        |c: &[u8]| rewritten_html_bytes.extend_from_slice(c),
    );

    // Run the rewriter synchronously (no await)
    if let Err(e) = rewriter.write(html_content.as_bytes()) {
        eprintln!("HTML rewriting error: {}", e);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to rewrite HTML").into_response();
    }
    if let Err(e) = rewriter.end() {
        eprintln!("HTML rewriting end error: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to finalize HTML rewriting",
        )
            .into_response();
    }

    // Build headers and final Response (same as before)
    let mut final_headers = HeaderMap::new();
    final_headers.insert(
        header::CONTENT_TYPE,
        "text/html; charset=utf-8".parse().unwrap(),
    );
    final_headers.insert(
        header::CACHE_CONTROL,
        "no-store, no-cache, must-revalidate, proxy-revalidate"
            .parse()
            .unwrap(),
    );
    final_headers.insert(header::PRAGMA, "no-cache".parse().unwrap());
    final_headers.insert(header::EXPIRES, "0".parse().unwrap());
    final_headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());

    let final_html = String::from_utf8(rewritten_html_bytes).unwrap_or_else(|e| {
        eprintln!("Rewritten HTML is not valid UTF-8: {}", e);
        "Error: Rewritten content is not valid UTF-8".to_string()
    });
    let mut response = (final_headers, Html(final_html)).into_response();

    response.headers_mut().remove(header::X_FRAME_OPTIONS);
    response
        .headers_mut()
        .remove(header::CONTENT_SECURITY_POLICY);

    println!("--- Sending HTML Response Headers ---");
    for (key, value) in response.headers() {
        println!(
            "{}: {}",
            key,
            value.to_str().unwrap_or("[invalid header value]")
        );
    }
    println!("-----------------------------------");

    response
}
// --- End of new function ---
