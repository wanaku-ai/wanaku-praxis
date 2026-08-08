#![deny(unsafe_code)]

mod proxy;
mod routes;

use http::Response;
use praxis_filter::{FilterRegistry, PipelineExtension};
use wanaku_praxis_apis::http_response::json_err;

use wanaku_praxis_apis::feature::Feature;

use crate::proxy::ArtifactRegistryProxy;
use crate::routes::{ArtifactRegistryRoute, handle_status, resolve_route};

pub struct ArtifactRegistryFeature {
    proxy: Option<ArtifactRegistryProxy>,
}

impl ArtifactRegistryFeature {
    #[must_use]
    pub fn new(base_url: Option<String>) -> Self {
        let proxy = base_url.map(ArtifactRegistryProxy::new);
        if proxy.is_some() {
            tracing::info!("artifact registry proxy enabled");
        }
        Self { proxy }
    }
}

#[async_trait::async_trait]
impl Feature for ArtifactRegistryFeature {
    fn name(&self) -> &'static str {
        "artifact-registry"
    }

    fn register_filters(&self, _registry: &mut FilterRegistry) {}

    fn pipeline_extensions(&self) -> Vec<Box<dyn PipelineExtension>> {
        vec![]
    }

    async fn handle_route(
        &self,
        method: &str,
        path: &str,
        query: Option<&str>,
        body: Option<&str>,
    ) -> Option<Response<Vec<u8>>> {
        let route = resolve_route(method, path);
        match route {
            ArtifactRegistryRoute::Status => Some(handle_status(self.proxy.is_some())),
            ArtifactRegistryRoute::Proxy => {
                let Some(proxy) = &self.proxy else {
                    return Some(json_err(
                        503,
                        "artifact registry not configured (set WANAKU_ARTIFACT_REGISTRY_URL)",
                    ));
                };
                Some(proxy.forward(method, path, query, body).await)
            }
            ArtifactRegistryRoute::NotFound => None,
        }
    }

    fn load_yaml_config(&self, _root: &serde_yaml::Value) {}

    fn load_env_config(&self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn unconfigured_proxy_returns_503_for_proxy_paths() {
        let feature = ArtifactRegistryFeature::new(None);
        let response = feature
            .handle_route("GET", "/api/v1/service-catalog", None, None)
            .await;
        let response = response.expect("should return Some for known proxy path");
        assert_eq!(response.status(), 503);
    }

    #[tokio::test]
    async fn unconfigured_proxy_returns_none_for_unknown_paths() {
        let feature = ArtifactRegistryFeature::new(None);
        let response = feature
            .handle_route("GET", "/api/v1/tools", None, None)
            .await;
        assert!(response.is_none());
    }

    #[tokio::test]
    async fn status_reports_unconfigured() {
        let feature = ArtifactRegistryFeature::new(None);
        let response = feature
            .handle_route("GET", "/api/v1/artifact-registry/status", None, None)
            .await;
        let response = response.expect("should return Some for status");
        assert_eq!(response.status(), 200);
        let body: serde_json::Value =
            serde_json::from_slice(response.body()).expect("valid json");
        assert_eq!(body["data"]["configured"], false);
    }

    #[tokio::test]
    async fn status_reports_configured() {
        let feature =
            ArtifactRegistryFeature::new(Some("http://localhost:8080".to_owned()));
        let response = feature
            .handle_route("GET", "/api/v1/artifact-registry/status", None, None)
            .await;
        let response = response.expect("should return Some for status");
        assert_eq!(response.status(), 200);
        let body: serde_json::Value =
            serde_json::from_slice(response.body()).expect("valid json");
        assert_eq!(body["data"]["configured"], true);
    }
}
