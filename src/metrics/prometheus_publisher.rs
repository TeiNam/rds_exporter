// src/metrics/prometheus_publisher.rs
use crate::metrics::collector::{MetricPoint, MetricPublisher};
use async_trait::async_trait;
use lazy_static::lazy_static;
use parking_lot::RwLock;
use prometheus::{GaugeVec, Opts, Registry};
use std::collections::HashMap;
use tracing::{debug, warn};

lazy_static! {
    static ref REGISTRY: Registry = Registry::new();
    static ref METRICS: RwLock<HashMap<String, GaugeVec>> = RwLock::new(HashMap::new());
}

#[derive(Clone)]
pub struct PrometheusPublisher {}

impl PrometheusPublisher {
    pub fn new() -> Self {
        Self {}
    }

    fn get_or_create_metric(&self, name: &str, help: &str, label_names: &[&str]) -> anyhow::Result<GaugeVec> {
        let mut metrics = METRICS.write();

        if let Some(gauge) = metrics.get(name) {
            return Ok(gauge.clone());
        }

        let opts = Opts::new(name, help);
        let gauge = GaugeVec::new(opts, label_names)?;

        if let Err(e) = REGISTRY.register(Box::new(gauge.clone())) {
            warn!("메트릭 등록 실패 ({}): {}", name, e);
            return Err(anyhow::anyhow!("메트릭 등록 실패: {}", e));
        }

        metrics.insert(name.to_string(), gauge.clone());
        Ok(gauge)
    }

    fn create_metric_name(&self, metric: &MetricPoint) -> String {
        format!("rds_{}", metric.metric_name.to_lowercase())
    }
}

#[async_trait]
impl MetricPublisher for PrometheusPublisher {
    async fn publish(&self, metrics: Vec<MetricPoint>) -> anyhow::Result<()> {
        debug!("Prometheus 메트릭 발행 시작: {} 개", metrics.len());

        for metric in metrics {
            let metric_name = self.create_metric_name(&metric);
            let help = format!("RDS metric: {}", metric_name);

            let label_names: Vec<&str> = metric.additional_tags
                .keys()
                .map(|s| s.as_str())
                .collect();

            debug!(
                "메트릭 처리: {} (값: {}, 레이블: {:?})",
                metric_name, metric.value, label_names
            );

            match self.get_or_create_metric(&metric_name, &help, &label_names) {
                Ok(gauge) => {
                    let label_values: Vec<&str> = label_names
                        .iter()
                        .map(|&name| metric.additional_tags.get(name).map(|s| s.as_str()).unwrap_or(""))
                        .collect();

                    let metric_gauge = gauge.with_label_values(&label_values);
                    metric_gauge.set(metric.value);
                    debug!(
                        "메트릭 설정 완료: {}{{{}}} = {}",
                        metric_name,
                        label_names.iter().zip(label_values.iter())
                            .map(|(k, v)| format!("{}=\"{}\"", k, v))
                            .collect::<Vec<_>>()
                            .join(","),
                        metric.value
                    );
                }
                Err(e) => {
                    warn!("메트릭 생성 실패 ({}): {}", metric_name, e);
                    continue;
                }
            }
        }

        debug!("Prometheus 메트릭 발행 완료");
        Ok(())
    }

    fn gather(&self) -> Vec<prometheus::proto::MetricFamily> {
        REGISTRY.gather()
    }
}