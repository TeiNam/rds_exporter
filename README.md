📊 RDS Exporter

![Rust](https://img.shields.io/badge/Rust-1.74+-orange.svg)

AWS RDS 인스턴스의 CloudWatch 메트릭을 수집하여 Prometheus 형식으로 제공하는 Rust 기반 exporter입니다.
✨ 주요 기능

AWS RDS 인스턴스 자동 탐색: 태그 기반으로 모니터링할 RDS 인스턴스 필터링
포괄적인 메트릭 수집: CPU, 메모리, 디스크, 네트워크, 데이터베이스 특화 메트릭 등 다양한 메트릭 수집
DB 엔진별 특화 메트릭: MySQL, PostgreSQL 등 각 데이터베이스 엔진에 최적화된 메트릭 수집
Prometheus 호환 엔드포인트: 표준 Prometheus 형식의 메트릭 제공
유연한 설정: 환경 변수, 설정 파일, 실행 모드(development/production)를 통한 세밀한 설정
강화된 오류 처리: 재시도 메커니즘, 캐싱, 타임아웃 처리
상태 확인 엔드포인트: 애플리케이션 헬스 체크 지원

🖥️ 시스템 요구사항

Rust 개발 환경 (1.74.0 이상 권장)
AWS 계정 및 적절한 권한을 가진 IAM 사용자 또는 역할
RDS 인스턴스에 접근 가능한 네트워크 환경

📥 설치 방법
소스에서 빌드
bash복사# 저장소 클론
git clone https://github.com/yourusername/rds_exporter.git
cd rds_exporter

# 릴리스 빌드
cargo build --release

# 실행 파일은 target/release/rds_exporter에 생성됩니다
Docker로 실행 (Dockerfile 예시)
dockerfile복사FROM rust:1.74-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/rds_exporter /usr/local/bin/
COPY --from=builder /app/config /etc/rds_exporter/config
EXPOSE 9043
CMD ["rds_exporter"]
⚙️ 설정 방법
RDS Exporter는 계층적 설정 시스템을 사용합니다:

기본 설정 (config/default.yaml)
환경별 설정 (config/[development|production].yaml)
환경 변수 (최우선 적용)

설정 파일 구조
yaml복사aws:
  region: ap-northeast-2
  credentials:
    profile: your-sso-profile  # 선택 사항

exporter:
  host: "0.0.0.0"
  port: 9043
  collection_interval: 60  # 초 단위

target:
  tag_key: "env"
  tag_value: "prd"

cloudwatch:
  period: 60  # 초 단위
  stat: "Average"
  retry_attempts: 3
  retry_delay: 1  # 초 단위
환경 변수
환경 변수설명기본값RUN_MODE실행 모드 (development/production)developmentAPP_AWS_REGIONAWS 리전ap-northeast-2APP_AWS_CREDENTIALS_PROFILEAWS 프로필-APP_EXPORTER_HOST메트릭 서버 호스트0.0.0.0APP_EXPORTER_PORT메트릭 서버 포트9043APP_EXPORTER_COLLECTION_INTERVAL메트릭 수집 주기 (초)60APP_TARGET_TAG_KEY대상 RDS 인스턴스 태그 키envAPP_TARGET_TAG_VALUE대상 RDS 인스턴스 태그 값prdAPP_CLOUDWATCH_PERIODCloudWatch 메트릭 기간 (초)60APP_CLOUDWATCH_STATCloudWatch 통계 (Average, Sum 등)AverageAPP_CLOUDWATCH_RETRY_ATTEMPTS재시도 횟수3APP_CLOUDWATCH_RETRY_DELAY재시도 지연 시간 (초)1
🔐 인증 방식
로컬 개발 환경
1. 환경 변수 사용
bash복사export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_REGION=ap-northeast-2
2. AWS CLI 프로필 사용
bash복사# ~/.aws/credentials 파일 설정
[default]
aws_access_key_id = your_access_key
aws_secret_access_key = your_secret_key

# ~/.aws/config 파일 설정
[default]
region = ap-northeast-2
3. AWS SSO 사용 시
bash복사# AWS SSO 로그인
aws sso login --profile your-profile-name

# 프로필 지정하여 실행
RUN_MODE=development AWS_PROFILE=your-profile-name cargo run
프로덕션 환경

ECS/EKS의 경우: Task/Pod에 적절한 IAM Role 할당
EC2의 경우: 인스턴스에 IAM Role 할당
온프레미스의 경우: IAM User 자격 증명 사용

👮 필요한 IAM 권한
json복사{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Action": [
                "rds:DescribeDBInstances",
                "rds:ListTagsForResource",
                "cloudwatch:GetMetricData",
                "cloudwatch:GetMetricStatistics"
            ],
            "Resource": "*"
        }
    ]
}
🚀 실행 방법
개발 환경에서 실행
bash복사# 개발 모드로 실행
RUN_MODE=development cargo run

# 특정 설정 오버라이드
APP_EXPORTER_PORT=9044 APP_AWS_REGION=ap-northeast-1 cargo run
프로덕션 환경에서 실행
bash복사# 프로덕션 모드로 실행
RUN_MODE=production ./target/release/rds_exporter

# 또는 환경 변수 지정
RUN_MODE=production APP_TARGET_TAG_VALUE=prod ./target/release/rds_exporter
🌐 API 엔드포인트

/metrics: Prometheus 형식의 메트릭 제공
/health: 애플리케이션 상태 확인 (200 OK 응답 = 정상)

📈 수집되는 메트릭
공통 메트릭

