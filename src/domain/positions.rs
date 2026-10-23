//! Manual preview inputs, never represented as platform-verified holdings or availability.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PositionEntry {
    pub key: String,
    pub held_units: u32,
    pub pending_buy_units: u32,
    pub available_units: Option<u32>,
    pub daily_volume: Option<u64>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PositionSnapshot {
    pub account_id: String,
    pub observed_at: u64,
    pub reserved_cash_cents: u64,
    pub entries: Vec<PositionEntry>,
}
impl PositionSnapshot {
    pub fn validate(&self) -> Result<(), String> {
        if self.account_id.is_empty() || self.account_id.len() > 64 || self.entries.len() > 2000 {
            return Err("仓位快照需要账户 ID，最多 2000 个商品条目".into());
        }
        let mut keys = std::collections::HashSet::new();
        for entry in &self.entries {
            if entry.key.is_empty() || entry.key.len() > 500 || !keys.insert(&entry.key) {
                return Err("仓位快照的商品标识为空、重复或过长".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LiquidityPolicy {
    pub participation_bps: u32,
    pub low_price_threshold_cents: u64,
    pub low_price_penalty_bps: u32,
    pub low_price_cap: u32,
}
impl LiquidityPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if self.participation_bps > 10_000 || self.low_price_penalty_bps > 10_000 {
            return Err("日量占比和低价系数须在 0–10000 bps 内".into());
        }
        Ok(())
    }
}

pub fn remaining_units(
    price: u64,
    cap: u32,
    entry: Option<&PositionEntry>,
    policy: Option<&LiquidityPolicy>,
) -> Result<u32, String> {
    let mut total_cap = u64::from(cap);
    if let Some(policy) = policy {
        policy.validate()?;
        let daily = entry
            .and_then(|e| e.daily_volume)
            .ok_or("缺少用户填报的日成交量；未将来源成交量猜作日量")?;
        let mut volume_cap = (daily as u128) * (policy.participation_bps as u128) / 10_000;
        if price < policy.low_price_threshold_cents {
            volume_cap = volume_cap * (policy.low_price_penalty_bps as u128) / 10_000;
            total_cap = total_cap.min(u64::from(policy.low_price_cap));
        }
        total_cap = total_cap.min(u64::try_from(volume_cap).map_err(|_| "日量上限溢出")?);
    }
    if let Some(entry) = entry {
        let exposure = u64::from(entry.held_units) + u64::from(entry.pending_buy_units);
        total_cap = total_cap.saturating_sub(exposure);
        if let Some(available) = entry.available_units {
            total_cap = total_cap.min(u64::from(available));
        }
    }
    Ok(total_cap as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn held_pending_liquidity_and_zero_availability_are_not_ignored() {
        let mut entry = PositionEntry {
            key: "x".into(),
            held_units: 2,
            pending_buy_units: 1,
            available_units: None,
            daily_volume: Some(200),
        };
        let policy = LiquidityPolicy {
            participation_bps: 500,
            low_price_threshold_cents: 500,
            low_price_penalty_bps: 5000,
            low_price_cap: 30,
        };
        assert_eq!(
            remaining_units(400, 50, Some(&entry), Some(&policy)).unwrap(),
            2
        );
        assert_eq!(
            remaining_units(600, 50, Some(&entry), Some(&policy)).unwrap(),
            7
        );
        entry.available_units = Some(0);
        assert_eq!(
            remaining_units(600, 50, Some(&entry), Some(&policy)).unwrap(),
            0
        );
        entry.daily_volume = None;
        assert!(remaining_units(600, 50, Some(&entry), Some(&policy)).is_err());
    }
}
