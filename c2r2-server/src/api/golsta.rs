use reqwest::{Client, Response, Url};

use super::{GolstaHarvestList, GolstaHealth};

#[derive(Clone)]
pub struct GolstaClient {
    base_url: Url,
    token: String,
    http: Client,
}

impl GolstaClient {
    pub fn new(base_url: &str, token: String) -> Result<Self, String> {
        let base_url = Url::parse(&format!("{}/", base_url.trim_end_matches('/')))
            .map_err(|e| format!("URL de Golsta inválida: {e}"))?;
        if base_url.scheme() != "http" {
            return Err("La API privada de Golsta debe usar http sobre loopback".to_string());
        }
        let host = base_url
            .host_str()
            .ok_or_else(|| "La URL de Golsta no tiene host".to_string())?;
        let is_loopback = host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|address| address.is_loopback());
        if !is_loopback {
            return Err("La API privada de Golsta debe permanecer en loopback".to_string());
        }
        if token.trim().is_empty() {
            return Err("El token de integración de Golsta está vacío".to_string());
        }
        let http = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(3))
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("No se pudo crear el cliente Golsta: {e}"))?;
        Ok(Self {
            base_url,
            token,
            http,
        })
    }

    pub async fn health(&self) -> Result<GolstaHealth, String> {
        self.get("api/integration/health")
            .send()
            .await
            .map_err(request_error)?
            .error_for_status()
            .map_err(request_error)?
            .json()
            .await
            .map_err(request_error)
    }

    pub async fn harvests(&self) -> Result<GolstaHarvestList, String> {
        self.get("api/integration/harvests")
            .send()
            .await
            .map_err(request_error)?
            .error_for_status()
            .map_err(request_error)?
            .json()
            .await
            .map_err(request_error)
    }

    pub async fn archive(&self, id: &str) -> Result<Response, String> {
        if !valid_archive_id(id) {
            return Err("ID de archivo Golsta inválido".to_string());
        }
        self.get("api/integration/archive")
            .query(&[("id", id)])
            .timeout(std::time::Duration::from_secs(600))
            .send()
            .await
            .map_err(request_error)?
            .error_for_status()
            .map_err(request_error)
    }

    fn get(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .get(self.base_url.join(path).expect("static Golsta API path"))
            .bearer_auth(&self.token)
    }
}

fn request_error(error: reqwest::Error) -> String {
    format!("Golsta backend: {error}")
}

fn valid_archive_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 255
        && id.to_ascii_lowercase().ends_with(".zip")
        && !id.contains("..")
        && !id.contains(['/', '\\', '\r', '\n'])
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        extract::Query,
        http::{HeaderMap, StatusCode},
        response::Response as AxumResponse,
        routing::get,
        Json, Router,
    };
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Deserialize)]
    struct ArchiveQuery {
        id: String,
    }

    #[tokio::test]
    async fn reads_authenticated_health_list_and_archive() {
        async fn health(headers: HeaderMap) -> Result<Json<serde_json::Value>, StatusCode> {
            authorize(&headers)?;
            Ok(Json(
                json!({"status":"ok","service":"golsta","harvest_count":1}),
            ))
        }
        async fn harvests(headers: HeaderMap) -> Result<Json<serde_json::Value>, StatusCode> {
            authorize(&headers)?;
            Ok(Json(json!({"harvests":[{
                "id":"XX_192.0.2.10_LAB_demo_fixture.zip",
                "country":"XX","ip":"192.0.2.10","hostname":"LAB","username":"demo",
                "size_bytes":4,"created_at":"2026-09-23T00:00:00Z",
                "password_count":1,"cookie_count":0,"wallet_count":0
            }],"total":1})))
        }
        async fn archive(
            headers: HeaderMap,
            Query(query): Query<ArchiveQuery>,
        ) -> Result<AxumResponse<Body>, StatusCode> {
            authorize(&headers)?;
            if query.id != "XX_192.0.2.10_LAB_demo_fixture.zip" {
                return Err(StatusCode::NOT_FOUND);
            }
            Ok(AxumResponse::new(Body::from(b"PK\x03\x04".to_vec())))
        }
        fn authorize(headers: &HeaderMap) -> Result<(), StatusCode> {
            match headers.get("authorization").and_then(|v| v.to_str().ok()) {
                Some("Bearer test-token") => Ok(()),
                _ => Err(StatusCode::UNAUTHORIZED),
            }
        }

        let app = Router::new()
            .route("/api/integration/health", get(health))
            .route("/api/integration/harvests", get(harvests))
            .route("/api/integration/archive", get(archive));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let client = GolstaClient::new(&format!("http://{addr}"), "test-token".into()).unwrap();
        assert!(GolstaClient::new("http://192.0.2.10:8080", "test-token".into()).is_err());
        assert_eq!(client.health().await.unwrap().harvest_count, 1);
        assert_eq!(client.harvests().await.unwrap().total, 1);
        assert_eq!(
            client
                .archive("XX_192.0.2.10_LAB_demo_fixture.zip")
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap()
                .as_ref(),
            b"PK\x03\x04"
        );
        assert!(client.archive("../escape.zip").await.is_err());
    }
}