rds_cpuutilization: CPU 사용률 (%)
rds_freeablememory: 사용 가능한 메모리 (바이트)
rds_freestoragespace: 사용 가능한 스토리지 공간 (바이트)
rds_databaseconnections: 활성 데이터베이스 연결 수
rds_readiops: 초당 읽기 I/O 작업 수
rds_writeiops: 초당 쓰기 I/O 작업 수
rds_readlatency: 읽기 지연 시간 (초)
rds_writelatency: 쓰기 지연 시간 (초)
rds_diskqueuedepth: 디스크 대기열 깊이
rds_readthroughput: 읽기 처리량 (바이트/초)
rds_writethroughput: 쓰기 처리량 (바이트/초)
rds_networkreceivethouthput: 수신 네트워크 처리량 (바이트/초)
rds_networktransmitthroughput: 송신 네트워크 처리량 (바이트/초)

MySQL/Aurora MySQL 특화 메트릭

rds_queries: 초당 쿼리 수
rds_threadsrunning: 실행 중인 스레드 수
rds_innodbpoolhits: InnoDB 버퍼 풀 히트 수
rds_innodbpoolreadrequests: InnoDB 버퍼 풀 읽기 요청 수
rds_innodbpoolreads: 실제 InnoDB 읽기 수
rds_deadlockscount: 데드락 발생 수

PostgreSQL/Aurora PostgreSQL 특화 메트릭

rds_activetransactions: 활성 트랜잭션 수
rds_buffercachehitratio: 버퍼 캐시 히트율 (%)
rds_indexhitratio: 인덱스 히트율 (%)
rds_deadlocks: 데드락 발생 수
rds_temporarytables: 임시 테이블 사용률
rds_replicationlag: 복제 지연 시간 (초)
rds_checkpointduration: 체크포인트 소요 시간 (초)
rds_walwritelatency: WAL 쓰기 지연 시간 (초)

🏗️ 아키텍처
RDS Exporter는 다음과 같은 주요 컴포넌트로 구성됩니다:

RdsInstanceManager: AWS RDS API를 사용하여 태그 기반으로 대상 RDS 인스턴스를 조회하고 관리
CloudWatchCollector: AWS CloudWatch API를 사용하여 RDS 인스턴스의 메트릭 데이터 수집
RdsMetricCollector: 수집 프로세스를 조정하고 메트릭 데이터를 발행자에게 전달
PrometheusPublisher: 수집된 메트릭을 Prometheus 형식으로 변환하여 제공
HTTP 서버: Prometheus와 상태 확인을 위한 엔드포인트 제공

📝 로깅
로깅 레벨은 환경 변수 RUST_LOG를 사용하여 설정할 수 있습니다:
bash복사# 디버그 수준 로깅 활성화
RUST_LOG=debug ./target/release/rds_exporter

# 특정 모듈에 대해서만 로깅 레벨 지정
RUST_LOG=rds_exporter=info,aws_sdk_cloudwatch=warn cargo run
⚡ 성능 고려사항

메모리 사용량: 캐싱을 통해 AWS API 호출을 최소화하므로 메모리 사용량이 증가할 수 있습니다.
API 호출 빈도: 수집 간격을 너무 짧게 설정하면 AWS API 제한에 도달할 수 있습니다.
CPU 사용량: 많은 수의 RDS 인스턴스를 모니터링할 경우 CPU 사용량이 증가할 수 있습니다.

🔧 문제 해결
일반적인 문제

AWS 인증 오류

AWS 자격 증명이 올바르게 설정되었는지 확인
IAM 사용자/역할에 필요한 권한이 있는지 확인


메트릭이 표시되지 않음

대상 태그 설정이 올바른지 확인
CloudWatch API에 접근 가능한지 확인
로그 레벨을 debug로 설정하여 상세 정보 확인


높은 지연 시간

수집 간격을 늘려보기
CloudWatch 재시도 설정 조정
메트릭 수를 줄이거나 필요한 메트릭만 수집하도록 코드 수정



로그 분석
bash복사# 오류 로그만 필터링
./target/release/rds_exporter 2>&1 | grep ERROR

# 특정 인스턴스 관련 로그 필터링
RUST_LOG=debug ./target/release/rds_exporter 2>&1 | grep "db-instance-id"
💻 개발 가이드
새로운 메트릭 추가
src/metrics/collector.rs 파일에서 get_common_metrics(), get_mysql_metrics() 또는 get_postgresql_metrics() 함수를 수정하여 새로운 메트릭을 추가할 수 있습니다.
새로운 게시자 추가
MetricPublisher 트레이트를 구현하는 새 구조체를 만들어 다른 형식(예: InfluxDB, Graphite)으로 메트릭을 내보낼 수 있습니다.
rust복사// 예시: InfluxDB 게시자
pub struct InfluxDBPublisher {
    client: InfluxDBClient,
}

#[async_trait]
impl MetricPublisher for InfluxDBPublisher {
    async fn publish(&self, metrics: Vec<MetricPoint>) -> anyhow::Result<()> {
        // InfluxDB로 메트릭 전송 구현
    }

    fn gather(&self) -> Vec<prometheus::proto::MetricFamily> {
        vec![] // Prometheus 전용 메서드이므로 빈 벡터 반환
    }
}
👥 참여하기

프로젝트를 Fork하기
기능 브랜치 생성 (git checkout -b feature/amazing-feature)
변경사항 커밋 (git commit -m 'Add amazing feature')
브랜치에 Push (git push origin feature/amazing-feature)
Pull Request 열기

📄 라이센스
이 프로젝트는 MIT 라이센스 하에 배포됩니다.