// src/config.rs
use serde::Deserialize;
use std::env;
use anyhow::Result;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub region: String,
    pub collection_interval: u64,
    pub target_tag_key: String,
    pub target_tag_value: String,
    pub prometheus_host: String,
    pub prometheus_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            region: "ap-northeast-2".to_string(),
            collection_interval: 60,
            target_tag_key: "env".to_string(),
            target_tag_value: "prd".to_string(),
            prometheus_host: "0.0.0.0".to_string(),  // 모든 인터페이스에서 수신하도록 변경
            prometheus_port: 9043,  // RDS exporter용 포트로 변경
        }
    }
}

pub fn from_env() -> Result<Config> {
    Ok(Config {
        region: env::var("AWS_REGION").unwrap_or_else(|_| "ap-northeast-2".to_string()),
        collection_interval: env::var("COLLECTION_INTERVAL")
            .unwrap_or_else(|_| "60".to_string())
            .parse()?,
        target_tag_key: env::var("TARGET_TAG_KEY").unwrap_or_else(|_| "env".to_string()),
        target_tag_value: env::var("TARGET_TAG_VALUE").unwrap_or_else(|_| "prd".to_string()),
        prometheus_host: env::var("PROMETHEUS_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
        prometheus_port: env::var("PROMETHEUS_PORT")
            .unwrap_or_else(|_| "9043".to_string())
            .parse()?,
    })
}