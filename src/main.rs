use axum::{
    Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, Uri, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use reqwest::Client;
use scraper::{Html as ScraperHtml, Node, Selector};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::fs;
use url::Url;

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
                format!("Failed to fetch upstream URL: {}", e),
            )
                .into_response();
        }
    };

    // Check content type - only process HTML
    let content_type = fetch_res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    if !content_type.contains("text/html") {
        // If not HTML, try to stream the response directly
        // Note: This is basic, might need more robust handling for different content types
        let headers = fetch_res.headers().clone();
        let status = fetch_res.status();
        let body = fetch_res.bytes().await.unwrap_or_default(); // Consider streaming
        let mut response_headers = HeaderMap::new();
        for (key, value) in headers.iter() {
            // Avoid forwarding problematic headers like content-encoding if we aren't handling it
            // Also remove frame-blocking headers
            let lower_key = key.as_str().to_lowercase();
            if lower_key != "content-encoding"
                && lower_key != "transfer-encoding"
                && lower_key != "x-frame-options"
                && lower_key != "content-security-policy"
            // Removing CSP entirely is broad, but simplest for now
            {
                response_headers.insert(key.clone(), value.clone());
            }
        }
        // Crucially set the content-type header from the original response
        if let Some(ct) = headers.get(header::CONTENT_TYPE) {
            response_headers.insert(header::CONTENT_TYPE, ct.clone());
        }
        // Allow access from any origin
        response_headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());

        return Response::builder()
            .status(status)
            .body(axum::body::Body::from(body))
            .unwrap_or_else(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to build response",
                )
                    .into_response()
            })
            .into_response();
    }

    // Read HTML content
    let html_content = match fetch_res.text().await {
        Ok(text) => text,
        Err(e) => {
            eprintln!("Failed to read text from {}: {}", target_url, e);
            return (
                StatusCode::BAD_GATEWAY,
                format!("Failed to read upstream response body: {}", e),
            )
                .into_response();
        }
    };

    // Parse HTML
    let document = ScraperHtml::parse_document(&html_content);
    let head_selector = Selector::parse("head").unwrap();
    let body_selector = Selector::parse("body").unwrap();

    // Inject rrweb script and communication script into <head>
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
                   // Important: Disable canvas recording unless specifically handled
                   recordCanvas: false, 
                 });
               } else {
                 console.error('rrweb failed to load inside iframe.');
               }

               // --- Navigation Interception ---
               document.addEventListener('click', (event) => {
                 let target = event.target;
                 // Find the nearest ancestor anchor tag
                 while (target && target.tagName !== 'A') {
                     target = target.parentElement;
                 }

                 if (target && target.href) {
                     // Prevent default navigation
                     event.preventDefault();

                     // Resolve the target URL (handles relative paths correctly)
                     const targetUrl = new URL(target.href, window.location.href).href;
                     
                     console.log('Intercepted navigation to:', targetUrl);

                     // Tell parent window to navigate the iframe via proxy
                     window.parent.postMessage({ type: 'navigateProxy', url: targetUrl }, '*');
                 }
               }, true); // Use capture phase to catch clicks early

             });
           </script>
        "#
        .to_string();
    // Inject a <base> tag to fix relative URLs
    let base_tag = format!(
        "<base href=\"{}/\">
",
        target_url.origin().unicode_serialization()
    );

    let mut modified_html = String::new();
    // Reconstruct HTML carefully - Add a generic DOCTYPE
    modified_html.push_str("<!DOCTYPE html>\n");
    modified_html.push_str("<html>");

    if let Some(head_node) = document.select(&head_selector).next() {
        modified_html.push_str("<head>");
        modified_html.push_str(&base_tag);
        modified_html.push_str(&rrweb_script);
        modified_html.push_str(&head_node.inner_html()); // Add original head content
        modified_html.push_str("</head>");
    } else {
        // If no head, create one
        modified_html.push_str("<head>");
        modified_html.push_str(&base_tag);
        modified_html.push_str(&rrweb_script);
        modified_html.push_str("</head>");
    }

    if let Some(body_node) = document.select(&body_selector).next() {
        modified_html.push_str("<body>");
        modified_html.push_str(&body_node.inner_html()); // Add original body content
        modified_html.push_str("</body>");
    } else {
        modified_html.push_str("<body>");
        // If no body tag, append the rest of the document content
        modified_html.push_str(&document.html());
        modified_html.push_str("</body>");
    }

    modified_html.push_str("</html>");

    // Ensure frame-blocking headers are not present in the final response for HTML
    let mut final_headers = HeaderMap::new();
    final_headers.insert(
        header::CONTENT_TYPE,
        "text/html; charset=utf-8".parse().unwrap(),
    );
    // Add headers to prevent caching
    final_headers.insert(
        header::CACHE_CONTROL,
        "no-store, no-cache, must-revalidate, proxy-revalidate"
            .parse()
            .unwrap(),
    );
    final_headers.insert(header::PRAGMA, "no-cache".parse().unwrap());
    final_headers.insert(header::EXPIRES, "0".parse().unwrap());
    // Allow access from any origin for the HTML response as well
    final_headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*".parse().unwrap());
    // Add other desired headers here if needed

    // Return the modified HTML with the cleaned headers
    (final_headers, Html(modified_html)).into_response()
}
