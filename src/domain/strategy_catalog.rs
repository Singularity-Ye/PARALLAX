//! Implemented research modules only. Order mirrors StrategyDraft validation.
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct RuleDescriptor {
    pub module: &'static str,
    pub name: &'static str,
    pub order: usize,
    pub requires: Vec<&'static str>,
    pub objectives: Vec<&'static str>,
    pub minimum_schema: u32,
    pub purpose: &'static str,
}
pub fn catalog() -> Vec<RuleDescriptor> {
    let wallet = vec!["wallet_conversion"];
    let both = vec!["wallet_conversion", "holding"];
    [
        (
            "strict_ratio",
            "钱包折算率",
            vec![],
            wallet.clone(),
            1,
            "现金成本与估算净Steam钱包的严格比例上限；不是现金折扣或收益",
        ),
        (
            "position_capacity",
            "仓位额度",
            vec![],
            both.clone(),
            1,
            "检查已引用仓位与可买额度；独立历史研究没有仓位/数量时为未知",
        ),
        (
            "history_coverage",
            "历史覆盖",
            vec![],
            both.clone(),
            1,
            "核对有效日、缺口、采集时效与最后有效日；持有观察必需",
        ),
        (
            "sample_cv",
            "样本CV",
            vec!["history_coverage"],
            both.clone(),
            1,
            "按原始或IQR清洗口径检查波动；清洗不消除原始尾部风险",
        ),
        (
            "trend7",
            "七日趋势",
            vec!["history_coverage"],
            both.clone(),
            1,
            "检查七自然日相对斜率、点数与R²；不预测未来价格",
        ),
        (
            "calendar_p10",
            "七日历史P10",
            vec!["history_coverage"],
            both,
            1,
            "按七自然日配对检查历史P10与滑点口径；不是未来损失上限",
        ),
        (
            "stress_exit_ratio",
            "压力净卖出折算率",
            vec!["history_coverage"],
            wallet,
            2,
            "显式盘口、历史压力、原币费用与FX下的净钱包估值；不执行出售",
        ),
    ]
    .into_iter()
    .enumerate()
    .map(
        |(order, (module, name, requires, objectives, minimum_schema, purpose))| RuleDescriptor {
            module,
            name,
            order,
            requires,
            objectives,
            minimum_schema,
            purpose,
        },
    )
    .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::strategy::{StrategyRule, tests::fixture};
    #[test]
    fn catalog_matches_supported_order_and_history_dependencies() {
        let rows = catalog();
        let (s, _, _, _) = fixture(10000 * 86400);
        assert_eq!(rows.len(), 7);
        for rule in s.draft.rules {
            let entry = rows.iter().find(|r| r.module == rule.key()).unwrap();
            if matches!(
                rule,
                StrategyRule::SampleCv { .. }
                    | StrategyRule::Trend7 { .. }
                    | StrategyRule::CalendarP10 { .. }
            ) {
                assert_eq!(entry.requires, vec!["history_coverage"]);
            }
        }
        assert_eq!(
            rows.iter().map(|r| r.order).collect::<Vec<_>>(),
            (0..7).collect::<Vec<_>>()
        );
        assert_eq!(rows[6].minimum_schema, 2);
        assert_eq!(rows[0].objectives, vec!["wallet_conversion"]);
    }
}
