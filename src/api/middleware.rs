use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{info, warn, error};

/// Middleware for request logging and timing
pub async fn logging_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    
    // Extract user agent and IP for logging
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    
    let forwarded_for = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    info!(
        "Request started: {} {} from {} ({})",
        method, uri, forwarded_for, user_agent
    );

    let response = next.run(request).await;
    let duration = start.elapsed();
    
    let status = response.status();
    
    if status.is_success() {
        info!(
            "Request completed: {} {} -> {} in {:?}",
            method, uri, status, duration
        );
    } else if status.is_client_error() {
        warn!(
            "Client error: {} {} -> {} in {:?}",
            method, uri, status, duration
        );
    } else {
        error!(
            "Server error: {} {} -> {} in {:?}",
            method, uri, status, duration
        );
    }

    // Log slow requests
    if duration.as_millis() > 1000 {
        warn!(
            "Slow request detected: {} {} took {:?}",
            method, uri, duration
        );
    }

    Ok(response)
}

/// Middleware for rate limiting (basic implementation)
pub async fn rate_limiting_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // TODO: Implement proper rate limiting with Redis or in-memory store
    // For now, just pass through
    
    let headers = request.headers();
    let forwarded_for = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    // Basic rate limiting check (placeholder)
    if forwarded_for == "blocked_ip" {
        warn!("Rate limit exceeded for IP: {}", forwarded_for);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Middleware for API key authentication (for sensitive endpoints)
pub async fn auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let headers = request.headers();
    
    // Check for API key in Authorization header
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(auth) if auth.starts_with("Bearer ") => {
            let token = &auth[7..]; // Remove "Bearer " prefix
            
            // TODO: Implement proper token validation
            if token == "valid_api_key" {
                Ok(next.run(request).await)
            } else {
                warn!("Invalid API key provided: {}", token);
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        Some(_) => {
            warn!("Invalid authorization header format");
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            // For now, allow requests without auth
            // In production, this should be more restrictive
            Ok(next.run(request).await)
        }
    }
}

/// Middleware for CORS handling
pub async fn cors_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    
    // Add CORS headers
    headers.insert(
        "access-control-allow-origin",
        "*".parse().unwrap()
    );
    headers.insert(
        "access-control-allow-methods",
        "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap()
    );
    headers.insert(
        "access-control-allow-headers",
        "content-type, authorization".parse().unwrap()
    );
    headers.insert(
        "access-control-max-age",
        "86400".parse().unwrap()
    );

    Ok(response)
}

/// Middleware for request size limiting
pub async fn request_size_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check content-length header
    if let Some(content_length) = request.headers().get("content-length") {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                const MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024; // 10MB
                
                if length > MAX_REQUEST_SIZE {
                    warn!("Request too large: {} bytes", length);
                    return Err(StatusCode::PAYLOAD_TOO_LARGE);
                }
            }
        }
    }

    Ok(next.run(request).await)
}

/// Middleware for security headers
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let mut response = next.run(request).await;
    
    let headers = response.headers_mut();
    
    // Add security headers
    headers.insert(
        "x-content-type-options",
        "nosniff".parse().unwrap()
    );
    headers.insert(
        "x-frame-options",
        "DENY".parse().unwrap()
    );
    headers.insert(
        "x-xss-protection",
        "1; mode=block".parse().unwrap()
    );
    headers.insert(
        "strict-transport-security",
        "max-age=31536000; includeSubDomains".parse().unwrap()
    );
    headers.insert(
        "referrer-policy",
        "strict-origin-when-cross-origin".parse().unwrap()
    );

    Ok(response)
}

/// Middleware for health check bypass (skip logging for health checks)
pub async fn health_check_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let uri = request.uri().path();
    
    // Skip detailed logging for health check endpoints
    if uri == "/health" || uri == "/metrics" {
        return Ok(next.run(request).await);
    }

    Ok(next.run(request).await)
}
