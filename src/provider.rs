use std::{collections::HashMap, num::ParseIntError};

use futures_util::StreamExt;

#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    #[error("invalid request: {0}")]
    Request(reqwest::Error),
    #[error("failed to get content length: {0}")]
    ContentLenToStr(reqwest::header::ToStrError),
    #[error("failed to get parse content length: {0}")]
    ParseContentLen(ParseIntError),
}

impl From<reqwest::Error> for DownloadError {
    fn from(value: reqwest::Error) -> Self {
        Self::Request(value)
    }
}

pub trait Provider {
    type Error;
    fn get_all_versions(
        &self,
        pkg_name: &str,
    ) -> impl Future<Output = Result<HashMap<String, Option<String>>, Self::Error>>;
    fn get_version_url(
        &self,
        pkg_name: &str,
        version: &str,
    ) -> impl Future<Output = Result<String, Self::Error>>;

    fn get_latest_version(
        &self,
        pkg_name: &str,
    ) -> impl std::future::Future<Output = Result<Option<(String, Option<String>)>, Self::Error>>
    where
        Self: Sync,
    {
        async {
            let versions = self.get_all_versions(pkg_name).await?;

            Ok(versions
                .iter()
                .max()
                .map(|(version, url)| (version.to_string(), url.as_ref().map(|v| v.to_string()))))
        }
    }
}

pub async fn download_streamed(
    url: &str,
    callback: Option<fn(usize, usize)>,
) -> Result<Vec<u8>, DownloadError> {
    let resp = reqwest::Client::new().get(url).send().await?;

    let content_len = resp.headers().get("Content-Length");

    let mut data = if let Some(len) = content_len {
        Vec::with_capacity(
            len.to_str()
                .map_err(|e| DownloadError::ContentLenToStr(e))?
                .parse()
                .map_err(|e| DownloadError::ParseContentLen(e))?,
        )
    } else {
        Vec::new()
    };
    let total_size = if let Some(len) = content_len {
        len.to_str()
            .map_err(|e| DownloadError::ContentLenToStr(e))?
            .parse()
            .map_err(|e| DownloadError::ParseContentLen(e))?
    } else {
        usize::MAX
    };

    let mut stream = resp.bytes_stream();

    while let Some(item) = stream.next().await {
        let item = item?;

        let current_size = data.len();
        if let Some(callback) = callback {
            callback(current_size, total_size);
        };

        data.extend_from_slice(&item);
    }

    if let Some(callback) = callback {
        callback(data.len(), total_size);
    }

    Ok(data)
}
