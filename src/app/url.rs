//! URL query parameters (`?floor=`, `?viz`, `?debug`, … — docs/URL_PARAMS.md).

/// The page URL's query string (e.g. "?viz"), empty if unavailable.
pub(crate) fn url_query() -> String {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default()
}

/// Whether the asset visualizer was requested via `?viz` in the URL.
pub(crate) fn wants_visualizer() -> bool {
    url_query().contains("viz")
}

/// The value of the query parameter `name` (`?name=value`), if present.
pub(crate) fn url_param(name: &str) -> Option<String> {
    let q = url_query();
    q.trim_start_matches('?').split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == name).then(|| v.to_string())
    })
}

/// Whether the query string carries the flag `name` (`?name`, `?name=1`,
/// `?floor=3&name`...).
pub(crate) fn url_flag(name: &str) -> bool {
    let q = url_query();
    q.trim_start_matches('?')
        .split('&')
        .any(|kv| kv.split_once('=').map(|(k, _)| k).unwrap_or(kv) == name)
}
