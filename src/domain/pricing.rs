//! Explainable CNY wall/gap listing research. This is not a sell order or a fill forecast.
use super::{
    money::{Currency, Money},
    orderbook::{OrderBook, PriceLevel},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WallGapPolicy {
    pub currency: Currency,
    pub wall_volume_threshold: u64,
    pub max_ignore_volume: u64,
    pub min_lowest_tier_volume: u64,
    pub min_step_minor: u64,
    pub min_floor_minor: u64,
    pub offset_minor: i64,
}
impl Default for WallGapPolicy {
    fn default() -> Self {
        Self {
            currency: Currency::cny(),
            wall_volume_threshold: 20,
            max_ignore_volume: 4,
            min_lowest_tier_volume: 3,
            min_step_minor: 1,
            min_floor_minor: 3,
            offset_minor: 0,
        }
    }
}
impl WallGapPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.currency != Currency::cny() {
            return Err("壁垒/断层 v1 使用人民币价格区间，只支持 CNY，不自动换汇".into());
        }
        if !(1..=1_000_000_000_000).contains(&self.wall_volume_threshold)
            || self.max_ignore_volume > 1_000_000_000_000
            || self.min_lowest_tier_volume > 1_000_000_000_000
            || !(1..=1_000_000_000_000).contains(&self.min_step_minor)
            || !(1..=1_000_000_000_000).contains(&self.min_floor_minor)
            || self.offset_minor.unsigned_abs() > 1_000_000_000_000
        {
            return Err("定价数量/步长/下限/偏移超出范围；墙量、步长和下限须大于零".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PricingReason {
    EmptyAsks,
    SingleLevel,
    RegularUndercut,
    GapJump,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WallEvidence {
    /// Index in the original, strictly ascending ask levels (not the trimmed slice).
    pub level_index: usize,
    pub price_minor: u64,
    pub cumulative_quantity: u64,
    pub reached_threshold: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GapCheck {
    pub lower_minor: u64,
    pub upper_minor: u64,
    pub blocking_quantity: u64,
    pub absolute_threshold_minor: u64,
    pub relative_threshold_bps: u32,
    pub jumped: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ListingPriceAnalysis {
    pub algorithm: String,
    pub book_currency: Currency,
    pub native_gross_listing_estimate: Option<Money>,
    pub gross_listing_estimate: Option<Money>,
    pub reason: PricingReason,
    pub ignored_lowest_tiers: Vec<PriceLevel>,
    pub wall: Option<WallEvidence>,
    pub gap_checks: Vec<GapCheck>,
    pub stopped_by_blocking_volume: bool,
    pub floor_applied: bool,
}

/// Integer CNY cents / bps. These are inherited strategy parameters, not platform rules.
fn thresholds(price_minor: u64) -> (u64, u32) {
    match price_minor {
        0..500 => (10, 800),
        500..2000 => (30, 500),
        2000..10000 => (100, 300),
        10000..50000 => (500, 200),
        _ => (1000, 150),
    }
}

pub fn wall_gap_price(
    book: &OrderBook,
    policy: &WallGapPolicy,
) -> Result<ListingPriceAnalysis, String> {
    if book.currency != Currency::cny() {
        return Err("外币定价需要有效汇率快照".into());
    }
    wall_gap_price_with_rate(book, policy, 1, 1)
}

/// Exact CNY-minor/native-minor ratio. No quantized/merged synthetic order book.
pub fn wall_gap_price_with_rate(
    book: &OrderBook,
    policy: &WallGapPolicy,
    n: u64,
    d: u64,
) -> Result<ListingPriceAnalysis, String> {
    policy.validate()?;
    book.validate()?;
    if !super::fx::supported(&book.currency)
        || n == 0
        || d == 0
        || n > 1_000_000_000_000
        || d > 1_000_000_000_000
    {
        return Err("汇率比率或币种无效".into());
    }
    let (n, d) = (n as u128, d as u128);
    if book
        .asks
        .last()
        .is_some_and(|l| (l.price_minor as u128) * n / d > 1_000_000_000_000)
    {
        return Err("折合人民币价格超出范围".into());
    }
    if book
        .best_ask()
        .zip(book.best_bid())
        .is_some_and(|(ask, bid)| bid > ask)
    {
        return Err("盘口交叉，不能计算挂牌参考价".into());
    }
    let mut result = ListingPriceAnalysis {
        algorithm: if book.currency == Currency::cny() {
            "wall_gap_cny_v1"
        } else {
            "wall_gap_fx_v1"
        }
        .into(),
        book_currency: book.currency.clone(),
        native_gross_listing_estimate: None,
        gross_listing_estimate: None,
        reason: PricingReason::EmptyAsks,
        ignored_lowest_tiers: vec![],
        wall: None,
        gap_checks: vec![],
        stopped_by_blocking_volume: false,
        floor_applied: false,
    };
    if book.asks.is_empty() {
        return Ok(result);
    }
    // Inherited rule: trim each small lowest tier while retaining at least one.
    // This is separate from the cumulative blocking-volume budget used below.
    let mut start = 0;
    while start + 1 < book.asks.len() && book.asks[start].quantity <= policy.min_lowest_tier_volume
    {
        result.ignored_lowest_tiers.push(book.asks[start].clone());
        start += 1;
    }
    let mut cumulative = 0;
    let mut wall_index = start;
    for (index, level) in book.asks.iter().enumerate().skip(start) {
        cumulative += level.quantity; // validated: <=4096 levels, each <=10^12
        wall_index = index;
        if cumulative >= policy.wall_volume_threshold {
            break;
        }
    }
    result.wall = Some(WallEvidence {
        level_index: wall_index,
        price_minor: book.asks[wall_index].price_minor,
        cumulative_quantity: cumulative,
        reached_threshold: cumulative >= policy.wall_volume_threshold,
    });
    let scope = &book.asks[start..=wall_index];
    let mut anchor = scope[0].price_minor;
    result.reason = if scope.len() == 1 {
        PricingReason::SingleLevel
    } else {
        PricingReason::RegularUndercut
    };
    let mut blocking = 0;
    for pair in scope.windows(2) {
        let (lower, upper) = (&pair[0], &pair[1]);
        blocking += lower.quantity;
        if blocking > policy.max_ignore_volume {
            result.stopped_by_blocking_volume = true;
            break;
        }
        let (absolute, relative) = thresholds(((lower.price_minor as u128) * n / d) as u64);
        let gap = upper.price_minor - lower.price_minor;
        // Strictly greater than BOTH thresholds, without rounding relative cents.
        let jumped = (gap as u128) * n > (absolute as u128) * d
            && (gap as u128) * 10_000 > (lower.price_minor as u128) * (relative as u128);
        result.gap_checks.push(GapCheck {
            lower_minor: lower.price_minor,
            upper_minor: upper.price_minor,
            blocking_quantity: blocking,
            absolute_threshold_minor: absolute,
            relative_threshold_bps: relative,
            jumped,
        });
        if jumped {
            anchor = upper.price_minor;
            result.reason = PricingReason::GapJump;
        }
    }
    // Signed arithmetic matters when an undercut crosses zero before a positive offset.
    let adjusted = (anchor as i128) * (n as i128) - (policy.min_step_minor as i128) * (d as i128)
        + (policy.offset_minor as i128) * (d as i128);
    let floor = (policy.min_floor_minor as i128) * (d as i128);
    result.floor_applied = adjusted < floor;
    // Round the native research price up, preserving the configured CNY floor.
    let native_price = (adjusted.max(floor) as u128).div_ceil(n);
    let price = native_price * n / d;
    if price > 1_000_000_000_000 || native_price > 1_000_000_000_000 {
        return Err("计算挂牌价超出支持的金额范围".into());
    }
    result.native_gross_listing_estimate = Some(Money {
        minor: native_price as u64,
        currency: book.currency.clone(),
    });
    result.gross_listing_estimate = Some(Money::cny(price as u64));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn book(levels: &[(u64, u64)]) -> OrderBook {
        OrderBook {
            currency: Currency::cny(),
            asks: levels
                .iter()
                .map(|&(price_minor, quantity)| PriceLevel {
                    price_minor,
                    quantity,
                })
                .collect(),
            bids: vec![],
            reported_sell_orders: None,
            reported_buy_orders: None,
        }
    }
    fn price(levels: &[(u64, u64)], p: &WallGapPolicy) -> ListingPriceAnalysis {
        wall_gap_price(&book(levels), p).unwrap()
    }
    fn gross(r: &ListingPriceAnalysis) -> u64 {
        r.gross_listing_estimate.as_ref().unwrap().minor
    }
    #[test]
    fn fx_comparisons_keep_native_tiers_and_cny_thresholds_exact() {
        let mut b = book(&[(5000, 1), (5500, 30)]);
        b.currency = "TWD".to_string().try_into().unwrap();
        let p = WallGapPolicy {
            min_lowest_tier_volume: 0,
            ..Default::default()
        };
        let r = wall_gap_price_with_rate(&b, &p, 1, 5).unwrap();
        assert_eq!(gross(&r), 1099);
        assert_eq!(r.native_gross_listing_estimate.unwrap().minor, 5495);
        assert_eq!(r.reason, PricingReason::GapJump);
        assert_eq!(r.gap_checks[0].lower_minor, 5000);
        b.asks[1].price_minor = 5250; // 50 CNY cents == 5%; equality must NOT jump.
        assert_eq!(
            wall_gap_price_with_rate(&b, &p, 1, 5).unwrap().reason,
            PricingReason::RegularUndercut
        );
        let mut b = book(&[(1, 30)]);
        b.currency = "TWD".to_string().try_into().unwrap();
        let r = wall_gap_price_with_rate(&b, &p, 3, 7).unwrap();
        assert!(gross(&r) >= 3);
        assert!(r.floor_applied);
        assert_eq!(r.native_gross_listing_estimate.unwrap().minor, 7);
    }
    #[test]
    fn empty_and_single_level_keep_absence_and_floor_explicit() {
        let p = WallGapPolicy::default();
        let empty = price(&[], &p);
        assert!(empty.gross_listing_estimate.is_none());
        assert!(empty.wall.is_none());
        assert_eq!(empty.reason, PricingReason::EmptyAsks);
        let single = price(&[(1, 1)], &p);
        assert_eq!(gross(&single), 3);
        assert!(single.floor_applied);
        assert!(single.ignored_lowest_tiers.is_empty());
        assert_eq!(single.reason, PricingReason::SingleLevel);
    }
    #[test]
    fn lowest_tier_trimming_is_per_tier_and_keeps_last_level() {
        let r = price(&[(100, 3), (110, 3), (120, 3)], &WallGapPolicy::default());
        assert_eq!(r.ignored_lowest_tiers.len(), 2);
        assert_eq!(gross(&r), 119);
        let wall = r.wall.unwrap();
        assert_eq!(wall.level_index, 2);
        assert_eq!(wall.cumulative_quantity, 3);
        assert!(!wall.reached_threshold);
    }
    #[test]
    fn wall_limits_scope_and_blocking_volume_stops_gap_search() {
        let r = price(&[(1000, 20), (2000, 30)], &WallGapPolicy::default());
        assert_eq!(gross(&r), 999);
        assert_eq!(r.reason, PricingReason::SingleLevel);
        assert!(r.wall.unwrap().reached_threshold);
        let r = price(&[(1000, 5), (2000, 30)], &WallGapPolicy::default());
        assert!(r.stopped_by_blocking_volume);
        assert!(r.gap_checks.is_empty());
        assert_eq!(gross(&r), 999);
    }
    #[test]
    fn last_gap_is_retained_even_if_later_blocking_volume_stops_search() {
        let p = WallGapPolicy {
            min_lowest_tier_volume: 0,
            max_ignore_volume: 4,
            ..Default::default()
        };
        let r = price(&[(1000, 2), (1100, 2), (1300, 3), (2000, 30)], &p);
        assert_eq!(gross(&r), 1299);
        assert_eq!(r.reason, PricingReason::GapJump);
        assert_eq!(r.gap_checks.len(), 2);
        assert!(r.stopped_by_blocking_volume);
        assert_eq!(r.gap_checks[1].blocking_quantity, 4);
    }
    #[test]
    fn dynamic_ranges_and_strict_threshold_equalities_are_integer_exact() {
        let p = WallGapPolicy {
            min_lowest_tier_volume: 0,
            max_ignore_volume: 1,
            ..Default::default()
        };
        for (lower, threshold) in [
            (100, 10),
            (499, 39),
            (500, 30),
            (1999, 99),
            (2000, 100),
            (9999, 299),
            (10000, 500),
            (49999, 999),
            (50000, 1000),
            (100000, 1500),
        ] {
            let r = price(&[(lower, 1), (lower + threshold, 30)], &p);
            assert_eq!(r.reason, PricingReason::RegularUndercut, "at {lower}");
            let r = price(&[(lower, 1), (lower + threshold + 1, 30)], &p);
            assert_eq!(r.reason, PricingReason::GapJump, "at {lower}");
            assert_eq!(gross(&r), lower + threshold);
        }
    }
    #[test]
    fn offset_is_applied_after_undercut_and_before_floor_without_wrapping() {
        let p = WallGapPolicy {
            min_step_minor: 10,
            offset_minor: 20,
            ..Default::default()
        };
        assert_eq!(gross(&price(&[(5, 30)], &p)), 15);
        let p = WallGapPolicy {
            offset_minor: -1000,
            ..Default::default()
        };
        let r = price(&[(100, 30)], &p);
        assert_eq!(gross(&r), 3);
        assert!(r.floor_applied);
        let p = WallGapPolicy {
            offset_minor: 1_000_000_000_000,
            ..Default::default()
        };
        assert!(wall_gap_price(&book(&[(100, 30)]), &p).is_err());
    }
    #[test]
    fn foreign_crossed_unsorted_duplicate_zero_and_invalid_policy_are_rejected() {
        let p = WallGapPolicy::default();
        for levels in [
            &[(100, 1), (90, 2)][..],
            &[(100, 1), (100, 2)],
            &[(0, 1)],
            &[(100, 0)],
        ] {
            assert!(wall_gap_price(&book(levels), &p).is_err());
        }
        let mut b = book(&[(100, 1)]);
        b.currency = "TWD".to_string().try_into().unwrap();
        assert!(wall_gap_price(&b, &p).is_err());
        b = book(&[(100, 1)]);
        b.bids = vec![PriceLevel {
            price_minor: 101,
            quantity: 1,
        }];
        assert!(wall_gap_price(&b, &p).is_err());
        for invalid in [
            WallGapPolicy {
                wall_volume_threshold: 0,
                ..p.clone()
            },
            WallGapPolicy {
                min_step_minor: 0,
                ..p.clone()
            },
            WallGapPolicy {
                min_floor_minor: 0,
                ..p.clone()
            },
            WallGapPolicy {
                offset_minor: i64::MIN,
                ..p.clone()
            },
            WallGapPolicy {
                currency: "USD".to_string().try_into().unwrap(),
                ..p
            },
        ] {
            assert!(wall_gap_price(&book(&[(100, 30)]), &invalid).is_err());
        }
    }
}
