use std::collections::HashMap;
use std::time::SystemTime;

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use super::types::FileEntry;

type HmacSha256 = Hmac<Sha256>;

/// S3/R2 region for Cloudflare R2.
const R2_REGION: &str = "auto";
/// S3 service name.
const S3_SERVICE: &str = "s3";
/// AWS SigV4 request type.
const AWS4_REQUEST: &str = "aws4_request";

/// Minimal S3/R2 client using reqwest + manual SigV4 signing.
/// Supports list, get, put, delete, head operations against
/// Cloudflare R2 (S3-compatible path-style API).
pub struct S3Client {
    endpoint: String,
    access_key_id: String,
    secret_access_key: String,
    bucket: String,
    http: reqwest::Client,
}

impl S3Client {
    pub fn new(
        endpoint: &str,
        access_key_id: &str,
        secret_access_key: &str,
        bucket: &str,
    ) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            access_key_id: access_key_id.to_string(),
            secret_access_key: secret_access_key.to_string(),
            bucket: bucket.to_string(),
            http: reqwest::Client::builder()
                .build()
                .unwrap_or_default(),
        }
    }

    /// List all objects under a given prefix.
    /// Returns a map of relative_path → FileEntry.
    pub async fn list_objects(
        &self,
        prefix: &str,
    ) -> Result<HashMap<String, FileEntry>, AppError> {
        let mut all_entries = HashMap::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut query_params: Vec<(String, String)> = vec![
                ("list-type".to_string(), "2".to_string()),
                ("prefix".to_string(), prefix.to_string()),
                ("max-keys".to_string(), "1000".to_string()),
            ];
            if let Some(ref token) = continuation_token {
                query_params.push(("continuation-token".to_string(), token.clone()));
            }

            let path = format!("/{}", self.bucket);
            let url = format!("{}{}", self.endpoint, path);

            let resp = self
                .signed_request("GET", &path, &url, &query_params, None, &[])
                .await?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(AppError::S3(format!(
                    "ListObjectsV2 failed: {} {}",
                    status, body
                )));
            }

            let xml = resp.text().await.unwrap_or_default();
            let (entries, next_token) = parse_list_xml(&xml, prefix)?;
            all_entries.extend(entries);

            continuation_token = next_token;
            if continuation_token.is_none() {
                break;
            }
        }

        Ok(all_entries)
    }

    /// Upload a file to the remote.
    pub async fn put_object(
        &self,
        key: &str,
        body: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<String, AppError> {
        let path = format!("/{}/{}", self.bucket, key);
        let url = format!("{}{}", self.endpoint, path);

        let ct = content_type.unwrap_or("application/octet-stream");
        let body_hash = hex::encode(Sha256::digest(&body));

        let resp = self
            .signed_request_with_hash(
                "PUT",
                &path,
                &url,
                &[],
                Some(ct),
                &body,
                &body_hash,
            )
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(AppError::S3(format!(
                "PutObject failed for {}: {} {}",
                key, status, body_text
            )));
        }

        // Return the ETag from response header
        let etag = resp
            .headers()
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim_matches('"').to_string())
            .unwrap_or_default();

        Ok(etag)
    }

    /// Download an object from the remote.
    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>, AppError> {
        let path = format!("/{}/{}", self.bucket, key);
        let url = format!("{}{}", self.endpoint, path);

        let resp = self
            .signed_request("GET", &path, &url, &[], None, &[])
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::S3(format!(
                "GetObject failed for {}: {} {}",
                key, status, body
            )));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::S3(format!("Failed to read response body: {}", e)))?;

        Ok(bytes.to_vec())
    }

    /// Delete an object from the remote.
    pub async fn delete_object(&self, key: &str) -> Result<(), AppError> {
        let path = format!("/{}/{}", self.bucket, key);
        let url = format!("{}{}", self.endpoint, path);

        let resp = self
            .signed_request("DELETE", &path, &url, &[], None, &[])
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::S3(format!(
                "DeleteObject failed for {}: {} {}",
                key, status, body
            )));
        }

        Ok(())
    }

    /// Check if an object exists and get its metadata.
    pub async fn head_object(
        &self,
        key: &str,
    ) -> Result<Option<FileEntry>, AppError> {
        let path = format!("/{}/{}", self.bucket, key);
        let url = format!("{}{}", self.endpoint, path);

        let resp = self
            .signed_request("HEAD", &path, &url, &[], None, &[])
            .await?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !resp.status().is_success() {
            let status = resp.status();
            return Err(AppError::S3(format!(
                "HeadObject failed for {}: {}",
                key, status
            )));
        }

        let headers = resp.headers();
        let etag = headers
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim_matches('"').to_string());
        let size: u64 = headers
            .get("Content-Length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let last_modified = headers
            .get("Last-Modified")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| parse_http_date(s));

        Ok(Some(FileEntry {
            path: key.to_string(),
            size,
            etag,
            mtime: last_modified,
        }))
    }

    /// Make a signed S3 request.
    async fn signed_request(
        &self,
        method: &str,
        path: &str,
        url: &str,
        query_params: &[(String, String)],
        content_type: Option<&str>,
        body: &[u8],
    ) -> Result<reqwest::Response, AppError> {
        let body_hash = hex::encode(Sha256::digest(body));
        self.signed_request_with_hash(
            method,
            path,
            url,
            query_params,
            content_type,
            body,
            &body_hash,
        )
        .await
    }

    /// Make a signed S3 request with a pre-computed body hash.
    async fn signed_request_with_hash(
        &self,
        method: &str,
        path: &str,
        url: &str,
        query_params: &[(String, String)],
        content_type: Option<&str>,
        body: &[u8],
        body_hash: &str,
    ) -> Result<reqwest::Response, AppError> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| AppError::S3(format!("System time error: {}", e)))?;
        let amz_date = format!("{}", chrono::DateTime::<chrono::Utc>::from_timestamp(now.as_secs() as i64, 0)
            .unwrap_or_default()
            .format("%Y%m%dT%H%M%SZ"));
        let date_stamp = amz_date[..8].to_string();

        // Build canonical query string (sorted, URL-encoded)
        let canonical_query = {
            let mut params: Vec<(String, String)> = query_params
                .iter()
                .map(|(k, v)| (uri_encode(k, true), uri_encode(v, true)))
                .collect();
            params.sort();
            params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&")
        };

        // Build canonical headers
        let mut header_lines = Vec::new();
        header_lines.push(format!("host:{}", self.host()));
        header_lines.push(format!("x-amz-content-sha256:{}", body_hash));
        header_lines.push(format!("x-amz-date:{}", amz_date));
        if let Some(ct) = content_type {
            header_lines.push(format!("content-type:{}", ct));
        }

        let canonical_headers = header_lines
            .iter()
            .map(|h| {
                let parts: Vec<&str> = h.splitn(2, ':').collect();
                format!("{}:{}\n", parts[0].trim().to_lowercase(), parts[1].trim())
            })
            .collect::<Vec<_>>()
            .join("");

        let signed_headers = {
            let mut h: Vec<String> = header_lines
                .iter()
                .map(|h| h.splitn(2, ':').next().unwrap_or("").trim().to_lowercase())
                .collect();
            h.sort();
            h.join(";")
        };

        // Build canonical request
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method.to_uppercase(),
            uri_encode(path, false),
            canonical_query,
            canonical_headers,
            signed_headers,
            body_hash,
        );

        // Build string to sign
        let credential_scope = format!("{}/{}/{}/{}", date_stamp, R2_REGION, S3_SERVICE, AWS4_REQUEST);
        let hashed_canonical = hex::encode(Sha256::digest(canonical_request.as_bytes()));
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date, credential_scope, hashed_canonical
        );

        // Calculate signature
        let signing_key = derive_signing_key(&self.secret_access_key, &date_stamp, R2_REGION, S3_SERVICE);
        let signature = hex::encode(hmac_sha256(&signing_key, string_to_sign.as_bytes()));

        // Build Authorization header
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key_id, credential_scope, signed_headers, signature
        );

        // Build request
        let mut req = self
            .http
            .request(reqwest::Method::from_bytes(method.as_bytes())
                .map_err(|e| AppError::S3(format!("Invalid method: {}", e)))?,
            url);

        // Add query params
        if !query_params.is_empty() {
            let query_string: String = query_params
                .iter()
                .map(|(k, v)| format!("{}={}", uri_encode(k, true), uri_encode(v, true)))
                .collect::<Vec<_>>()
                .join("&");
            req = req.query(&query_string);
        }

        req = req
            .header("Host", self.host())
            .header("x-amz-content-sha256", body_hash)
            .header("x-amz-date", &amz_date)
            .header("Authorization", &authorization);

        if let Some(ct) = content_type {
            req = req.header("Content-Type", ct);
        }

        if !body.is_empty() {
            req = req.body(body.to_vec());
        }

        req.send()
            .await
            .map_err(|e| AppError::S3(format!("HTTP request failed: {}", e)))
    }

    /// Extract the host portion from the endpoint URL.
    fn host(&self) -> String {
        self.endpoint
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string()
    }
}

