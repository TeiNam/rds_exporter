use aws_sdk_rds::types::{DbInstance, Tag};
use aws_sdk_rds::Client;
use aws_smithy_runtime_api::client::result::SdkError;
use aws_smithy_runtime_api::http::Response;
use std::collections::HashMap;
use std::time::SystemTime;
use thiserror::Error;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

#[derive(Error, Debug)]
pub enum RdsError {
    #[error("AWS RDS API 에러: {0}")]
    DescribeInstancesError(String),

    #[error("태그 조회 실패: {0}")]
    TagLookupError(String),

    #[error("재시도 횟수 초과: {0}")]
    RetryExhausted(String),
}

impl<E> From<SdkError<E, Response>> for RdsError
where
    E: std::fmt::Debug,
{
    fn from(err: SdkError<E, Response>) -> Self {
        match err {
            SdkError::ServiceError(service_err) => {
                RdsError::DescribeInstancesError(format!("Service error: {:?}", service_err))
            }
            SdkError::TimeoutError(timeout_err) => {
                RdsError::DescribeInstancesError(format!("Timeout error: {:?}", timeout_err))
            }
            _ => RdsError::DescribeInstancesError(format!("AWS SDK error: {:?}", err)),
        }
    }
}

pub type Result<T> = std::result::Result<T, RdsError>;

#[derive(Debug, Clone)]
pub struct RdsConfig {
    pub max_retries: u32,
    pub retry_delay: Duration,
    pub cache_ttl: Duration,
    pub page_size: i32,
    pub target_tag_key: String,
    pub target_tag_value: String,
}

impl Default for RdsConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
            cache_ttl: Duration::from_secs(300),
            page_size: 100,
            target_tag_key: "env".to_string(),
            target_tag_value: "prd".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TagFilter {
    pub key: String,
    pub value: String,
}

impl TagFilter {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    fn matches(&self, tags: &[Tag]) -> bool {
        tags.iter().any(|tag| {
            tag.key()
                .map_or(false, |k| k.eq_ignore_ascii_case(&self.key))
                && tag
                    .value()
                    .map_or(false, |v| v.eq_ignore_ascii_case(&self.value))
        })
    }
}

#[derive(Debug)]
struct CacheEntry {
    instances: Vec<DbInstance>,
    timestamp: SystemTime,
}

pub struct RdsInstanceManager {
    client: Client,
    config: RdsConfig,
    cache: HashMap<Vec<TagFilter>, CacheEntry>,
}

impl RdsInstanceManager {
    pub fn new(client: Client, config: RdsConfig) -> Self {
        Self {
            client,
            config,
            cache: HashMap::new(),
        }
    }

    pub async fn get_prd_instances(&mut self) -> Result<Vec<DbInstance>> {
        let filters = vec![TagFilter::new(
            self.config.target_tag_key.clone(),
            self.config.target_tag_value.clone(),
        )];
        self.get_instances_by_tags(filters).await
    }

    /// 특정 태그를 가진 RDS 인스턴스들을 조회합니다.
    pub async fn get_instances_by_tags(
        &mut self,
        filters: Vec<TagFilter>,
    ) -> Result<Vec<DbInstance>> {
        if let Some(entry) = self.cache.get(&filters) {
            match entry.timestamp.elapsed() {
                Ok(elapsed) if elapsed < self.config.cache_ttl => {
                    debug!("캐시된 인스턴스 정보 반환");
                    return Ok(entry.instances.clone());
                }
                Ok(_) => debug!("캐시 만료"),
                Err(e) => warn!("캐시 타임스탬프 확인 실패: {}", e),
            }
        }

        let instances = self.fetch_filtered_instances(filters.clone()).await?;

        // 캐시 업데이트
        self.cache.insert(
            filters,
            CacheEntry {
                instances: instances.clone(),
                timestamp: SystemTime::now(),
            },
        );

        Ok(instances)
    }

    async fn fetch_filtered_instances(&self, filters: Vec<TagFilter>) -> Result<Vec<DbInstance>> {
        let mut filtered_instances = Vec::new();
        let mut next_token = None;

        loop {
            let mut req = self.client.describe_db_instances();

            if let Some(token) = next_token.as_ref() {
                req = req.marker(token);
            }

            req = req.max_records(self.config.page_size);

            let response = self
                .call_with_retry(|| async { req.clone().send().await.map_err(RdsError::from) })
                .await?;

            for instance in response.db_instances() {
                if let Some(arn) = instance.db_instance_arn() {
                    match self.get_instance_tags(arn).await {
                        Ok(tags) => {
                            if filters.iter().all(|filter| filter.matches(&tags)) {
                                filtered_instances.push(instance.clone());
                            }
                        }
                        Err(e) => {
                            warn!("인스턴스 {} 태그 조회 실패: {:?}", arn, e);
                            continue;
                        }
                    }
                }
            }

            next_token = response.marker().map(String::from);
            if next_token.is_none() {
                break;
            }
        }

        Ok(filtered_instances)
    }

    async fn get_instance_tags(&self, arn: &str) -> Result<Vec<Tag>> {
        let response = self
            .call_with_retry(|| async {
                self.client
                    .list_tags_for_resource()
                    .resource_name(arn)
                    .send()
                    .await
                    .map_err(|e| {
                        RdsError::TagLookupError(format!("태그 조회 중 오류 발생: {:?}", e))
                    })
            })
            .await?;

        Ok(response.tag_list().to_vec())
    }

    async fn call_with_retry<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempts = 0;
        let mut last_error = None;
        let mut delay = self.config.retry_delay;

        while attempts < self.config.max_retries {
            match f().await {
                Ok(response) => {
                    if attempts > 0 {
                        info!("재시도 성공 (시도 횟수: {})", attempts + 1);
                    }
                    return Ok(response);
                }
                Err(e) => {
                    warn!("API 호출 실패 (시도 횟수: {}): {:?}", attempts + 1, e);
                    last_error = Some(e);
                    attempts += 1;

                    if attempts < self.config.max_retries {
                        sleep(delay).await;
                        delay *= 2;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            RdsError::RetryExhausted("최대 재시도 횟수를 초과했습니다".to_string())
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_sdk_rds::config::Config;
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
    async fn test_tag_filter_matching() {
        let filter = TagFilter::new("env", "prd");
        let tags = vec![Tag::builder().key("env").value("prd").build()];

        assert!(filter.matches(&tags));
    }

    #[tokio::test]
    async fn test_multiple_tag_filters() {
        let mut manager = RdsInstanceManager::new(create_test_client(), RdsConfig::default());
        let filters = vec![TagFilter::new("env", "prd")];
        let result = manager.get_instances_by_tags(filters).await;
        assert!(result.is_ok());
    }
}
