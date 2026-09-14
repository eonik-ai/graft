// SPDX-License-Identifier: Apache-2.0

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::id::{ActionKey, BlobId, CacheEntry};
use crate::kind::Kind;
use crate::{Error, Result, Store};

type HmacSha256 = Hmac<Sha256>;

/// S3 object store. Keys use the same layout as [`crate::Object`].
/// Endpoint from `AWS_ENDPOINT_URL` when set (MinIO / local). Tests skip
/// when AWS credentials are unset.
pub struct S3 {
    bucket: String,
    prefix: String,
    region: String,
    endpoint: String,
    access_key: String,
    secret_key: String,
    session: Option<String>,
}

impl S3 {
    pub fn open(spec: &str) -> Result<Self> {
        let rest = spec
            .strip_prefix("s3://")
            .ok_or_else(|| Error::invalid(format!("S3 store needs s3:// URL, got {spec}")))?;
        let (bucket, prefix) = rest.split_once('/').unwrap_or((rest, ""));
        let prefix = prefix.trim_matches('/').to_string();
        let region = std::env::var("AWS_REGION")
            .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
            .unwrap_or_else(|_| "us-east-1".into());
        let endpoint = std::env::var("AWS_ENDPOINT_URL")
            .unwrap_or_else(|_| format!("https://{bucket}.s3.{region}.amazonaws.com"));
        let access_key = std::env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| Error::invalid("AWS_ACCESS_KEY_ID is required for s3:// remotes"))?;
        let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| Error::invalid("AWS_SECRET_ACCESS_KEY is required for s3:// remotes"))?;
        Ok(Self {
            bucket: bucket.into(),
            prefix,
            region,
            endpoint: endpoint.trim_end_matches('/').into(),
            access_key,
            secret_key,
            session: std::env::var("AWS_SESSION_TOKEN").ok(),
        })
    }

    fn key(&self, kind: Kind, id: &BlobId) -> String {
        let hex = id.hex();
        let path = format!("blobs/{}/{}/{}", kind.as_str(), &hex[..2], &hex[2..]);
        join_prefix(&self.prefix, &path)
    }

    fn action_key(&self, key: &ActionKey) -> String {
        let hex = key.hex();
        join_prefix(
            &self.prefix,
            &format!("actions/{}/{}", &hex[..2], &hex[2..]),
        )
    }

    fn object_url(&self, key: &str) -> String {
        if self.endpoint.contains(&self.bucket) {
            format!("{}/{}", self.endpoint, key)
        } else {
            format!("{}/{}/{}", self.endpoint, self.bucket, key)
        }
    }

    pub fn missing_blobs(&self, wanted: &[(Kind, BlobId)]) -> Vec<(Kind, BlobId)> {
        wanted
            .iter()
            .filter(|(kind, id)| !self.contains_blob(*kind, id))
            .cloned()
            .collect()
    }

    pub fn list_blobs(&self) -> Result<Vec<(Kind, BlobId)>> {
        Err(Error::invalid(
            "s3 list is not implemented; pull named blobs or use the HTTP store listing",
        ))
    }

    pub fn list_actions(&self) -> Result<Vec<(ActionKey, CacheEntry)>> {
        Ok(Vec::new())
    }

    pub fn push_from(&self, local: &dyn Store, blobs: &[(Kind, BlobId)]) -> Result<Vec<String>> {
        let mut copied = Vec::new();
        for (kind, id) in blobs {
            if self.contains_blob(*kind, id) {
                continue;
            }
            let bytes = local.get_blob(*kind, id)?;
            self.put_blob(*kind, &bytes)?;
            copied.push(format!("{kind}/{id}"));
        }
        Ok(copied)
    }

    pub fn pull_into(&self, local: &dyn Store, blobs: &[(Kind, BlobId)]) -> Result<Vec<String>> {
        let mut copied = Vec::new();
        for (kind, id) in blobs {
            if local.contains_blob(*kind, id) {
                continue;
            }
            let bytes = self.get_blob(*kind, id)?;
            local.put_blob(*kind, &bytes)?;
            copied.push(format!("{kind}/{id}"));
        }
        Ok(copied)
    }

    fn signed(&self, method: &str, key: &str, payload: &[u8]) -> Result<ureq::Request> {
        let url = self.object_url(key);
        let host = url
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("")
            .to_string();
        let now = chrono_like_now();
        let date = &now[..8];
        let payload_hash = hex::encode(Sha256::digest(payload));
        let canonical = format!(
            "{method}\n/{}\n\nhost:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{now}\n\nhost;x-amz-content-sha256;x-amz-date\n{payload_hash}",
            key
        );
        let scope = format!("{date}/{}/s3/aws4_request", self.region);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{now}\n{scope}\n{}",
            hex::encode(Sha256::digest(canonical.as_bytes()))
        );
        let signing = signing_key(&self.secret_key, date, &self.region)?;
        let signature = hex::encode(hmac_sha256(&signing, string_to_sign.as_bytes())?);
        let auth = format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders=host;x-amz-content-sha256;x-amz-date, Signature={signature}",
            self.access_key
        );
        let mut req = ureq::request(method, &url)
            .set("host", &host)
            .set("x-amz-content-sha256", &payload_hash)
            .set("x-amz-date", &now)
            .set("Authorization", &auth);
        if let Some(token) = &self.session {
            req = req.set("x-amz-security-token", token);
        }
        Ok(req)
    }
}

