# mollu

[![License: Custom](https://img.shields.io/badge/License-Custom-blue.svg)](./LICENSE)

**capability-sandboxed, block-based composition engine** — 부품 조립 키트 방식의
IR 파이프라인 엔진과, 부품을 기술하는 전용 언어.

> 이름의 유래: 이 엔진의 요점은 UB("동작을 몰루")를 에러 처리가 아니라
> 문법·타입에서 제거하는 것이다.

| crate | 역할 |
|---|---|
| `mollu-engine` | 도메인 무관 — typed connector 그래프 + capability 샌드박스 + IR 파이프라인 실행기 |
| `mollu-lang` | mollu 언어 — 부품 선언·조립·본체의 단일 문법 (LALRPOP, 문법 스케치 진행 중) |
| `mollu-kit` | (예정) 직접조작 UI — 노드 캔버스가 아니라 문서형, 모바일 터치 퍼스트 |

첫 도메인: [MolluAI](https://github.com/shittim-plana/molluai)의
프롬프트 파이프라인 (`molluai-parts`, 예정).

설계 문서: [DESIGN.md](DESIGN.md)

## 원칙

1. **witness가 곧 테스트다** — 불변식(커넥터 불일치 거부, ReDoS 불가, capability
   밖 호출 불가)마다 그것을 증명하는 테스트가 있다.
2. **문법은 작게 고정, 어휘로 성장** — 표현력이 부족하면 문법 확장이 아니라
   도메인 함수 추가로 푼다.
3. **조립체 = 데이터** — UI의 직접 조작은 코드 생성이 아니라 데이터 변형이다.

## License

[Custom License](./LICENSE) © 2026 shittim-plana  
Commercial use requires prior permission. See [LICENSE](./LICENSE) for details.
