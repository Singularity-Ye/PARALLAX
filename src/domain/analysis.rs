use crate::domain::selection::{Candidate, MarketItem, RuleResult, ScanRequest};

pub fn evaluate(item: MarketItem, request: &ScanRequest) -> Candidate {
    let (valuation, valuation_error) = match request.wallet_policy.estimate(item.steam_gross_cents)
    {
        Ok(value) => (value, None),
        Err(error) => (None, Some(error)),
    };
    let wallet = valuation.as_ref().map_or(0, |v| v.net.minor);
    // Round upward so a ratio just above the cap can never pass due to truncation.
    let ratio = super::money::ratio_bps(item.cash_cents, wallet);
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
                "{}；上限 {} bps；使用请求指定的费用估算，尚未实价复核",
                ratio.map_or_else(
                    || valuation_error.clone().unwrap_or_else(|| {
                        "缺少有效 Steam 净值；费用或价格不足以得到正净值".into()
                    }),
                    |r| format!("{r} bps")
                ),
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
        valuation,
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
    #[test]
    fn rounded_down_ratio_cannot_pass_a_strict_numeric_cap() {
        let mut request = ScanRequest {
            min_price_cents: 0,
            min_volume: 0,
            max_ratio_bps: 3333,
            ..ScanRequest::default()
        };
        let item = MarketItem {
            key: "x".into(),
            name: "x".into(),
            goods_id: Some(1),
            cash_cents: 1,
            steam_gross_cents: 4,
            volume: 0,
            observed_at: 100,
        };
        let candidate = evaluate(item.clone(), &request);
        assert_eq!(candidate.ratio_bps, Some(3334));
        assert!(!candidate.passed);
        request.max_ratio_bps = 3334;
        assert!(evaluate(item, &request).passed);
    }
}