/// Derive the SigV4 signing key.
fn derive_signing_key(secret_key: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(format!("AWS4{}", secret_key).as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, AWS4_REQUEST.as_bytes())
}

/// Compute HMAC-SHA256 and return the raw bytes.
fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key error");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// URI-encode a string according to AWS SigV4 rules.
/// `encode_slash = true` encodes `/` as `%2F` (used in query params).
fn uri_encode(input: &str, encode_slash: bool) -> String {
    use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};

    const FRAGMENT: &AsciiSet = &CONTROLS
        .add(b' ')
        .add(b'!')
        .add(b'"')
        .add(b'#')
        .add(b'$')
        .add(b'%')
        .add(b'&')
        .add(b'\'')
        .add(b'(')
        .add(b')')
        .add(b'*')
        .add(b'+')
        .add(b',')
        .add(b':')
        .add(b';')
        .add(b'<')
        .add(b'=')
        .add(b'>')
        .add(b'?')
        .add(b'@')
        .add(b'[')
        .add(b'\\')
        .add(b']')
        .add(b'^')
        .add(b'`')
        .add(b'{')
        .add(b'|')
        .add(b'}')
        .add(b'~');

    const FRAGMENT_WITH_SLASH: &AsciiSet = &CONTROLS
        .add(b' ')
        .add(b'!')
        .add(b'"')
        .add(b'#')
        .add(b'$')
        .add(b'%')
        .add(b'&')
        .add(b'\'')
        .add(b'(')
        .add(b')')
        .add(b'*')
        .add(b'+')
        .add(b',')
        .add(b':')
        .add(b';')
        .add(b'<')
        .add(b'=')
        .add(b'>')
        .add(b'?')
        .add(b'@')
        .add(b'[')
        .add(b'\\')
        .add(b']')
        .add(b'^')
        .add(b'`')
        .add(b'{')
        .add(b'|')
        .add(b'}')
        .add(b'~')
        .add(b'/');

    if encode_slash {
        utf8_percent_encode(input, FRAGMENT_WITH_SLASH).to_string()
    } else {
        utf8_percent_encode(input, FRAGMENT).to_string()
    }
}

