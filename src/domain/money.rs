//! Integer amounts and explicit fee assumptions. No live settlement claims or FX fallback.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct Currency(String);
impl Currency {
    pub fn cny() -> Self {
        Self("CNY".into())
    }
}
impl TryFrom<String> for Currency {
    type Error = String;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() != 3 || !value.bytes().all(|b| b.is_ascii_uppercase()) {
            return Err("币种必须是三位大写代码".into());
        }
        Ok(Self(value))
    }
}
impl From<Currency> for String {
    fn from(value: Currency) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Money {
    pub minor: u64,
    pub currency: Currency,
}
impl Money {
    pub fn cny(minor: u64) -> Self {
        Self {
            minor,
            currency: Currency::cny(),
        }
    }
    pub fn checked_add(&self, other: &Self) -> Result<Self, String> {
        if self.currency != other.currency {
            return Err("币种不同，需显式汇率快照后再合计".into());
        }
        Ok(Self {
            minor: self.minor.checked_add(other.minor).ok_or("金额合计溢出")?,
            currency: self.currency.clone(),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeeSchedule {
    pub currency: Currency,
    pub platform_bps: u32,
    pub publisher_bps: u32,
    pub platform_min_minor: u64,
    pub publisher_min_minor: u64,
    pub increment_minor: u64,
}
impl FeeSchedule {
    pub fn validate(&self) -> Result<(), String> {
        if self.platform_bps > 10_000 || self.publisher_bps > 10_000 || self.increment_minor == 0 {
            return Err("费用比例须在 0–10000 bps 内，金额步长必须大于零".into());
        }
        Ok(())
    }
    fn component(net: u64, bps: u32, minimum: u64) -> u128 {
        if bps == 0 {
            0
        } else {
            ((net as u128) * (bps as u128) / 10_000).max(minimum as u128)
        }
    }
    fn total(&self, net: u64) -> u128 {
        net as u128
            + Self::component(net, self.platform_bps, self.platform_min_minor)
            + Self::component(net, self.publisher_bps, self.publisher_min_minor)
    }
    /// Largest step-aligned seller net whose modeled buyer total does not exceed gross.
    pub fn net_from_gross(&self, gross: &Money) -> Result<Option<FeeQuote>, String> {
        self.validate()?;
        if gross.currency != self.currency {
            return Err("费用模型与报价币种不一致".into());
        }
        let (mut lo, mut hi) = (0, gross.minor / self.increment_minor);
        while lo < hi {
            let mid = lo + (hi - lo).div_ceil(2);
            if self.total(mid * self.increment_minor) <= gross.minor as u128 {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        let net = lo * self.increment_minor;
        if net == 0 {
            return Ok(None);
        }
        let used = u64::try_from(self.total(net)).map_err(|_| "费用金额溢出")?;
        Ok(Some(FeeQuote {
            net: Money {
                minor: net,
                currency: gross.currency.clone(),
            },
            fees_minor: used - net,
            gross_used_minor: used,
            unallocated_gross_minor: gross.minor - used,
        }))
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FeeQuote {
    pub net: Money,
    pub fees_minor: u64,
    pub gross_used_minor: u64,
    pub unallocated_gross_minor: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WalletPolicy {
    #[default]
    Legacy115,
    ParameterizedFees {
        schedule: FeeSchedule,
    },
}
impl WalletPolicy {
    pub fn validate_cny(&self) -> Result<(), String> {
        if let Self::ParameterizedFees { schedule } = self {
            schedule.validate()?;
            if schedule.currency != Currency::cny() {
                return Err("当前扫描与计划只接收 CNY；不能把其他币种当人民币使用".into());
            }
        }
        Ok(())
    }
    pub fn estimate(&self, gross_cents: u64) -> Result<Option<FeeQuote>, String> {
        match self {
            Self::Legacy115 => {
                let net =
                    u64::try_from((gross_cents as u128) * 100 / 115).map_err(|_| "钱包估算溢出")?;
                Ok((net > 0).then(|| FeeQuote {
                    net: Money::cny(net),
                    fees_minor: gross_cents - net,
                    gross_used_minor: gross_cents,
                    unallocated_gross_minor: 0,
                }))
            }
            Self::ParameterizedFees { schedule } => {
                schedule.net_from_gross(&Money::cny(gross_cents))
            }
        }
    }
}

pub fn ratio_bps(cash: u64, net: u64) -> Option<u64> {
    if net == 0 {
        return None;
    }
    u64::try_from(((cash as u128) * 10_000).div_ceil(net as u128)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn schedule() -> FeeSchedule {
        FeeSchedule {
            currency: Currency::cny(),
            platform_bps: 500,
            publisher_bps: 1000,
            platform_min_minor: 7,
            publisher_min_minor: 7,
            increment_minor: 1,
        }
    }
    #[test]
    fn minimum_fees_step_and_no_affordable_net_are_explicit() {
        let s = schedule();
        assert!(s.net_from_gross(&Money::cny(14)).unwrap().is_none());
        let q = s.net_from_gross(&Money::cny(15)).unwrap().unwrap();
        assert_eq!((q.net.minor, q.fees_minor), (1, 14));
        let stepped = FeeSchedule {
            increment_minor: 5,
            ..s
        };
        let q = stepped.net_from_gross(&Money::cny(103)).unwrap().unwrap();
        assert_eq!(q.net.minor % 5, 0);
        assert!(q.gross_used_minor <= 103);
        assert!(stepped.total(q.net.minor + 5) > 103);
    }
    #[test]
    fn inverse_is_maximal_and_overflow_never_wraps() {
        let s = schedule();
        for gross in 1..2000 {
            if let Some(q) = s.net_from_gross(&Money::cny(gross)).unwrap() {
                assert_eq!(
                    q.net.minor + q.fees_minor + q.unallocated_gross_minor,
                    gross
                );
                assert!(s.total(q.net.minor + 1) > gross as u128);
            }
        }
        assert!(
            WalletPolicy::Legacy115
                .estimate(u64::MAX)
                .unwrap()
                .unwrap()
                .net
                .minor
                > u64::MAX / 2
        );
        assert_eq!(ratio_bps(1, 3), Some(3334));
        assert_eq!(ratio_bps(1, 0), None);
        assert_eq!(ratio_bps(u64::MAX, 1), None);
    }
    #[test]
    fn currencies_do_not_mix_and_zero_rate_has_zero_fee() {
        let usd = Money {
            minor: 10,
            currency: Currency::try_from("USD".to_string()).unwrap(),
        };
        assert!(Money::cny(10).checked_add(&usd).is_err());
        assert!(schedule().net_from_gross(&usd).is_err());
        let zero = FeeSchedule {
            platform_bps: 0,
            publisher_bps: 0,
            ..schedule()
        };
        assert_eq!(
            zero.net_from_gross(&Money::cny(10))
                .unwrap()
                .unwrap()
                .net
                .minor,
            10
        );
    }
}
