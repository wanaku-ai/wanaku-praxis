use std::time::Duration;

use http::Response;
use reqwest::Client;
use wanaku_praxis_apis::http_response::json_err;

const PROXIED_PREFIXES: &[&str] = &[
    "/api/v1/service-catalog",
    "/api/v1/service-template",
    "/api/v1/data-store",
    "/api/v1/toolset-repos",
];

pub(crate) struct ArtifactRegistryProxy {
    base_url: String,
    client: Client,
}

impl ArtifactRegistryProxy {
    pub(crate) fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self { base_url, client }
    }

    pub(crate) fn should_proxy(path: &str) -> bool {
        PROXIED_PREFIXES.iter().any(|prefix| path.starts_with(prefix))
    }

    pub(crate) async fn forward(
        &self,
        method: &str,
        path: &str,
        query: Option<&str>,
        body: Option<&str>,
    ) -> Response<Vec<u8>> {
        let url = match query {
            Some(q) => format!("{}{path}?{q}", self.base_url),
            None => format!("{}{path}", self.base_url),
        };

        let req_method = match method.parse::<reqwest::Method>() {
            Ok(m) => m,
            Err(_) => return json_err(400, &format!("unsupported method: {method}")),
        };

        let mut request = self.client.request(req_method, &url);

        if let Some(b) = body {
            request = request
                .header("Content-Type", "application/json")
                .body(b.to_owned());
        }

        let response = match request.send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(url = %url, error = %e, "proxy request to artifact registry failed");
                return json_err(502, &format!("upstream error: {e}"));
            }
        };

        let status = response.status().as_u16();
        let response_body = match response.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                tracing::warn!(error = %e, "failed to read artifact registry response body");
                return json_err(502, &format!("upstream body read error: {e}"));
            }
        };

        build_response(status, response_body)
    }
}

#[expect(clippy::expect_used, reason = "valid static response")]
fn build_response(status: u16, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Content-Length", body.len())
        .header(
            "Access-Control-Allow-Origin",
            wanaku_praxis_apis::config::ENV.cors_origin.as_str(),
        )
        .body(body)
        .expect("valid proxy response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_service_catalog() {
        assert!(ArtifactRegistryProxy::should_proxy("/api/v1/service-catalog"));
    }

    #[test]
    fn matches_service_catalog_subpath() {
        assert!(ArtifactRegistryProxy::should_proxy(
            "/api/v1/service-catalog/some/item"
        ));
    }

    #[test]
    fn matches_service_template() {
        assert!(ArtifactRegistryProxy::should_proxy(
            "/api/v1/service-template"
        ));
    }

    #[test]
    fn matches_data_store() {
        assert!(ArtifactRegistryProxy::should_proxy("/api/v1/data-store"));
    }

    #[test]
    fn matches_toolset_repos() {
        assert!(ArtifactRegistryProxy::should_proxy("/api/v1/toolset-repos"));
    }

    #[test]
    fn rejects_tools_path() {
        assert!(!ArtifactRegistryProxy::should_proxy("/api/v1/tools"));
    }

    #[test]
    fn rejects_resources_path() {
        assert!(!ArtifactRegistryProxy::should_proxy("/api/v1/resources"));
    }

    #[test]
    fn rejects_chat_path() {
        assert!(!ArtifactRegistryProxy::should_proxy("/api/v1/chat"));
    }

    #[test]
    fn rejects_healthz() {
        assert!(!ArtifactRegistryProxy::should_proxy("/healthz"));
    }

    #[test]
    fn rejects_empty_path() {
        assert!(!ArtifactRegistryProxy::should_proxy(""));
    }

    #[test]
    fn rejects_root_path() {
        assert!(!ArtifactRegistryProxy::should_proxy("/"));
    }
}
