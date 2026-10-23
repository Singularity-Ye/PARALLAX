//! Account-bound cash-side evidence; visible listings are neither locked nor purchased.
use super::{money::Currency, orderbook::PriceLevel};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BuffQuoteRequest {
    pub scan_id: String,
    pub item_key: String,
}
impl BuffQuoteRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.scan_id.is_empty()
            || self.scan_id.len() > 64
            || self.item_key.is_empty()
            || self.item_key.len() > 500
            || self.item_key.chars().any(char::is_control)
        {
            return Err("需要有效扫描 ID 与准确候选 key".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BuffSellPage {
    pub goods_id: u64,
    pub market_hash_name: String,
    pub levels: Vec<PriceLevel>,
    pub reported_total: Option<u64>,
}
impl BuffSellPage {
    pub fn validate(&self, goods_id: u64, name: &str) -> Result<(), String> {
        if self.goods_id != goods_id
            || self.goods_id == 0
            || self.goods_id > 1_000_000_000
            || self.market_hash_name != name
        {
            return Err("BUFF 商品身份与扫描候选不匹配".into());
        }
        let mut last = 0;
        let mut total = 0u64;
        for level in &self.levels {
            if level.price_minor == 0
                || level.price_minor > 100_000_000
                || level.price_minor <= last
                || level.quantity == 0
                || level.quantity > 80
            {
                return Err("BUFF 卖单档位无效".into());
            }
            last = level.price_minor;
            total = total.checked_add(level.quantity).ok_or("BUFF 数量溢出")?;
        }
        if total > 80 || self.reported_total.is_some_and(|q| q > 1_000_000_000) {
            return Err("BUFF 可见卖单数量超限".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BuffQuoteSnapshot {
    pub id: String,
    pub account_id: String,
    pub credential_generation: String,
    pub request: BuffQuoteRequest,
    pub observed_at: u64,
    pub currency: Currency,
    pub currency_basis: String,
    pub source: String,
    pub coverage: String,
    pub page: BuffSellPage,
    pub lowest_cash_cents: Option<u64>,
    pub lowest_tier_visible_units: u32,
}
impl BuffQuoteSnapshot {
    pub fn new(
        account_id: String,
        generation: String,
        request: BuffQuoteRequest,
        page: BuffSellPage,
        at: u64,
    ) -> Result<Self, String> {
        request.validate()?;
        page.validate(page.goods_id, &request.item_key)?;
        let lowest_cash_cents = page.levels.first().map(|l| l.price_minor);
        let lowest_tier_visible_units = page.levels.first().map_or(0, |l| l.quantity as u32);
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            account_id,
            credential_generation: generation,
            request,
            observed_at: at,
            currency: Currency::cny(),
            currency_basis: "buff_cny_protocol_not_live_verified".into(),
            source: "buff_sell_orders_readonly".into(),
            coverage: "first_page_lowest_tier_only".into(),
            page,
            lowest_cash_cents,
            lowest_tier_visible_units,
        })
    }
    pub fn cash_quote(
        &self,
        goods_id: u64,
        key: &str,
        at: u64,
        max_age: u64,
    ) -> Result<(Option<u64>, u32), String> {
        self.request.validate()?;
        self.page.validate(goods_id, key)?;
        if self.source != "buff_sell_orders_readonly"
            || self.currency != Currency::cny()
            || self.request.item_key != key
            || self.observed_at > at
            || at - self.observed_at > max_age
        {
            return Err("BUFF 快照来源、币种、商品或时间无效".into());
        }
        // Derive from evidence again rather than trusting denormalized display fields.
        Ok(self
            .page
            .levels
            .first()
            .map_or((None, 0), |l| (Some(l.price_minor), l.quantity as u32)))
    }
}
