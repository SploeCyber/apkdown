use std::collections::HashMap;

use reqwest::header::HeaderValue;
use serde::{Deserialize, Serialize};

use crate::provider::Provider;

#[derive(Debug, Copy, Clone)]
pub struct ApkPure {}

#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub os_ver: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectA {
    pub device_info: DeviceInfo,
}

#[derive(Debug, thiserror::Error)]
pub enum ApkPureError {
    #[error("failed to serialize json: {0}")]
    SerJson(serde_json::Error),
    #[error("invalid header value: {0}")]
    HeaderValue(reqwest::header::InvalidHeaderValue),
    #[error("request to {0} failed: {1}")]
    Request(String, reqwest::Error),
    #[error("failed to deserialize json: {0}")]
    DeJson(reqwest::Error),
    #[error("failed to find version: {0}")]
    NoVersion(String),
}

#[derive(Debug, Deserialize)]
pub struct ApkPureAsset {
    pub url: String,
    pub r#type: String,
}

#[derive(Debug, Deserialize)]
pub struct ApkPureVersionInfo {
    pub version_code: String,
    pub version_name: String,
    pub asset: ApkPureAsset,
}

#[derive(Debug, Deserialize)]
pub struct ApkPureRespVer {
    pub version_list: Vec<ApkPureVersionInfo>,
}
#[derive(Debug, Deserialize)]
pub struct ApkPureRespLatest {
    pub app_detail: ApkPureVersionInfo,
}

#[derive(Debug, Serialize)]
pub struct ApkPureAppAction {
    pub action: String,
    pub package_name: String,
}

impl Provider for ApkPure {
    type Error = ApkPureError;
    async fn get_all_versions(
        &self,
        pkg_name: &str,
    ) -> Result<HashMap<String, Option<String>>, Self::Error> {
        let url =
            format!("https://tapi.pureapk.com/v3/get_app_his_version?package_name={pkg_name}");
        let headers = get_apkpure_headers()?;

        let resp: ApkPureRespVer = reqwest::Client::new()
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| ApkPureError::Request(url.to_string(), e))?
            .json()
            .await
            .map_err(|e| ApkPureError::DeJson(e))?;

        let mut result = HashMap::new();

        for ver in resp.version_list {
            result.insert(ver.version_name, Some(ver.asset.url));
        }

        Ok(result)
    }

    async fn get_version_url(&self, pkg_name: &str, version: &str) -> Result<String, Self::Error> {
        let all_v = self.get_all_versions(pkg_name).await?;

        let version = all_v
            .get(version)
            .unwrap_or(&None)
            .as_ref()
            .ok_or(ApkPureError::NoVersion(version.to_string()))?;

        Ok(version.to_string())
    }

    async fn get_latest_version(
        &self,
        pkg_name: &str,
    ) -> Result<Option<(String, Option<String>)>, Self::Error> {
        let headers = get_apkpure_headers()?;

        let url = "https://tapi.pureapk.com/v3/get_app_detail";

        let app_action = ApkPureAppAction {
            action: "Download".into(),
            package_name: pkg_name.to_string(),
        };

        let resp: ApkPureRespLatest = reqwest::Client::new()
            .post(url)
            .headers(headers)
            .json(&app_action)
            .send()
            .await
            .map_err(|e| ApkPureError::Request(url.to_string(), e))?
            .json()
            .await
            .map_err(|e| ApkPureError::DeJson(e))?;

        Ok(Some((
            resp.app_detail.version_name,
            Some(resp.app_detail.asset.url),
        )))
    }
}

fn get_apkpure_headers() -> Result<reqwest::header::HeaderMap, <ApkPure as Provider>::Error> {
    let mut headers = reqwest::header::HeaderMap::new();
    let projecta = ProjectA {
        device_info: DeviceInfo {
            os_ver: "33".into(),
        },
    };
    let projecta_str = serde_json::to_string(&projecta).map_err(|e| ApkPureError::SerJson(e))?;
    headers.insert(
        "Ual-Access-Businessid",
        HeaderValue::from_static("projecta"),
    );
    headers.insert(
        "Ual-Access-ProjectA",
        HeaderValue::from_str(&projecta_str).map_err(|e| ApkPureError::HeaderValue(e))?,
    );
    Ok(headers)
}
