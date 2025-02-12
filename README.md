
# RDS Exporter

AWS RDS 인스턴스의 CloudWatch 메트릭을 수집하여 Prometheus 형식으로 제공하는 exporter입니다.

## 기능

- AWS RDS 인스턴스 자동 검색 및 모니터링
- CloudWatch 메트릭 수집
- Prometheus 호환 메트릭 엔드포인트 제공
- 태그 기반 RDS 인스턴스 필터링
- 설정 가능한 메트릭 수집 주기
- 상태 확인 엔드포인트

## 시작하기

### 필수 조건

- Rust 개발 환경
- AWS 자격 증명 설정
- AWS RDS 및 CloudWatch 접근 권한

### 설치

```bash
git clone [repository-url]
cd rds_exporter
cargo build --release
```

### 설정

환경 변수를 통해 설정할 수 있습니다:

| 환경 변수 | 설명 | 기본값 |
|-----------|------|--------|
| AWS_REGION | AWS 리전 | ap-northeast-2 |
| COLLECTION_INTERVAL | 메트릭 수집 주기 (초) | 60 |
| TARGET_TAG_KEY | 대상 RDS 인스턴스 태그 키 | env |
| TARGET_TAG_VALUE | 대상 RDS 인스턴스 태그 값 | prd |
| PROMETHEUS_HOST | 메트릭 서버 호스트 | 0.0.0.0 |
| PROMETHEUS_PORT | 메트릭 서버 포트 | 9043 |

### 실행

```bash
./target/release/rds_exporter
```

## API 엔드포인트

- `/metrics` - Prometheus 형식의 메트릭 제공
- `/health` - 상태 확인

## 메트릭

RDS 인스턴스의 다음과 같은 CloudWatch 메트릭을 수집합니다:
- CPU 사용률
- 메모리 사용량
- 디스크 I/O
- 네트워크 트래픽
- 데이터베이스 연결 수
등

## 개발

```bash
# 테스트 실행
cargo test

# 개발 모드 실행
cargo run
```

## 라이센스

This project is licensed under the MIT License - see the LICENSE file for details

## 기여하기

1. Fork the Project
2. Create your Feature Branch
3. Commit your Changes
4. Push to the Branch
5. Open a Pull Request
