use axum::{
    body::{to_bytes, Body, Bytes},
    extract::Request,
    http::{header, response::Parts, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
    Error as AxumError,
};
use cached::proc_macro::cached;
use cached::{Cached, SizedCache};
use std::sync::LazyLock as Lazy;

static ASSETS_TYPE: Lazy<Vec<&str>> =
    Lazy::new(|| vec!["css", "javascript", "wasm", "webp", "ttf", "x-icon"]);

#[cached(
    result = true,
    ty = "SizedCache<String, (Parts, Bytes)>",
    create = "{ SizedCache::with_size(200) }"
)]
async fn cached_response(_path: String) -> Result<(Parts, Bytes), AxumError> {
    unreachable!()
}

const CACHE_X_K: &str = "x-cache-status";
const CACHE_X_V: &str = "HIT";
const CACHE_C_K: &str = "cache-control";
const CACHE_C_V: &str = "public, max-age=31536000";

pub async fn handle(req: Request, next: Next) -> Response {
    let uri = req.uri().to_owned();
    let key = uri.to_string();

    // if match uri_key, return `cached`

    if let Some((parts, bytes)) = CACHED_RESPONSE.lock().await.cache_get(&key) {
        let body = Body::from(bytes.to_owned());
        let mut response = Response::from_parts(parts.to_owned(), body);
        *response.status_mut() = StatusCode::PARTIAL_CONTENT;
        return response;
    }

    // if match content type, `cached`

    let response = next.run(req).await;
    let (mut parts, body) = response.into_parts();

    let content_type = parts
        .headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("not_an_asset_type_condition");

    let is_contained = ASSETS_TYPE.iter().any(|at| content_type.contains(at));
    if is_contained {
        let c_x_v = HeaderValue::from_static(CACHE_X_V);
        let c_c_v = HeaderValue::from_static(CACHE_C_V);

        _ = parts.headers.insert(CACHE_X_K, c_x_v);
        _ = parts.headers.insert(CACHE_C_K, c_c_v);

        let bytes = to_bytes(body, usize::MAX)
            .await
            .expect(StatusCode::INSUFFICIENT_STORAGE.as_str());

        _ = CACHED_RESPONSE
            .lock()
            .await
            .cache_set(key, (parts.clone(), bytes.clone()));

        return Response::from_parts(parts, Body::from(bytes));
    }

    // next

    Response::from_parts(parts, body)
}
