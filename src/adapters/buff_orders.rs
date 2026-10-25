//! Original buy-order history endpoint. Only allowlisted observation fields survive.
use super::buff_read::BuffHttp;
use crate::{
    application::ports::{BuffOrderProvider, BuffReadError as Error},
    domain::{accounts::Credentials, buff_orders::*},
};
use serde_json::Value;

pub struct BuffPurchaseHistory {
    client: BuffHttp,
}
impl BuffPurchaseHistory {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: BuffHttp::new()?,
        })
    }
}
fn server_id(v: &Value) -> Result<String, Error> {
    let id = match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or(Error::InvalidPayload)?
            .to_string(),
        _ => return Err(Error::InvalidPayload),
    };
    if !identifier(&id) {
        return Err(Error::InvalidPayload);
    }
    Ok(id)
}
fn parse(body: &Value, page: u32, size: u32, principal: &str) -> Result<OrderHistoryPage, Error> {
    if let Some(e) = super::buff_read::payload_error(body) {
        return Err(e);
    }
    let data = &body["data"];
    if data
        .get("page_num")
        .is_some_and(|v| v.as_u64() != Some(page as u64))
        || data
            .get("page_size")
            .is_some_and(|v| v.as_u64() != Some(size as u64))
    {
        return Err(Error::InvalidPayload);
    }
    let items = data["items"]
        .as_array()
        .filter(|a| a.len() <= size as usize)
        .ok_or(Error::InvalidPayload)?;
    let mut orders = Vec::with_capacity(items.len());
    for item in items {
        if let Some(buyer) = item.get("buyer_id")
            && server_id(buyer)? != principal
        {
            return Err(Error::IdentityMismatch);
        }
        let state_code = match item.get("state") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(OrderStateCode::Text(s.clone())),
            Some(Value::Number(n)) => Some(OrderStateCode::Number(
                n.as_i64().ok_or(Error::InvalidPayload)?,
            )),
            _ => return Err(Error::InvalidPayload),
        };
        let timeout = match item.get("pay_expire_timeout") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.parse::<i64>().map_err(|_| Error::InvalidPayload)?),
            Some(Value::Number(n)) => Some(n.as_i64().ok_or(Error::InvalidPayload)?),
            _ => return Err(Error::InvalidPayload),
        };
        orders.push(ObservedOrder {
            order_id: server_id(&item["id"])?,
            state_code,
            pay_expire_timeout_secs: timeout,
        });
    }
    let reported_total = data
        .get("total_count")
        .map(|n| n.as_u64().ok_or(Error::InvalidPayload))
        .transpose()?;
    let result = OrderHistoryPage {
        page_num: page,
        page_size: size,
        reported_total,
        orders,
    };
    result
        .validate(page, size)
        .map_err(|_| Error::InvalidPayload)?;
    Ok(result)
}
impl BuffOrderProvider for BuffPurchaseHistory {
    async fn history_page(
        &self,
        page: u32,
        size: u32,
        principal: &str,
        secret: &Credentials,
    ) -> Result<OrderHistoryPage, Error> {
        let body = self
            .client
            .get(
                "/api/market/buy_order/history",
                &[
                    ("game", "csgo".into()),
                    ("page_num", page.to_string()),
                    ("page_size", size.to_string()),
                ],
                "https://buff.163.com/market/buy_order/history?game=csgo",
                secret,
            )
            .await?;
        parse(&body, page, size, principal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn buff_order_parser_projects_fields_and_keeps_unverified_states_unknown() {
        let page=parse(&json!({"code":"OK","data":{"page_num":1,"total_count":2,"items":[
            {"id":"order-1","buyer_id":123,"state":"PAYING","pay_expire_timeout":30,"cookies":"must-not-survive","url":"private-url"},
            {"id":998,"state":3}
        ]}}),1,20,"123").unwrap();
        assert_eq!(
            page.orders[0].payment_evidence(),
            PaymentEvidence::AwaitingPaymentAtObservation
        );
        assert_eq!(page.orders[1].payment_evidence(), PaymentEvidence::Unknown);
        let raw = serde_json::to_string(&page).unwrap();
        assert!(!raw.contains("must-not-survive") && !raw.contains("private-url"));
    }
    #[test]
    fn buff_order_parser_rejects_wrong_identity_shape_and_duplicate_rows() {
        let base = json!({"code":"OK","data":{"items":[{"id":"a","state":"PAYING","pay_expire_timeout":0}]}});
        assert!(parse(&base, 1, 20, "123").is_ok());
        for (key, value) in [
            ("id", json!(true)),
            ("state", json!({"secret":"x"})),
            ("pay_expire_timeout", json!("NaN")),
            ("buyer_id", json!(321)),
        ] {
            let mut v = base.clone();
            v["data"]["items"][0][key] = value;
            assert!(parse(&v, 1, 20, "123").is_err());
        }
        let mut v = base.clone();
        v["data"]["items"] = json!([{"id":"a"},{"id":"a"}]);
        assert!(parse(&v, 1, 20, "123").is_err());
        let mut v = base.clone();
        v["data"]["page_num"] = json!(2);
        assert!(parse(&v, 1, 20, "123").is_err());
        let mut v = base.clone();
        v["code"] = json!("Login Required");
        assert!(matches!(parse(&v, 1, 20, "123"), Err(Error::Expired)));
        let mut v = base;
        v["data"].as_object_mut().unwrap().remove("items");
        assert!(parse(&v, 1, 20, "123").is_err());
    }
}
