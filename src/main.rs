use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Responder, ResponseError, web};
use serde::Deserialize;
use std::fmt;

// A custom error type to demonstrate how error responses can leak information.
#[derive(Debug)]
struct ApiError {
    message: String,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

// Implementing ResponseError allows actix-web to convert our custom error into an HTTP response.
// In the vulnerable case, we will deliberately include sensitive data in the response.
impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        // This is where the information leak happens!
        // The error message, containing the sensitive URL, is sent to the client.
        HttpResponse::InternalServerError().body(self.message.clone())
    }
}

// Represents the application's configuration, including the sensitive API key.
#[derive(Clone)]
struct AppState {
    api_key: String,
    // A whitelist of allowed hostnames for the secure version.
    allowed_hosts: Vec<String>,
}

// Struct to deserialize query parameters like "?email=test@example.com"
#[derive(Deserialize)]
struct WaitlistParams {
    email: String,
}

/// # Vulnerable Handler
/// This handler extracts the `Host` header and uses it to construct a backend API URL.
/// It uses `.unwrap()` to parse the URL, which will cause a `panic` if the host is invalid,
/// crashing the thread and causing a Denial of Service.
///
/// If the URL parsing itself throws a recoverable error, our custom `ApiError`
/// will leak the constructed URL, including the API key.
async fn vulnerable_waitlist(
    req: HttpRequest,
    query: web::Query<WaitlistParams>,
    state: web::Data<AppState>,
) -> Result<impl Responder, ApiError> {
    // 1. Extract the host header from the user's request.
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(""); // Use empty string if host is not present, similar to the scenario.

    // 2. Construct the backend URL with the sensitive API key.
    let backend_url_str = format!(
        "https://{}/v1/waitlist?api_key={}&email={}",
        host, &state.api_key, &query.email
    );
    log::info!(
        "Vulnerable handler attempting to use URL: {}",
        backend_url_str
    );

    // 3. Attempt to parse the URL. This is where the error occurs.
    // The original JS example had a library that threw an error which Express then
    // printed to the response. We simulate this by returning our custom ApiError,
    // whose ResponseError implementation leaks the details.
    match reqwest::Url::parse(&backend_url_str) {
        Ok(_) => {
            // In a real app, we would make the request here.
            // For this demo, we assume success if parsing works.
            Ok(HttpResponse::Ok()
                .body("Thank you for your interest. You have been added to the waitlist."))
        }
        Err(e) => {
            // VULNERABILITY: The error returned to the user includes the full URL
            // with the API key, and the internal error message.
            let error_message = format!(
                "Failed to construct backend request. URL: '{}', Error: {}",
                backend_url_str, e
            );
            Err(ApiError {
                message: error_message,
            })
        }
    }
}

/// # Secure Handler
/// This handler follows best practices to prevent the vulnerability.
async fn secure_waitlist(
    req: HttpRequest,
    query: web::Query<WaitlistParams>,
    state: web::Data<AppState>,
) -> impl Responder {
    // 1. Extract the host header.
    let host_header = req.headers().get("host").and_then(|h| h.to_str().ok());

    // 2. MITIGATION: Perform input validation.
    // Check if the host is present and is in our whitelist.
    match host_header {
        Some(host) if state.allowed_hosts.contains(&host.to_string()) => {
            // Host is valid, proceed.
        }
        _ => {
            log::warn!(
                "Rejected request with invalid or missing host header: {:?}",
                host_header
            );
            return HttpResponse::BadRequest().body("Invalid 'Host' header provided.");
        }
    };

    // We can safely unwrap here because we've already validated the host.
    let host = host_header.unwrap();

    // 3. Construct the backend URL.
    let backend_url_str = format!(
        "https://{}/v1/waitlist?api_key={}&email={}",
        host, &state.api_key, &query.email
    );
    log::info!("Secure handler attempting to use URL: {}", backend_url_str);

    // 4. MITIGATION: Handle the `Result` gracefully instead of using `unwrap()`.
    // We use a `match` statement to handle both success and failure cases.
    match reqwest::Url::parse(&backend_url_str) {
        Ok(_) => {
            // The URL is valid. We would make the backend `reqwest` call here.
            HttpResponse::Ok().body(
                "Thank you for your interest. We will notify you when we are ready to launch.",
            )
        }
        Err(e) => {
            // Log the detailed error for debugging purposes on the server-side only.
            log::error!(
                "Internal error during URL parsing: {}. URL was: {}",
                e,
                backend_url_str
            );

            // Return a generic error message to the user, hiding internal details.
            HttpResponse::InternalServerError()
                .body("Oops! Something went wrong. Please try again later.")
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Set up logging. This is marked unsafe because modifying environment variables
    // can cause race conditions in highly concurrent code. Since we do it here,
    // at the start of a single-threaded main function, it is safe.
    unsafe {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // Create shared application state
    let app_state = web::Data::new(AppState {
        api_key: "88665751-288d-4175-852f-6519d79fdf1f".to_string(),
        allowed_hosts: vec![
            "my-app.com:8080".to_string(),
            "prod.my-app.com:8080".to_string(),
            "127.0.0.1:8080".to_string(),
        ],
    });

    log::info!("Starting server at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .route("/vulnerable/waitlist", web::get().to(vulnerable_waitlist))
            .route("/secure/waitlist", web::get().to(secure_waitlist))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
