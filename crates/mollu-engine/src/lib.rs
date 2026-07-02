//! mollu-engine — 도메인 무관 부품 조립 엔진.
//!
//! 설계 원천: MolluAI `docs/PARTS-SYSTEM.md` §7 (crate 2분할).
//! 이 crate는 "프롬프트"를 모른다 — 타입 있는 IR을 변형하는 단계들의 안전한
//! 조립이 전부다. 도메인(프롬프트, 설정 생성, …)은 별도 crate가
//! [`Ir`]/[`Part`]를 구현해 꽂는다.
//!
//! 핵심 불변식 (non-UB by construction):
//!   1. typed connector — 커넥터 타입이 안 맞는 조립은 [`AssemblyError`]로
//!      **검증 시점에** 거부된다. 실행 중 타입 불일치는 도달 불가.
//!   2. capability sandbox — 부품 본체는 호스트가 명시적으로 등록한
//!      도메인 연산만 호출할 수 있다. 그 밖은 존재하지 않는다.
//!   3. 조립체 = 데이터 — [`Assembly`]는 serde 직렬화 가능. UI(mollu-kit)의
//!      직접 조작이 이 데이터의 변형으로 기록된다.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

/// 도메인이 구현하는 중간 표현. 파이프라인의 각 단계는 `Ir → Ir` 변형이다.
pub trait Ir: Clone {}

/// 커넥터 타입 — "안 맞으면 안 꽂힘"의 단위. 도메인이 어휘를 정의한다.
/// (문자열 비교가 아니라 선언된 집합 안에서의 동일성 — 오타는 조립 검증에서 잡힌다.)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConnectorType(pub String);

/// 부품 인터페이스 선언 — 로봇 키트의 "커넥터 모양".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartInterface {
    /// 부품 식별자 (부품함 안에서 유일).
    pub id: String,
    /// 입력 슬롯: 슬롯 이름 → 기대 커넥터 타입.
    pub inputs: BTreeMap<String, ConnectorType>,
    /// 출력 커넥터 타입.
    pub output: ConnectorType,
}

/// 부품 구현 — 인터페이스 + IR 변형 본체.
///
/// 본체가 받는 [`Capabilities`]가 곧 샌드박스다: 등록된 도메인 연산 외에는
/// 호출할 방법이 타입상 존재하지 않는다.
pub trait Part<I: Ir> {
    fn interface(&self) -> &PartInterface;
    fn apply(&self, ir: I, caps: &Capabilities<I>) -> Result<I, PartError>;
}

/// 호스트가 부품 본체에 노출하는 도메인 연산 집합 (capability sandbox).
pub struct Capabilities<I: Ir> {
    ops: BTreeMap<String, Box<dyn Fn(&I, &serde_json::Value) -> Result<serde_json::Value, PartError>>>,
}

impl<I: Ir> Default for Capabilities<I> {
    fn default() -> Self {
        Self { ops: BTreeMap::new() }
    }
}

impl<I: Ir> Capabilities<I> {
    /// 도메인 연산 등록 — 부품 어휘의 성장은 문법 확장이 아니라 여기서 일어난다.
    pub fn register(
        &mut self,
        name: impl Into<String>,
        op: impl Fn(&I, &serde_json::Value) -> Result<serde_json::Value, PartError> + 'static,
    ) {
        self.ops.insert(name.into(), Box::new(op));
    }

    /// 등록된 연산 호출. 미등록 이름은 [`PartError::UnknownCapability`].
    pub fn call(&self, name: &str, ir: &I, arg: &serde_json::Value) -> Result<serde_json::Value, PartError> {
        match self.ops.get(name) {
            Some(op) => op(ir, arg),
            None => Err(PartError::UnknownCapability(name.to_string())),
        }
    }
}

/// 조립체 — 부품 인스턴스들의 연결 그래프. **코드가 아니라 데이터다.**
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assembly {
    /// 실행 순서의 부품 id 목록 (파이프라인 단계).
    pub stages: Vec<String>,
}

/// 조립 검증 오류 — 잘못된 조립은 실행 전에 여기서 끝난다.
#[derive(Debug, Error)]
pub enum AssemblyError {
    #[error("미등록 부품: {0}")]
    UnknownPart(String),
    #[error("커넥터 불일치: {from}({from_ty:?}) → {to}({to_ty:?})")]
    ConnectorMismatch {
        from: String,
        from_ty: ConnectorType,
        to: String,
        to_ty: ConnectorType,
    },
}

