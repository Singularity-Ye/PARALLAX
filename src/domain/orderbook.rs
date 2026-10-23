//! Native-currency public depth evidence, distinct from account quotes and fills.
use super::money::{Currency, Money};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OrderBookRequest {
    pub app_id: u32,
    pub market_hash_name: String,
}
impl OrderBookRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.app_id != 730
            || self.market_hash_name.trim().is_empty()
            || self.market_hash_name != self.market_hash_name.trim()
            || self.market_hash_name.len() > 500
            || self.market_hash_name.chars().any(char::is_control)
        {
            return Err(
                "当前盘口只支持 CS2（730）；需要准确、无首尾空白的市场名称，最多 500 字节".into(),
            );
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PriceLevel {
    pub price_minor: u64,
    pub quantity: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrderBook {
    pub currency: Currency,
    pub asks: Vec<PriceLevel>,
    pub bids: Vec<PriceLevel>,
    pub reported_sell_orders: Option<u64>,
    pub reported_buy_orders: Option<u64>,
}
impl OrderBook {
    pub fn best_ask(&self) -> Option<u64> {
        self.asks.first().map(|l| l.price_minor)
    }
    pub fn best_bid(&self) -> Option<u64> {
        self.bids.first().map(|l| l.price_minor)
    }
    pub fn validate(&self) -> Result<(), String> {
        for (levels, ascending) in [(&self.asks, true), (&self.bids, false)] {
            if levels.len() > 4096 {
                return Err("订单簿档位过多".into());
            }
            let mut last = None;
            let mut total = 0u64;
            for level in levels {
                if level.price_minor == 0
                    || level.price_minor > 1_000_000_000_000
                    || level.quantity == 0
                    || level.quantity > 1_000_000_000_000
                {
                    return Err("订单簿价格或数量无效".into());
                }
                if last.is_some_and(|p| {
                    if ascending {
                        level.price_minor <= p
                    } else {
                        level.price_minor >= p
                    }
                }) {
                    return Err("订单簿未按价格严格排序或存在重复档位".into());
                }
                last = Some(level.price_minor);
                total = total.checked_add(level.quantity).ok_or("订单簿数量溢出")?;
            }
        }
        if self
            .reported_buy_orders
            .is_some_and(|q| q > 1_000_000_000_000)
            || self
                .reported_sell_orders
                .is_some_and(|q| q > 1_000_000_000_000)
        {
            return Err("平台报告数量超出允许范围".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrderBookSnapshot {
    pub id: String,
    pub request: OrderBookRequest,
    pub observed_at: u64,
    pub source: String,
    pub identity_binding: String,
    pub requested_currency: Currency,
    pub book: OrderBook,
    pub summary: OrderBookSummary,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OrderBookSummary {
    pub best_ask: Option<Money>,
    pub best_bid: Option<Money>,
    pub spread_minor: Option<i64>,
    pub visible_ask_quantity: u64,
    pub visible_bid_quantity: u64,
}
impl OrderBookSnapshot {
    pub fn new(request: OrderBookRequest, book: OrderBook, at: u64) -> Result<Self, String> {
        request.validate()?;
        book.validate()?;
        let mut warnings =
            vec!["公开盘口为请求时快照，不保证深度完整；不是成交承诺或账户可执行报价".into()];
        if book.currency != Currency::cny() {
            warnings.push("平台实际返回非 CNY；保留原币，不换汇，不能用于人民币预算".into());
        }
        if book
            .best_ask()
            .zip(book.best_bid())
            .is_some_and(|(ask, bid)| bid > ask)
        {
            warnings.push("买价高于卖价，盘口交叉；禁止用于预算估值".into());
        }
        let summary = OrderBookSummary {
            best_ask: book.best_ask().map(|minor| Money {
                minor,
                currency: book.currency.clone(),
            }),
            best_bid: book.best_bid().map(|minor| Money {
                minor,
                currency: book.currency.clone(),
            }),
            spread_minor: book
                .best_ask()
                .zip(book.best_bid())
                .map(|(ask, bid)| ask as i64 - bid as i64),
            visible_ask_quantity: book.asks.iter().map(|l| l.quantity).sum(),
            visible_bid_quantity: book.bids.iter().map(|l| l.quantity).sum(),
        };
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            request,
            observed_at: at,
            source: "steam_public_query_action".into(),
            identity_binding: "request_parameters".into(),
            requested_currency: Currency::cny(),
            book,
            summary,
            warnings,
        })
    }
    pub fn cny_ask(&self, at: u64, max_age: u64) -> Result<u64, String> {
        self.request.validate()?;
        self.book.validate()?;
        if self.source != "steam_public_query_action" || self.book.currency != Currency::cny() {
            return Err("预算只接受本应用保存的 CNY Steam 盘口，不能把外币视为人民币".into());
        }
        if self.observed_at > at || at - self.observed_at > max_age {
            return Err("Steam 盘口快照已过期或时间异常".into());
        }
        let ask = self
            .book
            .best_ask()
            .ok_or("Steam 卖盘为空，不能生成钱包估值")?;
        if self.book.best_bid().is_some_and(|bid| bid > ask) {
            return Err("Steam 盘口交叉，不能用于预算估值".into());
        }
        Ok(ask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn fixture() -> OrderBookSnapshot {
        OrderBookSnapshot::new(
            OrderBookRequest {
                app_id: 730,
                market_hash_name: "Prisma 2 Case".into(),
            },
            OrderBook {
                currency: Currency::cny(),
                asks: vec![PriceLevel {
                    price_minor: 1150,
                    quantity: 2,
                }],
                bids: vec![PriceLevel {
                    price_minor: 1000,
                    quantity: 1,
                }],
                reported_sell_orders: None,
                reported_buy_orders: None,
            },
            100,
        )
        .unwrap()
    }
    #[test]
    fn summary_keeps_native_values_and_missing_depth_unknown() {
        let s = fixture();
        assert_eq!(s.summary.spread_minor, Some(150));
        assert_eq!(s.summary.visible_ask_quantity, 2);
        assert_eq!(s.cny_ask(101, 300).unwrap(), 1150);
        let empty = OrderBookSnapshot::new(
            s.request,
            OrderBook {
                currency: Currency::cny(),
                asks: vec![],
                bids: vec![],
                reported_sell_orders: Some(0),
                reported_buy_orders: Some(0),
            },
            100,
        )
        .unwrap();
        assert!(empty.summary.best_ask.is_none());
        assert!(empty.summary.spread_minor.is_none());
        assert!(empty.cny_ask(101, 300).is_err());
    }
    #[test]
    fn stale_foreign_crossed_and_wrong_app_snapshots_cannot_value_cny_plans() {
        let s = fixture();
        assert!(s.cny_ask(99, 300).is_err());
        assert!(s.cny_ask(401, 300).is_err());
        let mut foreign = s.clone();
        foreign.book.currency = "TWD".to_string().try_into().unwrap();
        assert!(foreign.cny_ask(101, 300).is_err());
        let mut crossed = s.clone();
        crossed.book.bids[0].price_minor = 1200;
        assert!(crossed.cny_ask(101, 300).is_err());
        let mut wrong = s;
        wrong.request.app_id = 570;
        assert!(wrong.cny_ask(101, 300).is_err());
    }
}
