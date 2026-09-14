// SPDX-License-Identifier: Apache-2.0

use std::io::Read;

use crate::id::{ActionKey, BlobId, CacheEntry};
use crate::kind::Kind;
use crate::{Error, Result, Store};

/// HTTP object store. Same layout as [`crate::Object`]:
/// `blobs/<kind>/<aa>/<hex>` and `actions/<aa>/<hex>`.
pub struct Http {
    base: String,
    token: Option<String>,
}

impl Http {
    pub fn open(base: impl Into<String>) -> Result<Self> {
        let mut base = base.into();
        while base.ends_with('/') {
            base.pop();
        }
        if !(base.starts_with("http://") || base.starts_with("https://")) {
            return Err(Error::invalid(format!(
                "HTTP store needs http(s) URL, got {base}"
            )));
        }
        Ok(Self {
            token: std::env::var("GRAFT_STORE_TOKEN").ok(),
            base,
        })
    }

    fn blob_url(&self, kind: Kind, id: &BlobId) -> String {
        let hex = id.hex();
        format!(
            "{}/blobs/{}/{}/{}",
            self.base,
            kind.as_str(),
            &hex[..2],
            &hex[2..]
        )
    }

    fn action_url(&self, key: &ActionKey) -> String {
        let hex = key.hex();
        format!("{}/actions/{}/{}", self.base, &hex[..2], &hex[2..])
    }

    fn req(&self, method: &str, url: &str) -> ureq::Request {
        let mut req = ureq::request(method, url);
        if let Some(token) = &self.token {
            req = req.set("Authorization", &format!("Bearer {token}"));
        }
        req
    }

    pub fn missing_blobs(&self, wanted: &[(Kind, BlobId)]) -> Vec<(Kind, BlobId)> {
        wanted
            .iter()
            .filter(|(kind, id)| !self.contains_blob(*kind, id))
            .cloned()
            .collect()
    }

    pub fn list_blobs(&self) -> Result<Vec<(Kind, BlobId)>> {
        let body = self
            .req("GET", &format!("{}/blobs", self.base))
            .call()
            .map_err(|e| Error::invalid(e.to_string()))?
            .into_string()
            .map_err(|e| Error::invalid(e.to_string()))?;
        parse_listing(&body, true)
    }

    pub fn list_actions(&self) -> Result<Vec<(ActionKey, CacheEntry)>> {
        let body = self
            .req("GET", &format!("{}/actions", self.base))
            .call()
            .map_err(|e| Error::invalid(e.to_string()))?
            .into_string()
            .map_err(|e| Error::invalid(e.to_string()))?;
        let mut out = Vec::new();
        for line in body.lines() {
            let Some((key, json)) = line.split_once('\t') else {
                continue;
            };
            if let Ok(key) = ActionKey::parse(key) {
                if let Ok(entry) = serde_json::from_str::<CacheEntry>(json) {
                    out.push((key, entry));
                }
            }
        }
        Ok(out)
    }

    pub fn push_from(&self, local: &dyn Store, blobs: &[(Kind, BlobId)]) -> Result<Vec<String>> {
        let mut copied = Vec::new();
        for (kind, id) in blobs {
            if self.contains_blob(*kind, id) {
                continue;
            }
            let bytes = local.get_blob(*kind, id)?;
            let got = self.put_blob(*kind, &bytes)?;
            if &got != id {
                return Err(Error::invalid(format!("http blob {id} hashed as {got}")));
            }
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
            let got = local.put_blob(*kind, &bytes)?;
            if &got != id {
                return Err(Error::invalid(format!("http blob {id} hashed as {got}")));
            }
            copied.push(format!("{kind}/{id}"));
        }
        Ok(copied)
    }
}

impl Store for Http {
    fn put_blob(&self, kind: Kind, bytes: &[u8]) -> Result<BlobId> {
        let id = BlobId::of(bytes);
        resumable_put(&self.req("PUT", &self.blob_url(kind, &id)), bytes)?;
        Ok(id)
    }

