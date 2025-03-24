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

/// A list of asset types that can be cached.
///
/// This constant defines the types of assets that are eligible for caching.
const ASSETS_TYPE: Lazy<Vec<&str>> =
    Lazy::new(|| vec!["css", "javascript", "wasm", "webp", "ttf", "x-icon"]);

/// Asynchronous function to handle cached responses.
///
/// **Caching Configuration :**
/// - **Result Caching** : The result of this function is cached.
/// - **Cache Type** : Uses `SizedCache<String, (Parts, Bytes)>` for storing cached responses.
/// - **Cache Size** : Initialized with a size of 200 entries.
///
/// **Functionality :**
/// - This function is designed to return cached responses for given paths.
/// - It should never be directly invoked because its purpose is to serve as a caching layer.
///
/// **Parameters :**
/// - `_path`: A `String` representing the path for which the cached response is requested. This parameter is unused within the function.
///
/// **Returns :**
/// - A `Result` containing a tuple `(Parts, Bytes)` on success, or an `AxumError` in case of failure.
#[cached(
    result = true,
    ty = "SizedCache<String, (Parts, Bytes)>",
    create = "{ SizedCache::with_size(200) }"
)]
async fn cached_response(_path: String) -> Result<(Parts, Bytes), AxumError> {
    // ! This function should never be called directly.
    unreachable!()
}

const CACHE_X_K: &str = "x-cache-status";
const CACHE_X_V: &str = "HIT";
const CACHE_C_K: &str = "cache-control";
const CACHE_C_V: &str = "public, max-age=31536000";

/// This middleware function caches responses for requests involving `ASSETS_TYPE` files.
///
/// **Functionality :**
/// - **Cache Hit :** If the requested file is already cached, the function returns the cached response with a [*206 Partial Content* status code](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Status/206)).
/// - **Cache Miss :** If the file is not cached but meets the caching criteria, it caches the response. This cached response will be returned on subsequent requests.
pub async fn handle(req: Request, next: Next) -> Response {
    {
        let mt = CACHED_RESPONSE.lock().await;
        let cache_size = mt.cache_size();
        let cache_capacity = mt.cache_capacity();
        log::debug!("the cache size is {cache_size} / {cache_capacity:?}");
    }

    let uri = req.uri();
    let key = uri.to_string();

    // if `cached`, return `cached`

    if let Some((parts, bytes)) = CACHED_RESPONSE.lock().await.cache_get(&key) {
        let body = Body::from(bytes.to_owned());
        let mut response = Response::from_parts(parts.to_owned(), body);
        *response.status_mut() = StatusCode::PARTIAL_CONTENT;
        return response;
    }

    let mut response = next.run(req).await;

    // if matches the condition, continue
    // else returns response
    if !response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|header_value| {
            header_value
                .to_str()
                .map(|value| ASSETS_TYPE.iter().any(|type_| value.contains(type_)))
                .ok()
        })
        .unwrap_or_default()
    {
        return response;
    }

    // `cached`

    let c_x_v = HeaderValue::from_static(CACHE_X_V);
    let c_c_v = HeaderValue::from_static(CACHE_C_V);
    response.headers_mut().insert(CACHE_X_K, c_x_v);
    response.headers_mut().insert(CACHE_C_K, c_c_v);

    let (parts, body) = response.into_parts();

    match to_bytes(body, usize::MAX).await {
        Ok(bytes) => {
            _ = CACHED_RESPONSE
                .lock()
                .await
                .cache_set(key, (parts.clone(), bytes.clone()));

            Response::from_parts(parts, Body::from(bytes))
        }
        _ => Response::builder()
            .status(StatusCode::INSUFFICIENT_STORAGE)
            .body(Body::from("Failed to convert body to bytes"))
            .unwrap_or_default(),
    }
}
