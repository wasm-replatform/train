

use std::str::FromStr;

use anyhow::{bail, Context, Result, anyhow};
use async_trait::async_trait;
use http::{HeaderName, HeaderValue, Method as HttpMethod, Request as HttpRequest, Response as HttpResponse};
use jiff::Zoned;
use r9k_position::provider::{Key, Provider, Source, SourceData, Time};
use dilax::provider::HttpRequest as DilaxHttpRequest;
use serde_json::Value;
use wasi::http::outgoing_handler;
use wasi::http::types::{
    FutureIncomingResponse, Headers as WasiHeaders, Method as WasiMethod, OutgoingBody,
    OutgoingRequest, Scheme,
};

use crate::block_mgt::BlockMgtApi;
use crate::gtfs::GtfsApi;

#[derive(Debug, Clone, Default)]
pub struct AppContext {
    gtfs: GtfsApi,
    block_mgt: BlockMgtApi,
}

impl Provider for AppContext {}

impl Time for AppContext {
    fn now(&self) -> Zoned {
        Zoned::now()
    }
}

impl Source for AppContext {
    async fn fetch(&self, _owner: &str, key: &Key) -> Result<SourceData> {
        match key {
            Key::StopInfo(stop_code) => {
                let stop_info = self
                    .gtfs
                    .get_stop_info(stop_code)?
                    .ok_or_else(|| anyhow!("stop info not found for stop code {stop_code}"))?;
                Ok(SourceData::StopInfo(stop_info))
            }
            Key::BlockMgt(train_id) => {
                Ok(SourceData::BlockMgt(self.block_mgt.get_vehicles_by_external_ref_id(train_id)?))
            }
        }
    }
}

pub struct WasiHttpClient;

#[async_trait]
impl DilaxHttpRequest for WasiHttpClient {
    async fn fetch(&self, request: HttpRequest<Vec<u8>>) -> Result<HttpResponse<Vec<u8>>> {
        send_http_request(request)
    }
}

fn send_http_request(request: HttpRequest<Vec<u8>>) -> Result<HttpResponse<Vec<u8>>> {
    let (parts, body) = request.into_parts();
    let http::request::Parts { method, uri, headers, .. } = parts;

    let outgoing = prepare_outgoing_request(method, uri, headers, body)?;
    let future = outgoing_handler::handle(outgoing, None)
        .map_err(|e| anyhow::anyhow!("making request: {e}"))?;
    process_response(&future)
}

fn prepare_outgoing_request(
    method: HttpMethod, uri: http::Uri, headers: http::HeaderMap, body: Vec<u8>,
) -> Result<OutgoingRequest> {
    let wasi_method = match method {
        HttpMethod::GET => WasiMethod::Get,
        HttpMethod::POST => WasiMethod::Post,
        HttpMethod::PUT => WasiMethod::Put,
        HttpMethod::DELETE => WasiMethod::Delete,
        HttpMethod::PATCH => WasiMethod::Patch,
        HttpMethod::OPTIONS => WasiMethod::Options,
        HttpMethod::HEAD => WasiMethod::Head,
        HttpMethod::TRACE => WasiMethod::Trace,
        HttpMethod::CONNECT => WasiMethod::Connect,
        other => bail!("unsupported HTTP method in WASM client: {other}"),
    };

    let wasi_headers = WasiHeaders::new();
    for (name, value) in headers.iter() {
        wasi_headers
            .append(name.as_str(), value.as_bytes())
            .map_err(|e| anyhow::anyhow!("setting header {name}: {e}"))?;
    }

    let request = OutgoingRequest::new(wasi_headers);
    request.set_method(&wasi_method).map_err(|()| anyhow::anyhow!("setting method"))?;

    let scheme = match uri.scheme_str() {
        Some("http") => Scheme::Http,
        Some("https") => Scheme::Https,
        Some(other) => bail!("unsupported URL scheme: {other}"),
        None => bail!("missing URL scheme"),
    };
    request.set_scheme(Some(&scheme)).map_err(|()| anyhow::anyhow!("setting scheme"))?;

    request
        .set_authority(uri.authority().map(|auth| auth.as_str()))
        .map_err(|()| anyhow::anyhow!("setting authority"))?;

    let mut path = uri.path().to_string();
    if let Some(query) = uri.query() {
        path.push('?');
        path.push_str(query);
    }
    request
        .set_path_with_query(Some(&path))
        .map_err(|()| anyhow::anyhow!("setting path_with_query"))?;

    let outgoing_body = request.body().map_err(|()| anyhow::anyhow!("getting outgoing body"))?;
    if !body.is_empty() {
        let out_stream = outgoing_body
            .write()
            .map_err(|()| anyhow::anyhow!("opening body stream"))?;
        let pollable = out_stream.subscribe();
        let mut buf: &[u8] = &body;
        while !buf.is_empty() {
            pollable.block();
            let permit = match out_stream.check_write() {
                Ok(value) => value as usize,
                Err(err) => bail!("output stream is not writable: {err:?}"),
            };
            let len = buf.len().min(permit);
            let (chunk, rest) = buf.split_at(len);
            if out_stream.write(chunk).is_err() {
                bail!("writing request body");
            }
            buf = rest;
        }
        if out_stream.flush().is_err() {
            bail!("flushing request body");
        }
        pollable.block();
        if let Err(err) = out_stream.check_write() {
            bail!("output stream error: {err:?}");
        }
    }
    if let Err(err) = OutgoingBody::finish(outgoing_body, None) {
        bail!("finishing body: {err:?}");
    }
    Ok(request)
}

fn process_response(future: &FutureIncomingResponse) -> Result<HttpResponse<Vec<u8>>> {
    future.subscribe().block();
    let Some(result) = future.get() else {
        bail!("missing response");
    };
    let response = result
        .map_err(|()| anyhow::anyhow!("issue getting response"))?
        .map_err(|e| anyhow::anyhow!("response error: {e}"))?;

    let body = response.consume().map_err(|()| anyhow::anyhow!("getting response body"))?;
    let stream = body.stream().map_err(|()| anyhow::anyhow!("opening body stream"))?;

    let mut bytes = Vec::new();
    while let Ok(chunk) = stream.blocking_read(1024 * 1024) {
        if chunk.is_empty() {
            break;
        }
        bytes.extend_from_slice(&chunk);
    }

    let status = response.status();
    if !(200..300).contains(&status) {
        let message = if bytes.is_empty() {
            String::from("request unsuccessful")
        } else if let Ok(json) = serde_json::from_slice::<Value>(&bytes) {
            json.to_string()
        } else {
            String::from_utf8_lossy(&bytes).to_string()
        };
        bail!("request unsuccessful {status}, {message}");
    }

    let mut builder = HttpResponse::builder().status(status);
    let header_entries = response.headers().entries();
    for (name, value) in header_entries.iter() {
        let header_name = HeaderName::from_str(name)
            .context("failed to parse header")?;
        let header_value = HeaderValue::from_bytes(value)
            .context("failed to parse header value")?;
        builder = builder.header(header_name, header_value);
    }

    drop(stream);
    drop(response);

    builder
        .body(bytes)
        .map_err(|e| anyhow::anyhow!("building HTTP response: {e}"))
}