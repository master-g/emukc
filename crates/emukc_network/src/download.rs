//! Download requests and responses.

use std::{
    fs::{OpenOptions, create_dir_all},
    io::Cursor,
    path::PathBuf,
};

use emukc_crypto::SimpleHash;
use http::header::{CONTENT_ENCODING, CONTENT_TYPE, HeaderValue};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::client::new_reqwest_client;

/// Download request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Request {
    /// URL to download
    pub url: String,

    /// Save as
    pub save_as: PathBuf,

    /// If the file already exists, overwrite it
    pub overwrite: bool,

    /// Skip header check
    pub skip_header_check: bool,
}

/// Download request builder
#[derive(Debug, Clone, Default)]
pub struct Builder {
    url: Option<String>,
    save_as: Option<PathBuf>,
    overwrite: bool,
    skip_header_check: bool,
}

/// Errors that can occur when building a request
#[derive(Debug, Error)]
pub enum BuilderError {
    /// URL is required
    #[error("URL is required")]
    UrlRequired,

    /// Save as is required
    #[error("Save as is required")]
    SaveAsRequired,
}

impl Builder {
    /// Set URL
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set save as
    pub fn save_as(mut self, save_as: impl Into<PathBuf>) -> Self {
        self.save_as = Some(save_as.into());
        self
    }

    /// Set overwrite
    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    /// Set skip header check
    pub fn skip_header_check(mut self, skip_header_check: bool) -> Self {
        self.skip_header_check = skip_header_check;
        self
    }

    /// Build the request
    pub fn build(self) -> Result<Request, BuilderError> {
        let url = self.url.ok_or(BuilderError::UrlRequired)?;
        let save_as = self.save_as.ok_or(BuilderError::SaveAsRequired)?;
        Ok(Request {
            url,
            save_as,
            overwrite: self.overwrite,
            skip_header_check: self.skip_header_check,
        })
    }
}

/// Errors that can occur when downloading
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum DownloadError {
    /// Reqwest client builder error
    #[error("Reqwest client error: {0}")]
    Client(#[source] reqwest::Error),

    /// Request or response body error
    #[error(
		"Reqwest error while {phase} {url} -> {}{}{}{}{}: {source}",
		save_as.display(),
		match final_url {
			Some(final_url) => format!(", final_url={final_url}"),
			None => String::new(),
		},
		match content_type {
			Some(content_type) => format!(", content_type={content_type}"),
			None => String::new(),
		},
		match content_encoding {
			Some(content_encoding) => format!(", content_encoding={content_encoding}"),
			None => String::new(),
		},
		match content_length {
			Some(content_length) => format!(", content_length={content_length}"),
			None => String::new(),
		},
	)]
    Reqwest {
        phase: DownloadPhase,
        url: String,
        save_as: PathBuf,
        final_url: Option<String>,
        content_type: Option<String>,
        content_encoding: Option<String>,
        content_length: Option<u64>,
        #[source]
        source: reqwest::Error,
    },

    /// IO error
    #[error("IO error while {action} {url} -> {} (path={}): {source}", save_as.display(), path.display())]
    Io {
        action: &'static str,
        url: String,
        save_as: PathBuf,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Header check failed
    #[error("HEAD request failed for {url} -> {} with status code: {status}", save_as.display())]
    HeaderCheckFailed {
        url: String,
        save_as: PathBuf,
        status: http::StatusCode,
    },

    /// Response error
    #[error(
		"GET request failed for {url} -> {} with status code: {status}, final_url={final_url}{}",
		save_as.display(),
		match body_preview {
			Some(body_preview) => format!(", body_preview={body_preview:?}"),
			None => String::new(),
		},
	)]
    ResponseError {
        url: String,
        save_as: PathBuf,
        status: http::StatusCode,
        final_url: String,
        body_preview: Option<String>,
    },

    /// File already exists
    #[error("File already exists: {0}")]
    FileAlreadyExists(String),

    /// File not found
    #[error("File not found while downloading {url} -> {} (final_url={final_url})", save_as.display())]
    FileNotFound {
        url: String,
        save_as: PathBuf,
        final_url: String,
    },
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Copy)]
pub enum DownloadPhase {
    SendHead,
    SendGet,
    ReadBody,
}

impl std::fmt::Display for DownloadPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SendHead => f.write_str("sending HEAD request"),
            Self::SendGet => f.write_str("sending GET request"),
            Self::ReadBody => f.write_str("reading response body"),
        }
    }
}

fn header_value_to_string(value: Option<&HeaderValue>) -> Option<String> {
    value.and_then(|value| value.to_str().ok()).map(ToOwned::to_owned)
}

fn preview_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }

    let snippet = String::from_utf8_lossy(bytes);
    let snippet = snippet
        .chars()
        .take(200)
        .map(|ch| {
            if ch.is_control() && !ch.is_whitespace() {
                ' '
            } else {
                ch
            }
        })
        .collect::<String>();
    let snippet = snippet.split_whitespace().collect::<Vec<_>>().join(" ");

    if snippet.is_empty() {
        None
    } else {
        Some(snippet)
    }
}

