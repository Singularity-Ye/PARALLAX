//! Bounded observations, never a payment verdict or proof of absence.
use super::ledger::text_id;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderHistoryRequest {
    pub max_pages: u32,
    pub page_size: u32,
}
impl OrderHistoryRequest {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=5).contains(&self.max_pages) || !(1..=80).contains(&self.page_size) {
            return Err("订单观察限1–5页，每页1–80条".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrderStateCode {
    Text(String),
    Number(i64),
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedOrder {
    pub order_id: String,
    pub state_code: Option<OrderStateCode>,
    pub pay_expire_timeout_secs: Option<i64>,
}
impl ObservedOrder {
    pub fn validate(&self) -> Result<(), String> {
        if !identifier(&self.order_id)
            || self
                .pay_expire_timeout_secs
                .is_some_and(|n| n.unsigned_abs() > 1_000_000_000)
            || matches!(&self.state_code, Some(OrderStateCode::Text(s)) if !identifier(s))
        {
            return Err("订单标识、状态码或支付时限无效".into());
        }
        Ok(())
    }
    pub fn payment_evidence(&self) -> PaymentEvidence {
        // R's textual state is useful pending evidence; A's numeric paid map
        // has not been validated on this project's actual platform account.
        match (&self.state_code, self.pay_expire_timeout_secs) {
            (Some(OrderStateCode::Text(s)), Some(t)) if s == "PAYING" && t >= 0 => {
                PaymentEvidence::AwaitingPaymentAtObservation
            }
            _ => PaymentEvidence::Unknown,
        }
    }
}
pub fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 96
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentEvidence {
    AwaitingPaymentAtObservation,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderHistoryPage {
    pub page_num: u32,
    pub page_size: u32,
    pub reported_total: Option<u64>,
    pub orders: Vec<ObservedOrder>,
}
impl OrderHistoryPage {
    pub fn validate(&self, number: u32, size: u32) -> Result<(), String> {
        if self.page_num != number
            || self.page_size != size
            || self.orders.len() > size as usize
            || self
                .reported_total
                .is_some_and(|n| n > 1_000_000_000 || n < self.orders.len() as u64)
        {
            return Err("订单分页范围或条数无效".into());
        }
        let mut seen = HashSet::new();
        for order in &self.orders {
            order.validate()?;
            if !seen.insert(&order.order_id) {
                return Err("同页出现重复订单，不能把不稳定结果当新快照".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HistoryStop {
    ShortPage,
    ReportedEnd,
    PageBudget,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderHistorySnapshot {
    pub id: String,
    pub account_id: String,
    pub principal: String,
    pub credential_generation: String,
    pub started_at: u64,
    pub observed_at: u64,
    pub source: String,
    pub protocol_live_verified: bool,
    pub request: OrderHistoryRequest,
    pub pages: Vec<OrderHistoryPage>,
    pub stop_reason: HistoryStop,
    pub complete: bool,
    pub execution_allowed: bool,
}
impl OrderHistorySnapshot {
    pub fn new(
        account: String,
        principal: String,
        generation: String,
        request: OrderHistoryRequest,
        pages: Vec<OrderHistoryPage>,
        started_at: u64,
        at: u64,
    ) -> Result<Self, String> {
        request.validate()?;
        if !text_id(&account, 64)
            || !text_id(&principal, 128)
            || !text_id(&generation, 128)
            || started_at == 0
            || at < started_at
            || pages.is_empty()
            || pages.len() > request.max_pages as usize
        {
            return Err("订单观察身份、时间或页数无效".into());
        }
        let mut seen = HashSet::new();
        let mut reported = None;
        let mut stop = None;
        for (index, page) in pages.iter().enumerate() {
            if stop.is_some() {
                return Err("已到观察终点后仍追加页面".into());
            }
            page.validate(index as u32 + 1, request.page_size)?;
            if index > 0 && page.reported_total != reported {
                return Err("分页总数变化，请重新观察；旧快照保留".into());
            }
            reported = page.reported_total;
            for order in &page.orders {
                if !seen.insert(&order.order_id) {
                    return Err("跨页重复订单，分页可能移动；不保存部分结果".into());
                }
            }
            if reported.is_some_and(|n| n < seen.len() as u64) {
                return Err("订单总数与观察不一致".into());
            }
            if page.orders.len() < request.page_size as usize {
                if reported.is_some_and(|n| n != seen.len() as u64) {
                    return Err("短页与总数不一致".into());
                }
                stop = Some(HistoryStop::ShortPage);
            } else if reported == Some(seen.len() as u64) {
                stop = Some(HistoryStop::ReportedEnd);
            }
        }
        let stop_reason = match stop {
            Some(reason) => reason,
            None if pages.len() == request.max_pages as usize => HistoryStop::PageBudget,
            _ => return Err("观察尚未完成请求页数".into()),
        };
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            account_id: account,
            principal,
            credential_generation: generation,
            started_at,
            observed_at: at,
            source: "buff_buy_history_readonly".into(),
            protocol_live_verified: false,
            request,
            pages,
            stop_reason,
            complete: false,
            execution_allowed: false,
        })
    }
    pub fn lookup(&self, order_id: &str, at: u64) -> Result<OrderLookup, String> {
        if !identifier(order_id) {
            return Err("请填写准确订单号，不能按名称猜测".into());
        }
        let order = self
            .pages
            .iter()
            .flat_map(|p| &p.orders)
            .find(|o| o.order_id == order_id)
            .cloned();
        let payment_evidence = order
            .as_ref()
            .map_or(PaymentEvidence::Unknown, ObservedOrder::payment_evidence);
        let reason = match (&order, &payment_evidence) {
            (None, _) => "本次已观察页面没有该订单；不证明不存在、已取消或付款失败",
            (_, PaymentEvidence::AwaitingPaymentAtObservation) => {
                "保存时为PAYING且支付窗口未过期；不代表当前状态或已付款"
            }
            _ => "已找到准确订单号，但该状态未完成本项目实机付款语义验收；支付结果未知",
        };
        Ok(OrderLookup {
            snapshot_id: self.id.clone(),
            order_id: order_id.into(),
            found: order.is_some(),
            order,
            payment_evidence,
            reason: reason.into(),
            observation_age_secs: at.checked_sub(self.observed_at),
            execution_allowed: false,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderLookupRequest {
    pub order_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderLookup {
    pub snapshot_id: String,
    pub order_id: String,
    pub found: bool,
    pub order: Option<ObservedOrder>,
    pub payment_evidence: PaymentEvidence,
    pub reason: String,
    pub observation_age_secs: Option<u64>,
    pub execution_allowed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn page(number: u32, ids: &[&str], total: Option<u64>) -> OrderHistoryPage {
        OrderHistoryPage {
            page_num: number,
            page_size: 2,
            reported_total: total,
            orders: ids
                .iter()
                .map(|id| ObservedOrder {
                    order_id: (*id).into(),
                    state_code: None,
                    pay_expire_timeout_secs: None,
                })
                .collect(),
        }
    }
    fn snapshot(pages: Vec<OrderHistoryPage>, max: u32) -> Result<OrderHistorySnapshot, String> {
        OrderHistorySnapshot::new(
            "account".into(),
            "buff-user".into(),
            "generation".into(),
            OrderHistoryRequest {
                max_pages: max,
                page_size: 2,
            },
            pages,
            10,
            11,
        )
    }
    #[test]
    fn buff_orders_numeric_terminal_and_expired_text_are_not_verified_payment() {
        for state in [
            OrderStateCode::Number(3),
            OrderStateCode::Number(4),
            OrderStateCode::Number(5),
            OrderStateCode::Text("FINISHED".into()),
            OrderStateCode::Text("FUTURE_STATE".into()),
        ] {
            let order = ObservedOrder {
                order_id: "a".into(),
                state_code: Some(state),
                pay_expire_timeout_secs: Some(30),
            };
            assert_eq!(order.payment_evidence(), PaymentEvidence::Unknown);
        }
        for timeout in [None, Some(-1)] {
            let order = ObservedOrder {
                order_id: "a".into(),
                state_code: Some(OrderStateCode::Text("PAYING".into())),
                pay_expire_timeout_secs: timeout,
            };
            assert_eq!(order.payment_evidence(), PaymentEvidence::Unknown);
        }
    }
    #[test]
    fn buff_orders_page_limit_and_short_empty_do_not_prove_absence() {
        let s = snapshot(vec![page(1, &["a", "b"], Some(20))], 1).unwrap();
        assert_eq!(s.stop_reason, HistoryStop::PageBudget);
        let missing = s.lookup("c", 100).unwrap();
        assert!(!missing.found && !missing.execution_allowed);
        assert_eq!(missing.payment_evidence, PaymentEvidence::Unknown);
        let empty = snapshot(vec![page(1, &[], Some(0))], 2).unwrap();
        assert_eq!(empty.stop_reason, HistoryStop::ShortPage);
        assert!(!empty.complete);
        let unknown = snapshot(vec![page(1, &[], None)], 2).unwrap();
        assert!(!unknown.complete);
    }
    #[test]
    fn buff_orders_pagination_rejects_duplicates_total_drift_inconsistent_end_and_gap() {
        for pages in [
            vec![page(1, &["a", "b"], Some(4)), page(2, &["b", "c"], Some(4))],
            vec![page(1, &["a", "b"], Some(4)), page(2, &["c", "d"], Some(5))],
            vec![page(1, &["a"], Some(20))],
            vec![page(2, &["a", "b"], Some(2))],
            vec![page(1, &["a", "b"], Some(2)), page(2, &[], Some(2))],
            vec![page(1, &["a", "b"], None)],
        ] {
            assert!(snapshot(pages, 2).is_err());
        }
    }
}