/// Parse an HTTP date header (RFC 7231 format).
fn parse_http_date(date_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    // Example: "Wed, 21 Oct 2015 07:28:00 GMT"
    chrono::DateTime::parse_from_rfc2822(date_str)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

/// Parse S3 ListObjectsV2 XML response.
/// Returns (entries map, optional continuation token).
fn parse_list_xml(
    xml: &str,
    prefix: &str,
) -> Result<(HashMap<String, FileEntry>, Option<String>), AppError> {
    let mut entries = HashMap::new();
    let prefix_trim = prefix.trim_end_matches('/');

    // Extract continuation token
    let next_token = extract_xml_value(xml, "NextContinuationToken");

    // Split by <Contents> blocks
    for block in xml.split("<Contents>").skip(1) {
        let key = match extract_xml_value(block, "Key") {
            Some(k) => k,
            None => continue,
        };

        // Strip the prefix to get the relative path
        let rel_path = if let Some(stripped) = key.strip_prefix(&format!("{}/", prefix_trim)) {
            stripped.to_string()
        } else if key == prefix_trim {
            continue; // This is the prefix itself (directory marker)
        } else {
            key.clone()
        };

        let etag = extract_xml_value(block, "ETag").map(|s| s.trim_matches('"').to_string());
        let size: u64 = extract_xml_value(block, "Size")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let last_modified = extract_xml_value(block, "LastModified")
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        entries.insert(
            rel_path,
            FileEntry {
                path: key,
                size,
                etag,
                mtime: last_modified,
            },
        );
    }

    Ok((entries, next_token))
}

/// Extract the text content of an XML element from a string.
fn extract_xml_value(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].to_string())
}
