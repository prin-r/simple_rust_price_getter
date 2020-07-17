use hyper::{
    client::{Client, HttpConnector},
    header, Body, Request,
};

use base64;
use bytes::buf::ext::BufExt;
use hyper_rustls::HttpsConnector;
use obi::OBIDecode;
use serde::de;
use serde::de::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub const BASE_URI: &str = "http://guanyu-devnet.bandchain.org/rest";

pub fn format_err<T>(e: T) -> String
where
    T: std::fmt::Display,
{
    format!("{}", e)
}

fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: std::fmt::Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}

fn from_base64_to_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    base64::decode(s).map_err(de::Error::custom)
}

#[derive(OBIDecode)]
pub struct Price {
    pub px: u64,
}

pub struct BandSource {
    pub http_client: Client<HttpsConnector<HttpConnector>>,
    pub oracle_script_id: u64,
    pub calldata: String,
    pub min_count: u64,
    pub ask_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleScriptResult {
    pub owner: String,
    pub name: String,
    pub description: String,
    pub filename: String,
    pub schema: String,
    pub source_code_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleScript {
    pub height: String,
    pub result: OracleScriptResult,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BandRequest {
    #[serde(deserialize_with = "from_str")]
    pub height: u64,
    pub result: Res,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Res {
    pub request: Req,
    pub reports: Vec<Report>,
    pub result: Packet,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Req {
    #[serde(deserialize_with = "from_str")]
    pub oracle_script_id: u64,
    #[serde(deserialize_with = "from_base64_to_bytes")]
    pub calldata: Vec<u8>,
    pub requested_validators: Vec<String>,
    #[serde(deserialize_with = "from_str")]
    pub min_count: u64,
    #[serde(deserialize_with = "from_str")]
    pub request_height: u64,
    pub request_time: String,
    pub client_id: String,
    pub raw_requests: Vec<RawRequest>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawRequest {
    #[serde(deserialize_with = "from_str")]
    pub external_id: u64,
    #[serde(deserialize_with = "from_str")]
    pub data_source_id: u64,
    #[serde(deserialize_with = "from_base64_to_bytes")]
    pub calldata: Vec<u8>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub validator: String,
    pub in_before_resolve: bool,
    pub raw_reports: Vec<RawReport>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawReport {
    #[serde(deserialize_with = "from_str")]
    pub external_id: u64,
    pub data: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Packet {
    #[serde(rename = "RequestPacketData")]
    pub request_packet_data: RequestPacketData,
    #[serde(rename = "ResponsePacketData")]
    pub response_packet_data: ResponsePacketData,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestPacketData {
    pub client_id: String,
    #[serde(deserialize_with = "from_str")]
    pub oracle_script_id: u64,
    #[serde(deserialize_with = "from_base64_to_bytes")]
    pub calldata: Vec<u8>,
    #[serde(deserialize_with = "from_str")]
    pub ask_count: u64,
    #[serde(deserialize_with = "from_str")]
    pub min_count: u64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponsePacketData {
    pub client_id: String,
    #[serde(deserialize_with = "from_str")]
    pub request_id: u64,
    #[serde(deserialize_with = "from_str")]
    pub ans_count: u64,
    #[serde(deserialize_with = "from_str")]
    pub request_time: u64,
    #[serde(deserialize_with = "from_str")]
    pub resolve_time: u64,
    pub resolve_status: u64,
    #[serde(deserialize_with = "from_base64_to_bytes")]
    pub result: Vec<u8>,
}

impl BandSource {
    pub fn new(oracle_script_id: u64, calldata: String, min_count: u64, ask_count: u64) -> Self {
        Self {
            http_client: Client::builder().build(HttpsConnector::new()),
            oracle_script_id: oracle_script_id,
            calldata: calldata,
            min_count: min_count,
            ask_count: ask_count,
        }
    }

    pub async fn get_orcle_script(&self) -> Result<OracleScript, String> {
        let mut request = Request::builder()
            .method("GET")
            .uri(&format!(
                "{}/oracle/oracle_scripts/{}",
                BASE_URI, self.oracle_script_id
            ))
            .body(Body::empty())
            .map_err(format_err)?;
        request.headers_mut().insert(
            header::CONTENT_TYPE,
            "application/json".parse().map_err(format_err)?,
        );

        let response = self
            .http_client
            .request(request)
            .await
            .map_err(format_err)?;

        let body = hyper::body::aggregate(response.into_body())
            .await
            .map_err(format_err)?;

        serde_json::from_reader(body.reader()).map_err(format_err)
    }

    pub async fn request_data(&self) -> Result<u64, String> {
        let mut request = Request::builder()
            .method("GET")
            .uri(&format!(
                "{}/oracle/request_search?oid={}&calldata={}&min_count={}&ask_count={}",
                BASE_URI, self.oracle_script_id, self.calldata, self.min_count, self.ask_count
            ))
            .body(Body::empty())
            .map_err(format_err)?;

        request.headers_mut().insert(
            header::CONTENT_TYPE,
            "application/json".parse().map_err(format_err)?,
        );

        let response = self
            .http_client
            .request(request)
            .await
            .map_err(format_err)?;

        let body = hyper::body::aggregate(response.into_body())
            .await
            .map_err(format_err)?;

        let lr: BandRequest = serde_json::from_reader(body.reader()).map_err(format_err)?;

        let price = Price::try_from_slice(&lr.result.result.response_packet_data.result)
            .map_err(format_err)?;

        Ok(price.px)
    }
}
