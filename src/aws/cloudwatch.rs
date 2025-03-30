// src/aws/cloudwatch.rs
use aws_sdk_cloudwatch::operation::get_metric_data::GetMetricDataOutput;
use aws_sdk_cloudwatch::types::{Dimension, Metric, MetricDataQuery, MetricStat};
use aws_sdk_cloudwatch::{Client, Error as AwsError};
use aws_smithy_types::DateTime as SmithyDateTime;
use chrono::{DateTime, Duration, Utc};
use thiserror::Error;
use tokio::time::sleep;
use tracing::{error, info, warn};

#[derive(Error, Debug)]
pub enum CloudWatchError {
    #[error("AWS API 에러: {0}")]
    AwsError(#[from] AwsError),

    #[error("잘못된 매개변수: {0}")]
    InvalidParameter(String),

    #[error("타임아웃: {0}")]
    Timeout(String),

    #[error("재시도 횟수 초과: {0}")]
    RetryExhausted(String),
}

pub type Result<T> = std::result::Result<T, CloudWatchError>;

#[derive(Debug, Clone)]
pub struct MetricConfig {
    pub period: i32,
    pub stat: String,
    pub retry_attempts: u32,
    pub retry_delay: Duration,
}

impl Default for MetricConfig {
    fn default() -> Self {
        Self {
            period: 60,
            stat: "Average".to_string(),
            retry_attempts: 3,
            retry_delay: Duration::seconds(1),
        }
    }
}

pub struct CloudWatchCollector {
    client: Client,
    config: MetricConfig,
}

impl CloudWatchCollector {
    pub fn new(client: Client, config: MetricConfig) -> Self {
        Self { client, config }
    }

    pub async fn collect_all_metrics(
        &mut self,
        metrics: Vec<(&str, &str, &str, &str)>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<GetMetricDataOutput> {
        let mut queries = Vec::with_capacity(metrics.len());

        for (idx, (namespace, metric_name, dimension_name, dimension_value)) in
            metrics.into_iter().enumerate()
        {
            let metric_stat =
                self.build_metric_stat(namespace, metric_name, dimension_name, dimension_value)?;
            let query = MetricDataQuery::builder()
                .id(format!("m{}", idx))
                .metric_stat(metric_stat)
                .return_data(true)
                .build();
            queries.push(query);
        }

        self.call_with_retry(start_time, end_time, queries).await
    }

    fn build_metric_stat(
        &self,
        namespace: &str,
        metric_name: &str,
        dimension_name: &str,
        dimension_value: &str,
    ) -> Result<MetricStat> {
        if namespace.is_empty() || metric_name.is_empty() {
            return Err(CloudWatchError::InvalidParameter(
                "namespace와 metric_name은 비어있을 수 없습니다.".to_string(),
            ));
        }

        Ok(MetricStat::builder()
            .metric(
                Metric::builder()
                    .namespace(namespace)
                    .metric_name(metric_name)
                    .dimensions(
                        Dimension::builder()
                            .name(dimension_name)
                            .value(dimension_value)
                            .build(),
                    )
                    .build(),
            )
            .period(self.config.period)
            .stat(&self.config.stat)
            .build())
    }

    async fn call_with_retry(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        queries: Vec<MetricDataQuery>,
    ) -> Result<GetMetricDataOutput> {
        let mut attempts = 0;
        let mut last_error_message = String::new();

        let start_smithy = SmithyDateTime::from_secs(start_time.timestamp());
        let end_smithy = SmithyDateTime::from_secs(end_time.timestamp());

        while attempts < self.config.retry_attempts {
            match tokio::time::timeout(
                std::time::Duration::from_secs(30),
                self.client
                    .get_metric_data()
                    .start_time(start_smithy)
                    .end_time(end_smithy)
                    .set_metric_data_queries(Some(queries.clone()))
                    .send(),
            )
            .await
            {
                Ok(result) => match result {
                    Ok(response) => {
                        if attempts > 0 {
                            info!("재시도 성공 (시도 횟수: {})", attempts + 1);
                        }
                        return Ok(response);
                    }
                    Err(err) => {
                        last_error_message = format!("{:?}", err);
                        warn!(
                            "API 호출 실패 (시도 횟수: {}): {}",
                            attempts + 1,
                            last_error_message
                        );
                    }
                },
                Err(_) => {
                    last_error_message = "API 호출 타임아웃".to_string();
                    warn!("API 호출 타임아웃 (시도 횟수: {})", attempts + 1);
                    return Err(CloudWatchError::Timeout(last_error_message));
                }
            }

            attempts += 1;
            if attempts < self.config.retry_attempts {
                sleep(self.config.retry_delay.to_std().unwrap()).await;
            }
        }

        Err(CloudWatchError::RetryExhausted(format!(
            "최대 재시도 횟수({})를 초과했습니다: {}",
            self.config.retry_attempts, last_error_message
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_cloudwatch::config::Config;
    use aws_smithy_types::test_connection::TestConnection;

    fn create_test_client() -> Client {
        let conn = TestConnection::new(Vec::new());
        let conf = Config::builder()
            .behavior_version_latest()
            .connection_provider(conn)
            .build();
        Client::from_conf(conf)
    }

    #[tokio::test]
    async fn test_invalid_parameters() {
        let client = create_test_client();
        let config = MetricConfig::default();
        let mut collector = CloudWatchCollector::new(client, config);

        let result = collector
            .collect_all_metrics(
                vec![(
                    "",
                    "CPUUtilization",
                    "DBInstanceIdentifier",
                    "test-instance",
                )],
                Utc::now() - Duration::minutes(5),
                Utc::now(),
            )
            .await;

        assert!(result.is_err());
        match result {
            Err(CloudWatchError::InvalidParameter(_)) => (),
            _ => panic!("잘못된 에러 타입"),
        }
    }
}
