use crate::domain::selection::{Candidate, MarketItem, RuleResult, ScanRequest};

// Indicative legacy estimate only, not a platform fee settlement implementation.
pub fn estimated_wallet(gross_cents: u64) -> u64 {
    gross_cents.saturating_mul(100) / 115
}

pub fn evaluate(item: MarketItem, request: &ScanRequest) -> Candidate {
    let wallet = estimated_wallet(item.steam_gross_cents);
    let ratio = (wallet > 0).then(|| item.cash_cents.saturating_mul(10_000) / wallet);
    let name = item.name.to_lowercase();
    let excluded = request
        .exclude_keywords
        .iter()
        .filter(|s| !s.trim().is_empty())
        .find(|s| name.contains(&s.to_lowercase()));
    let checks = [
        (
            "关键词",
            excluded.is_none(),
            excluded.map_or("未命中排除词".into(), |s| format!("命中 {s}")),
        ),
        (
            "平台映射",
            item.goods_id.is_some_and(|id| id > 0),
            "需要有效 BUFF 商品 ID".into(),
        ),
        (
            "现金价格",
            item.cash_cents > 0
                && item.cash_cents >= request.min_price_cents
                && item.cash_cents <= request.max_price_cents,
            format!(
                "{} 分；允许 {}–{} 分",
                item.cash_cents, request.min_price_cents, request.max_price_cents
            ),
        ),
        (
            "来源成交量",
            item.volume >= request.min_volume,
            format!(
                "{}；最低 {}；统计周期沿用来源，未推定为日量",
                item.volume, request.min_volume
            ),
        ),
        (
            "估算折算率",
            ratio.is_some_and(|r| r <= request.max_ratio_bps),
            format!(
                "{}；上限 {} bps；使用榜单报价，尚未实价复核",
                ratio.map_or("缺少 Steam 价格".into(), |r| format!("{r} bps")),
                request.max_ratio_bps
            ),
        ),
    ];
    let rules: Vec<_> = checks
        .into_iter()
        .map(|(rule, passed, reason)| RuleResult {
            rule: rule.into(),
            passed,
            reason,
        })
        .collect();
    Candidate {
        passed: rules.iter().all(|r| r.passed),
        item,
        estimated_wallet_cents: wallet,
        ratio_bps: ratio,
        rules,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_quote_is_not_zero_cost_opportunity() {
        let item = MarketItem {
            key: "x".into(),
            name: "x".into(),
            goods_id: Some(1),
            cash_cents: 500,
            steam_gross_cents: 0,
            volume: 300,
            observed_at: 0,
        };
        let candidate = evaluate(item, &ScanRequest::default());
        assert!(!candidate.passed);
        assert_eq!(candidate.ratio_bps, None);
    }
}