    fn get_blob(&self, kind: Kind, id: &BlobId) -> Result<Vec<u8>> {
        match self.req("GET", &self.blob_url(kind, id)).call() {
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
        self.req("HEAD", &self.blob_url(kind, id)).call().is_ok()
    }

    fn get_action(&self, key: &ActionKey) -> Result<Option<CacheEntry>> {
        match self.req("GET", &self.action_url(key)).call() {
            Ok(resp) => {
                let text = resp
                    .into_string()
                    .map_err(|e| Error::invalid(e.to_string()))?;
                let entry =
                    serde_json::from_str(&text).map_err(|e| Error::invalid(e.to_string()))?;
                Ok(Some(entry))
            }
            Err(ureq::Error::Status(404, _)) => Ok(None),
            Err(e) => Err(Error::invalid(e.to_string())),
        }
    }

    fn put_action(&self, key: &ActionKey, entry: CacheEntry) -> Result<()> {
        let bytes = serde_json::to_vec(&entry).map_err(|e| Error::invalid(e.to_string()))?;
        resumable_put(&self.req("PUT", &self.action_url(key)), &bytes)
    }
}

fn resumable_put(req: &ureq::Request, bytes: &[u8]) -> Result<()> {
    const CHUNK: usize = 1024 * 1024;
    let mut offset = 0usize;
    while offset < bytes.len() {
        let end = (offset + CHUNK).min(bytes.len());
        req.clone()
            .set("X-Graft-Offset", &offset.to_string())
            .set("Content-Length", &(end - offset).to_string())
            .send_bytes(&bytes[offset..end])
            .map_err(|e| Error::invalid(e.to_string()))?;
        offset = end;
    }
    Ok(())
}

fn parse_listing(body: &str, blobs: bool) -> Result<Vec<(Kind, BlobId)>> {
    let mut out = Vec::new();
    if !blobs {
        return Ok(out);
    }
    for line in body.lines() {
        let Some((kind, id)) = line.split_once('/') else {
            continue;
        };
        if let (Ok(kind), Ok(id)) = (Kind::parse(kind), BlobId::parse(id)) {
            out.push((kind, id));
        }
    }
    Ok(out)
}

/// Tiny HTTP/1.1 stub for tests. Not a product server.
#[cfg(test)]
pub fn serve_layout(
    root: impl AsRef<std::path::Path>,
) -> Result<(String, std::thread::JoinHandle<()>)> {
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| Error::io("bind", e))?;
    let addr = listener.local_addr().map_err(|e| Error::io("addr", e))?;
    let root = root.as_ref().to_path_buf();
    let handle = std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let _ = handle_conn(stream, &root);
        }
    });
    Ok((format!("http://{addr}"), handle))
}

#[cfg(test)]
fn handle_conn(mut stream: std::net::TcpStream, root: &std::path::Path) -> std::io::Result<()> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf)?;
    let req = String::from_utf8_lossy(&buf[..n]);
    let mut lines = req.split("\r\n");
    let start = lines.next().unwrap_or("");
    let mut parts = start.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");
    let mut offset = 0usize;
    let mut content_len = 0usize;
    for line in lines {
        if line.is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("x-graft-offset:") {
            offset = v.trim().parse().unwrap_or(0);
        }
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_len = v.trim().parse().unwrap_or(0);
        }
    }
    let header_end = req.find("\r\n\r\n").map(|i| i + 4).unwrap_or(n);
    let mut body = buf[header_end.min(n)..n].to_vec();
    while body.len() < content_len {
        let mut more = [0u8; 4096];
        let got = stream.read(&mut more)?;
        if got == 0 {
            break;
        }
        body.extend_from_slice(&more[..got]);
    }
    let rel = path.trim_start_matches('/');
    if method == "GET" && (rel == "blobs" || rel.is_empty()) {
        let listing = list_disk_blobs(root);
        return write_http(&mut stream, 200, listing.as_bytes());
    }
    if method == "GET" && rel == "actions" {
        let listing = list_disk_actions(root);
        return write_http(&mut stream, 200, listing.as_bytes());
    }
    let file = root.join(rel);
    match method {
        "HEAD" => {
            if file.is_file() {
                write_http(&mut stream, 200, b"")
            } else {
                write_http(&mut stream, 404, b"missing")
            }
        }
        "GET" => {
            if let Ok(bytes) = std::fs::read(&file) {
                write_http(&mut stream, 200, &bytes)
            } else {
                write_http(&mut stream, 404, b"missing")
            }
        }
        "PUT" => {
            if let Some(parent) = file.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let part = file.with_extension("part");
            let mut existing = if offset > 0 && part.exists() {
                std::fs::read(&part)?
            } else {
                Vec::new()
            };
            if existing.len() > offset {
                existing.truncate(offset);
            }
            existing.extend_from_slice(&body[..content_len.min(body.len())]);
            std::fs::write(&part, &existing)?;
            std::fs::rename(&part, &file)?;
            write_http(&mut stream, 200, b"ok")
        }
        _ => write_http(&mut stream, 405, b"no"),
    }
}