impl Store for S3 {
    fn put_blob(&self, kind: Kind, bytes: &[u8]) -> Result<BlobId> {
        let id = BlobId::of(bytes);
        self.signed("PUT", &self.key(kind, &id), bytes)?
            .send_bytes(bytes)
            .map_err(|e| Error::invalid(e.to_string()))?;
        Ok(id)
    }

    fn get_blob(&self, kind: Kind, id: &BlobId) -> Result<Vec<u8>> {
        match self.signed("GET", &self.key(kind, id), b"")?.call() {
            Ok(resp) => {
                let mut out = Vec::new();
                resp.into_reader()
                    .read_to_end(&mut out)
                    .map_err(|e| Error::invalid(e.to_string()))?;
                Ok(out)
            }
            Err(ureq::Error::Status(404, _)) => Err(Error::Missing(format!("{kind}/{id}"))),
            Err(e) => Err(Error::invalid(e.to_string())),
        }
    }

    fn contains_blob(&self, kind: Kind, id: &BlobId) -> bool {
        self.signed("HEAD", &self.key(kind, id), b"")
            .and_then(|req| req.call().map_err(|e| Error::invalid(e.to_string())))
            .is_ok()
    }

    fn get_action(&self, key: &ActionKey) -> Result<Option<CacheEntry>> {
        match self.signed("GET", &self.action_key(key), b"")?.call() {
            Ok(resp) => {
                let text = resp
                    .into_string()
                    .map_err(|e| Error::invalid(e.to_string()))?;
                Ok(Some(
                    serde_json::from_str(&text).map_err(|e| Error::invalid(e.to_string()))?,
                ))
            }
            Err(ureq::Error::Status(404, _)) => Ok(None),
            Err(e) => Err(Error::invalid(e.to_string())),
        }
    }

    fn put_action(&self, key: &ActionKey, entry: CacheEntry) -> Result<()> {
        let bytes = serde_json::to_vec(&entry).map_err(|e| Error::invalid(e.to_string()))?;
        self.signed("PUT", &self.action_key(key), &bytes)?
            .send_bytes(&bytes)
            .map_err(|e| Error::invalid(e.to_string()))?;
        Ok(())
    }
}

fn join_prefix(prefix: &str, rest: &str) -> String {
    if prefix.is_empty() {
        rest.into()
    } else {
        format!("{prefix}/{rest}")
    }
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key).map_err(|e| Error::invalid(e.to_string()))?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn signing_key(secret: &str, date: &str, region: &str) -> Result<Vec<u8>> {
    let k_date = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes())?;
    let k_region = hmac_sha256(&k_date, region.as_bytes())?;
    let k_service = hmac_sha256(&k_region, b"s3")?;
    hmac_sha256(&k_service, b"aws4_request")
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // UTC YYYYMMDD'T'HHMMSS'Z' without extra crates.
    let days = secs / 86400;
    let tod = secs % 86400;
    let (y, m, d) = civil_from_days(days as i64);
    let hh = tod / 3600;
    let mm = (tod % 3600) / 60;
    let ss = tod % 60;
    format!("{y:04}{m:02}{d:02}T{hh:02}{mm:02}{ss:02}Z")
}

fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

use std::io::Read;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn s3_open_requires_credentials() {
        let had_key = std::env::var("AWS_ACCESS_KEY_ID").ok();
        let had_secret = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
        std::env::remove_var("AWS_ACCESS_KEY_ID");
        std::env::remove_var("AWS_SECRET_ACCESS_KEY");
        assert!(S3::open("s3://bucket/prefix").is_err());
        if let Some(key) = had_key {
            std::env::set_var("AWS_ACCESS_KEY_ID", key);
        }
        if let Some(secret) = had_secret {
            std::env::set_var("AWS_SECRET_ACCESS_KEY", secret);
        }
    }

    #[test]
    fn s3_roundtrip_skips_without_endpoint() {
        if std::env::var("GRAFT_S3_TEST").is_err() {
            return;
        }
        let Ok(store) = S3::open("s3://graft-test/cas") else {
            return;
        };
        let id = store.put_blob(Kind::Material, b"s3-probe").unwrap();
        assert_eq!(store.get_blob(Kind::Material, &id).unwrap(), b"s3-probe");
    }
}
