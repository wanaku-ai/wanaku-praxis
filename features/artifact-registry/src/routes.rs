use http::Response;
use wanaku_praxis_apis::http_response::json_ok;

use crate::proxy::ArtifactRegistryProxy;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ArtifactRegistryRoute {
    Status,
    Proxy,
    NotFound,
}

pub(crate) fn resolve_route(method: &str, path: &str) -> ArtifactRegistryRoute {
    if method == "GET" && path == "/api/v1/artifact-registry/status" {
        return ArtifactRegistryRoute::Status;
    }

    if ArtifactRegistryProxy::should_proxy(path) {
        return ArtifactRegistryRoute::Proxy;
    }

    ArtifactRegistryRoute::NotFound
}

pub(crate) fn handle_status(configured: bool) -> Response<Vec<u8>> {
    json_ok(&serde_json::json!({"configured": configured}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_route() {
        assert_eq!(
            resolve_route("GET", "/api/v1/artifact-registry/status"),
            ArtifactRegistryRoute::Status
        );
    }

    #[test]
    fn status_wrong_method() {
        assert_eq!(
            resolve_route("POST", "/api/v1/artifact-registry/status"),
            ArtifactRegistryRoute::NotFound
        );
    }

    #[test]
    fn proxy_route() {
        assert_eq!(
            resolve_route("GET", "/api/v1/service-catalog"),
            ArtifactRegistryRoute::Proxy
        );
    }

    #[test]
    fn unknown_route() {
        assert_eq!(
            resolve_route("GET", "/api/v1/unknown"),
            ArtifactRegistryRoute::NotFound
        );
    }
}