#[cfg(test)]
fn write_http(stream: &mut std::net::TcpStream, status: u16, body: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        _ => "Error",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    Ok(())
}

#[cfg(test)]
fn list_disk_blobs(root: &std::path::Path) -> String {
    let mut out = String::new();
    let blobs = root.join("blobs");
    let Ok(kinds) = std::fs::read_dir(&blobs) else {
        return out;
    };
    for kind_ent in kinds.flatten() {
        let kind = kind_ent.file_name().to_string_lossy().into_owned();
        let Ok(shards) = std::fs::read_dir(kind_ent.path()) else {
            continue;
        };
        for shard in shards.flatten() {
            let prefix = shard.file_name().to_string_lossy().into_owned();
            let Ok(files) = std::fs::read_dir(shard.path()) else {
                continue;
            };
            for file in files.flatten() {
                if file.path().extension().is_some() {
                    continue;
                }
                let hex = format!("{prefix}{}", file.file_name().to_string_lossy());
                out.push_str(&format!("{kind}/blake3:{hex}\n"));
            }
        }
    }
    out
}

#[cfg(test)]
fn list_disk_actions(root: &std::path::Path) -> String {
    let mut out = String::new();
    let actions = root.join("actions");
    let Ok(shards) = std::fs::read_dir(&actions) else {
        return out;
    };
    for shard in shards.flatten() {
        let prefix = shard.file_name().to_string_lossy().into_owned();
        let Ok(files) = std::fs::read_dir(shard.path()) else {
            continue;
        };
        for file in files.flatten() {
            if file.path().extension().is_some() {
                continue;
            }
            let hex = format!("{prefix}{}", file.file_name().to_string_lossy());
            if let Ok(bytes) = std::fs::read(file.path()) {
                if let Ok(text) = String::from_utf8(bytes) {
                    out.push_str(&format!("{hex}\t{text}\n"));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Memory;

    #[test]
    fn http_missing_push_pull_and_resumable_put() {
        let dir = std::env::temp_dir().join(format!("graft-http-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (base, _thread) = serve_layout(&dir).unwrap();
        let remote = Http::open(&base).unwrap();
        let local = Memory::new();
        let id = local.put_blob(Kind::Material, b"hook-http").unwrap();
        assert_eq!(
            remote.missing_blobs(&[(Kind::Material, id.clone())]),
            vec![(Kind::Material, id.clone())]
        );
        remote
            .push_from(&local, &[(Kind::Material, id.clone())])
            .unwrap();
        assert!(remote.contains_blob(Kind::Material, &id));
        let other = Memory::new();
        remote
            .pull_into(&other, &[(Kind::Material, id.clone())])
            .unwrap();
        assert_eq!(other.get_blob(Kind::Material, &id).unwrap(), b"hook-http");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
