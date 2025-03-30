// src/config.rs
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub aws: AwsSettings,
    pub exporter: ExporterSettings,
    pub target: TargetSettings,
    pub cloudwatch: CloudWatchSettings,
}

#[derive(Debug, Deserialize)]
pub struct AwsSettings {
    pub region: String,
    pub credentials: Option<AwsCredentials>,
}

#[derive(Debug, Deserialize)]
pub struct AwsCredentials {
    pub profile: String,
}

#[derive(Debug, Deserialize)]
pub struct ExporterSettings {
    pub host: String,
    pub port: u16,
    pub collection_interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct TargetSettings {
    pub tag_key: String,
    pub tag_value: String,
}

#[derive(Debug, Deserialize)]
pub struct CloudWatchSettings {
    pub period: i32,
    pub stat: String,
    pub retry_attempts: u32,
    pub retry_delay: u64,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let run_mode = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());

        let s = Config::builder()
            // 기본 설정 파일
            .add_source(File::with_name("config/default"))
            // 환경별 설정 파일
            .add_source(File::with_name(&format!("config/{}", run_mode)).required(false))
            // 환경 변수로 오버라이드 (예: AWS_REGION, EXPORTER_PORT 등)
            .add_source(Environment::with_prefix("APP"))
            .build()?;

        s.try_deserialize()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            aws: AwsSettings {
                region: "ap-northeast-2".to_string(),
                credentials: None,
            },
            exporter: ExporterSettings {
                host: "0.0.0.0".to_string(),
                port: 9043,
                collection_interval: 60,
            },
            target: TargetSettings {
                tag_key: "env".to_string(),
                tag_value: "prd".to_string(),
            },
            cloudwatch: CloudWatchSettings {
                period: 60,
                stat: "Average".to_string(),
                retry_attempts: 3,
                retry_delay: 1,
            },
        }
    }
}
