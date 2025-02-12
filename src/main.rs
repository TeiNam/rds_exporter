// src/main.rs
use aws_config::BehaviorVersion;
use aws_sdk_cloudwatch::Client as CloudWatchClient;
use aws_sdk_rds::Client as RdsClient;
use chrono::Duration;
use std::net::SocketAddr;
use std::convert::Infallible;
use tracing::{error, info };
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use warp::Filter;
use prometheus::{Encoder, TextEncoder};

use crate::aws::cloudwatch::{CloudWatchCollector, MetricConfig as CWConfig};
use crate::aws::rds::{RdsConfig, RdsInstanceManager};
use crate::metrics::collector::{MetricPublisher, RdsMetricCollector};
use crate::metrics::prometheus_publisher::PrometheusPublisher;
use crate::config::Settings;

mod aws;
mod config;
mod metrics;

async fn serve_metrics(publisher: PrometheusPublisher) -> Result<impl warp::Reply, Infallible> {
    let metrics = publisher.gather();
    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    encoder.encode(&metrics, &mut buffer).unwrap();
    Ok(String::from_utf8(buffer).unwrap())
}

async fn serve_health() -> Result<impl warp::Reply, Infallible> {
    Ok("OK")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 로깅 설정
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));  // 기본 레벨은 info

    FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .with_thread_names(true)
        .with_ansi(true)
        .pretty()
        .init();

    info!("RDS 메트릭 수집기 시작...");

    // 설정 로드
    let config = Settings::new()?;
    info!("설정 로드 완료: {:?}", config);

    // AWS SDK 설정
    let mut aws_config_builder = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(config.aws.region.clone()));

    // AWS 프로필 설정이 있는 경우 적용
    if let Some(credentials) = &config.aws.credentials {
        aws_config_builder = aws_config_builder
            .profile_name(&credentials.profile)
            .credentials_provider(
                aws_config::profile::ProfileFileCredentialsProvider::builder()
                    .profile_name(&credentials.profile)
                    .build()
            );
    }

    let aws_config = aws_config_builder.load().await;

    // CloudWatch 수집기 설정
    let cw_config = CWConfig {
        period: config.cloudwatch.period,
        stat: config.cloudwatch.stat.clone(),
        retry_attempts: config.cloudwatch.retry_attempts,
        retry_delay: Duration::seconds(config.cloudwatch.retry_delay as i64),
    };

    // RDS 매니저 설정
    let rds_config = RdsConfig {
        target_tag_key: config.target.tag_key,
        target_tag_value: config.target.tag_value,
        ..Default::default()
    };

    // AWS 클라이언트 초기화
    let rds_client = RdsClient::new(&aws_config);
    let cloudwatch_client = CloudWatchClient::new(&aws_config);

    // 컴포넌트 초기화
    let rds_manager = RdsInstanceManager::new(rds_client, rds_config);
    let cloudwatch = CloudWatchCollector::new(cloudwatch_client, cw_config);
    let prometheus_publisher = PrometheusPublisher::new();
    let publishers: Vec<Box<dyn MetricPublisher>> = vec![Box::new(prometheus_publisher.clone())];

    // 메트릭 수집기 초기화
    let mut collector = RdsMetricCollector::new(
        cloudwatch,
        rds_manager,
        publishers,
        Duration::seconds(config.exporter.collection_interval as i64),
    );

    // Prometheus 메트릭 엔드포인트 설정
    let prometheus_publisher = warp::any().map(move || prometheus_publisher.clone());

    let metrics_route = warp::path("metrics")
        .and(warp::get())
        .and(prometheus_publisher.clone())
        .and_then(serve_metrics);

    let health_route = warp::path("health")
        .and(warp::get())
        .and_then(serve_health);

    let routes = metrics_route.or(health_route);

    // 서버 주소 설정
    let addr: SocketAddr = format!("{}:{}", config.exporter.host, config.exporter.port)
        .parse()
        .expect("Invalid address");

    info!(
        "메트릭 수집 시작 (수집 주기: {}초)",
        config.exporter.collection_interval
    );

    // 수집기와 HTTP 서버 동시 실행
    let server = warp::serve(routes).run(addr);
    let collector_handle = tokio::spawn(async move {
        if let Err(e) = collector.start_collection().await {
            error!("메트릭 수집 중 오류 발생: {}", e);
        }
    });

    // 서버와 수집기 실행
    tokio::select! {
        _ = server => {
            error!("HTTP 서버 종료");
        }
        _ = collector_handle => {
            error!("메트릭 수집기 종료");
        }
    }

    Ok(())
}