impl Request {
    /// Create a new builder
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Execute the download
    pub async fn execute(self, client: Option<reqwest::Client>) -> Result<(), DownloadError> {
        let client = match client {
            Some(client) => client,
            None => {
                trace!("using default reqwest client");
                new_reqwest_client(None, None).map_err(DownloadError::Client)?
            }
        };
        let request_url = self.url.clone();
        let requested_save_as = self.save_as.clone();

        if !self.skip_header_check {
            trace!("checking if the file exists via a HEAD request");
            // check if the file exists via a HEAD request
            let head =
                client.head(&self.url).send().await.map_err(|source| DownloadError::Reqwest {
                    phase: DownloadPhase::SendHead,
                    url: request_url.clone(),
                    save_as: requested_save_as.clone(),
                    final_url: None,
                    content_type: None,
                    content_encoding: None,
                    content_length: None,
                    source,
                })?;
            if !head.status().is_success() {
                let status = head.status();
                error!("HEAD request failed with status code: {}", status);
                if status == http::StatusCode::NOT_FOUND {
                    return Err(DownloadError::FileNotFound {
                        url: request_url.clone(),
                        save_as: requested_save_as.clone(),
                        final_url: String::new(),
                    });
                }
                return Err(DownloadError::HeaderCheckFailed {
                    url: request_url.clone(),
                    save_as: requested_save_as.clone(),
                    status,
                });
            }
        }

        // send
        let response = client
            .get(&self.url)
            .header("Cache-Control", "no-cache, no-store, must-revalidate")
            .header("Pragma", "no-cache")
            .header("Expires", "0")
            .send()
            .await
            .map_err(|source| DownloadError::Reqwest {
                phase: DownloadPhase::SendGet,
                url: request_url.clone(),
                save_as: requested_save_as.clone(),
                final_url: None,
                content_type: None,
                content_encoding: None,
                content_length: None,
                source,
            })?;
        let status = response.status();
        let final_url = response.url().to_string();
        if !response.status().is_success() {
            error!("GET request failed with status code: {}", status);
            if status == http::StatusCode::NOT_FOUND {
                return Err(DownloadError::FileNotFound {
                    url: request_url.clone(),
                    save_as: requested_save_as.clone(),
                    final_url,
                });
            }
            let body_preview = match response.bytes().await {
                Ok(bytes) => preview_bytes(&bytes),
                Err(_) => None,
            };
            return Err(DownloadError::ResponseError {
                url: request_url.clone(),
                save_as: requested_save_as.clone(),
                status,
                final_url,
                body_preview,
            });
        }

        // get save path
        let save_as = {
            let save_as = self.save_as;
            if save_as.is_dir() {
                // pre calculate the file name hash
                let url_hash = self.url.simple_hash();
                let fname = response
                    .url()
                    .path_segments()
                    .and_then(std::iter::Iterator::last)
                    .unwrap_or(&url_hash);
                // join
                let fname = std::path::Path::new(fname);
                save_as.join(fname)
            } else if save_as.exists() && !self.overwrite {
                let save_as = save_as.display().to_string();
                return Err(DownloadError::FileAlreadyExists(save_as));
            } else {
                save_as
            }
        };
        let content_type = header_value_to_string(response.headers().get(CONTENT_TYPE));
        let content_encoding = header_value_to_string(response.headers().get(CONTENT_ENCODING));
        let content_length = response.content_length();

        // read body before opening the destination file so failed transfers do not leave 0-byte files behind
        let body = response.bytes().await.map_err(|source| DownloadError::Reqwest {
            phase: DownloadPhase::ReadBody,
            url: request_url.clone(),
            save_as: save_as.clone(),
            final_url: Some(final_url),
            content_type,
            content_encoding,
            content_length,
            source,
        })?;

        // create parent directory
        let parent = save_as.parent().expect("cannot get parent directory of the save path");
        create_dir_all(parent).map_err(|source| DownloadError::Io {
            action: "creating parent directory",
            url: request_url.clone(),
            save_as: save_as.clone(),
            path: parent.to_path_buf(),
            source,
        })?;

        // open
        let mut dest =
            OpenOptions::new().write(true).create(true).truncate(true).open(&save_as).map_err(
                |source| DownloadError::Io {
                    action: "opening destination file",
                    url: request_url.clone(),
                    save_as: save_as.clone(),
                    path: save_as.clone(),
                    source,
                },
            )?;

        // write
        let mut content = Cursor::new(body);
        std::io::copy(&mut content, &mut dest).map_err(|source| DownloadError::Io {
            action: "writing response body",
            url: request_url,
            save_as: save_as.clone(),
            path: save_as,
            source,
        })?;

        Ok(())
    }
}
