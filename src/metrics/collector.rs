// src/metrics/collector.rs
use crate::aws::cloudwatch::CloudWatchCollector;
use crate::aws::rds::RdsInstanceManager;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use std::collections::HashMap;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct MetricPoint {
    pub value: f64,
    pub metric_name: String,
    pub additional_tags: HashMap<String, String>,
}

#[async_trait]
pub trait MetricPublisher: Send + Sync {
    async fn publish(&self, metrics: Vec<MetricPoint>) -> anyhow::Result<()>;
    fn gather(&self) -> Vec<prometheus::proto::MetricFamily>;
}

pub struct RdsMetricCollector {
    cloudwatch: CloudWatchCollector,
    rds_manager: RdsInstanceManager,
    publishers: Vec<Box<dyn MetricPublisher>>,
    collection_interval: Duration,
}

impl RdsMetricCollector {
    pub fn new(
        cloudwatch: CloudWatchCollector,
        rds_manager: RdsInstanceManager,
        publishers: Vec<Box<dyn MetricPublisher>>,
        collection_interval: Duration,
    ) -> Self {
        Self {
            cloudwatch,
            rds_manager,
            publishers,
            collection_interval,
        }
    }

    pub async fn start_collection(&mut self) -> anyhow::Result<()> {
        loop {
            match self.collect_and_publish().await {
                Ok(_) => info!("메트릭 수집 및 발행 완료"),
                Err(e) => error!("메트릭 수집 중 오류 발생: {}", e),
            }
            tokio::time::sleep(std::time::Duration::from_secs(
                self.collection_interval.num_seconds() as u64,
            ))
            .await;
        }
    }

    async fn collect_and_publish(&mut self) -> anyhow::Result<()> {
        let instances = self.rds_manager.get_prd_instances().await?;
        let mut all_metrics = Vec::new();
        let end_time = Utc::now();
        let start_time = end_time - Duration::minutes(5);

        for instance in instances {
            let instance_id = instance.db_instance_identifier().unwrap_or_default();
            let engine = instance.engine().unwrap_or_default();
            let mut tags = self.get_instance_tags(&instance);
            // instance_id를 tags에 포함
            tags.insert("instance_id".to_string(), instance_id.to_string());

            let metrics_to_collect = match &engine[..] {
                "aurora-mysql" | "mysql" => self.get_mysql_metrics(),
                "aurora-postgresql" | "postgres" => self.get_postgresql_metrics(),
                _ => self.get_common_metrics(),
            };

            let metric_tuples: Vec<(&str, &str, &str, &str)> = metrics_to_collect
                .iter()
                .map(|metric_name| (
                    "AWS/RDS",
                    metric_name.as_str(),
                    "DBInstanceIdentifier",
                    instance_id,
                ))
                .collect();

            match self.cloudwatch
                .collect_all_metrics(metric_tuples, start_time, end_time)
                .await
            {
                Ok(response) => {
                    for (idx, data) in response.metric_data_results().iter().enumerate() {
                        let metric_name = &metrics_to_collect[idx];

                        for value in data.values() {
                            let metric = MetricPoint {
                                value: *value,
                                metric_name: metric_name.clone(),
                                additional_tags: tags.clone(),
                            };
                            all_metrics.push(metric);
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "메트릭 수집 실패 (인스턴스: {}): {}",
                        instance_id, e
                    );
                    continue;
                }
            }
        }

        for publisher in &self.publishers {
            if let Err(e) = publisher.publish(all_metrics.clone()).await {
                error!("메트릭 발행 실패: {}", e);
            }
        }

        Ok(())
    }

    fn get_common_metrics(&self) -> Vec<String> {
        vec![
            "CPUUtilization".to_string(),
            "FreeableMemory".to_string(),
            "FreeStorageSpace".to_string(),
            "DatabaseConnections".to_string(),
            "ReadIOPS".to_string(),
            "WriteIOPS".to_string(),
            "ReadLatency".to_string(),
            "WriteLatency".to_string(),
        ]
    }

    fn get_mysql_metrics(&self) -> Vec<String> {
        let mut metrics = self.get_common_metrics();
        metrics.extend(vec![
            "Queries".to_string(),
            "ThreadsRunning".to_string(),
            "InnodbBufferPoolHits".to_string(),
            "DeadlocksCount".to_string(),
        ]);
        metrics
    }

    fn get_postgresql_metrics(&self) -> Vec<String> {
        let mut metrics = self.get_common_metrics();
        metrics.extend(vec![
            "ActiveTransactions".to_string(),
            "BufferCacheHitRatio".to_string(),
            "IndexHitRatio".to_string(),
            "Deadlocks".to_string(),
        ]);
        metrics
    }

    fn get_instance_tags(&self, instance: &aws_sdk_rds::types::DbInstance) -> HashMap<String, String> {
        let mut tags = HashMap::new();

        if let Some(id) = instance.db_instance_identifier() {
            tags.insert("db_instance_identifier".to_string(), id.to_string());
        }
        if let Some(engine) = instance.engine() {
            tags.insert("engine".to_string(), engine.to_string());
        }
        if let Some(version) = instance.engine_version() {
            tags.insert("engine_version".to_string(), version.to_string());
        }
        if let Some(class) = instance.db_instance_class() {
            tags.insert("class".to_string(), class.to_string());
        }
        if let Some(az) = instance.availability_zone() {
            tags.insert("availability_zone".to_string(), az.to_string());
        }

        tags
    }
}