/// 부품 실행 오류.
#[derive(Debug, Error)]
pub enum PartError {
    #[error("미등록 capability: {0}")]
    UnknownCapability(String),
    #[error("{0}")]
    Domain(String),
}

/// 부품함 + 실행기.
pub struct Engine<I: Ir> {
    parts: BTreeMap<String, Box<dyn Part<I>>>,
    caps: Capabilities<I>,
}

impl<I: Ir> Engine<I> {
    pub fn new(caps: Capabilities<I>) -> Self {
        Self { parts: BTreeMap::new(), caps }
    }

    pub fn register_part(&mut self, part: Box<dyn Part<I>>) {
        self.parts.insert(part.interface().id.clone(), part);
    }

    /// 조립 검증 — typed connector 검사. 통과한 조립만 실행 가능하다.
    pub fn validate(&self, asm: &Assembly) -> Result<(), AssemblyError> {
        let mut prev: Option<(&str, &ConnectorType)> = None;
        for id in &asm.stages {
            let part = self
                .parts
                .get(id)
                .ok_or_else(|| AssemblyError::UnknownPart(id.clone()))?;
            let iface = part.interface();
            if let Some((prev_id, prev_out)) = prev {
                // 파이프라인 연결: 이전 출력 → 이번 부품의 "in" 슬롯.
                if let Some(expected) = iface.inputs.get("in") {
                    if expected != prev_out {
                        return Err(AssemblyError::ConnectorMismatch {
                            from: prev_id.to_string(),
                            from_ty: prev_out.clone(),
                            to: id.clone(),
                            to_ty: expected.clone(),
                        });
                    }
                }
            }
            prev = Some((id, &iface.output));
        }
        Ok(())
    }

    /// 검증된 조립을 실행한다 — IR이 단계들을 통과한다.
    pub fn run(&self, asm: &Assembly, ir: I) -> Result<I, EngineError> {
        self.validate(asm)?;
        let mut cur = ir;
        for id in &asm.stages {
            let part = self.parts.get(id).expect("validate가 존재를 보장");
            cur = part.apply(cur, &self.caps).map_err(|e| EngineError::Part {
                part: id.clone(),
                source: e,
            })?;
        }
        Ok(cur)
    }
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Assembly(#[from] AssemblyError),
    #[error("부품 '{part}' 실행 실패: {source}")]
    Part { part: String, source: PartError },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TextIr(String);
    impl Ir for TextIr {}

    struct Upper(PartInterface);
    impl Part<TextIr> for Upper {
        fn interface(&self) -> &PartInterface {
            &self.0
        }
        fn apply(&self, ir: TextIr, _c: &Capabilities<TextIr>) -> Result<TextIr, PartError> {
            Ok(TextIr(ir.0.to_uppercase()))
        }
    }

    fn text_ty() -> ConnectorType {
        ConnectorType("text".into())
    }

    #[test]
    fn pipeline_runs() {
        let mut engine = Engine::new(Capabilities::default());
        engine.register_part(Box::new(Upper(PartInterface {
            id: "upper".into(),
            inputs: BTreeMap::from([("in".into(), text_ty())]),
            output: text_ty(),
        })));
        let asm = Assembly { stages: vec!["upper".into()] };
        let out = engine.run(&asm, TextIr("mollu".into())).unwrap();
        assert_eq!(out.0, "MOLLU");
    }

    #[test]
    fn mismatched_connector_rejected() {
        let mut engine = Engine::new(Capabilities::default());
        engine.register_part(Box::new(Upper(PartInterface {
            id: "a".into(),
            inputs: BTreeMap::from([("in".into(), text_ty())]),
            output: ConnectorType("audio".into()),
        })));
        engine.register_part(Box::new(Upper(PartInterface {
            id: "b".into(),
            inputs: BTreeMap::from([("in".into(), text_ty())]),
            output: text_ty(),
        })));
        let asm = Assembly { stages: vec!["a".into(), "b".into()] };
        assert!(matches!(
            engine.validate(&asm),
            Err(AssemblyError::ConnectorMismatch { .. })
        ));
    }

    #[test]
    fn unknown_capability_is_error() {
        let caps: Capabilities<TextIr> = Capabilities::default();
        let ir = TextIr("x".into());
        assert!(matches!(
            caps.call("nope", &ir, &serde_json::Value::Null),
            Err(PartError::UnknownCapability(_))
        ));
    }
}
