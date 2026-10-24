//! Composes existing wall/gap, historical return, native fees and FX primitives.
use super::{
    fx::{self, FxSnapshot},
    money::{Currency, FeeQuote, FeeSchedule, Money},
    orderbook::OrderBook,
    pricing::{ListingPriceAnalysis, WallGapPolicy, wall_gap_price_with_rate},
    risk::{SevenDayRisk, stress_price},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitValuationPolicy {
    pub pricing: WallGapPolicy,
    pub fees: FeeSchedule,
    pub min_pairs: u32,
    pub slippage_bps: u32,
    pub max_book_age_secs: u64,
}
impl ExitValuationPolicy {
    pub fn validate(&self) -> Result<(), String> {
        self.pricing.validate()?;
        self.fees.validate()?;
        if !fx::supported(&self.fees.currency)
            || self.fees.platform_min_minor > 1_000_000_000_000
            || self.fees.publisher_min_minor > 1_000_000_000_000
            || self.fees.increment_minor > 1_000_000_000_000
            || !(1..=358).contains(&self.min_pairs)
            || self.slippage_bps > 10000
            || !(1..=3600).contains(&self.max_book_age_secs)
        {
            return Err(
                "出售估值需支持的原币费用、1–358个配对、0–10000bps滑点和1–3600秒盘口有效期".into(),
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExitValuation {
    pub algorithm: String,
    pub pricing: ListingPriceAnalysis,
    pub return_p10_bps: i64,
    pub sample_count: usize,
    pub slippage_bps: u32,
    pub stress_base: String,
    pub stressed_gross_native: Money,
    pub normal_fees: Option<FeeQuote>,
    pub stressed_fees: Option<FeeQuote>,
    pub normal_net_cny: Money,
    pub stressed_net_cny: Money,
}
/// Caller binds identities, account, snapshot times and historical coverage.
/// This calculation never substitutes a historical absolute price for a current quote.
pub fn estimate(
    book: &OrderBook,
    history_currency: &Currency,
    risk: &SevenDayRisk,
    policy: &ExitValuationPolicy,
    fx: Option<&FxSnapshot>,
    at: u64,
) -> Result<ExitValuation, String> {
    policy.validate()?;
    if book.currency != *history_currency || book.currency != policy.fees.currency {
        return Err("盘口、历史和费用必须同一实际原币，不能用当前汇率改写历史币种".into());
    }
    if risk.algorithm != "calendar_p10_v1"
        || risk.sample_count < policy.min_pairs as usize
        || risk.slippage_bps != policy.slippage_bps
    {
        return Err("缺少足够七自然日配对或历史滑点与策略版本不匹配".into());
    }
    let p10 = risk.return_p10_bps.ok_or("历史P10未知，不能生成压力估值")?;
    let (n, d) = if book.currency == Currency::cny() {
        (1, 1)
    } else {
        let saved = fx.ok_or("外币出售估值需要计划内显式保存的汇率；请重新生成含外币盘口的计划")?;
        saved.validate(at)?;
        saved.factor(&book.currency, &Currency::cny())?
    };
    let pricing = wall_gap_price_with_rate(book, &policy.pricing, n, d)?;
    let reference = pricing
        .native_gross_listing_estimate
        .as_ref()
        .ok_or("卖盘为空，没有挂牌参考价")?;
    let stressed = stress_price(reference, p10, policy.slippage_bps)?;
    let normal_fees = policy.fees.net_from_gross(reference)?;
    let stressed_fees = policy.fees.net_from_gross(&stressed)?;
    let to_cny = |quote: &Option<FeeQuote>| -> Result<Money, String> {
        match quote {
            None => Ok(Money::cny(0)),
            Some(q) if q.net.currency == Currency::cny() => Ok(q.net.clone()),
            Some(q) => fx
                .ok_or("缺少原币净额换算证据")?
                .convert(&q.net, &Currency::cny()),
        }
    };
    Ok(ExitValuation {
        algorithm: "wall_gap_p10_exit_v1".into(),
        normal_net_cny: to_cny(&normal_fees)?,
        stressed_net_cny: to_cny(&stressed_fees)?,
        pricing,
        return_p10_bps: p10,
        sample_count: risk.sample_count,
        slippage_bps: policy.slippage_bps,
        stress_base: "current_wall_gap_native_gross".into(),
        stressed_gross_native: stressed,
        normal_fees,
        stressed_fees,
    })
}

#[cfg(test)]
#[path = "exit_valuation_tests.rs"]
pub(crate) mod tests;
