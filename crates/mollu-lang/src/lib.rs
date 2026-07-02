//! mollu-lang — mollu 언어 (부품 선언·조립·본체의 단일 문법).
//!
//! 상태: 문법 스케치 전 골격. LALRPOP 문법은 `PARTS-SYSTEM.md` §10의
//! "전용 문법 구체 설계"가 확정되면 build-dependency로 활성화한다.
//!
//! 확정된 설계 제약 (MolluAI docs/PARTS-SYSTEM.md §6):
//!   - 문법은 작게 고정, 성장은 도메인 함수(capability) 추가로.
//!   - 위험 구문은 문법에 존재하지 않는다 — 샌드박스가 문법 수준에서 성립.
//!   - 패턴 매칭은 rust regex (RE2 계열) — ReDoS 구조적 불가능.
//!   - lexer는 "생 텍스트 + {{ }} 섬" 모드 전환 필요 (커스텀 lexer).

/// 패턴 센서용 regex 컴파일 — RE2 계열이라 어떤 유저 패턴도 선형 시간.
/// lookaround/backreference는 없다 (의도된 교환 — PARTS-SYSTEM.md §6).
pub fn compile_pattern(pattern: &str) -> Result<regex::Regex, regex::Error> {
    regex::Regex::new(pattern)
}

#[cfg(test)]
mod tests {
    #[test]
    fn redos_pattern_is_still_linear() {
        // 백트래킹 엔진이면 파국적인 패턴 — rust regex에선 컴파일되고 선형이다.
        let re = super::compile_pattern("(a+)+$").unwrap();
        let s = "a".repeat(10_000) + "b";
        assert!(!re.is_match(&s));
    }

    #[test]
    fn backreference_is_unrepresentable() {
        // 백레퍼런스는 문법에 없다 — 컴파일 자체가 거부된다 (구조적 제거의 witness).
        assert!(super::compile_pattern(r"(a)\1").is_err());
    }
}
