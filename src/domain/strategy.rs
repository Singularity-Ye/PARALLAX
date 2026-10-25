//! Versioned purchase research. Decisions describe saved evidence, never authorize orders.
use super::{history::HistorySnapshot, planning::PlanPreview, positions::remaining_units};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategyDraft {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "ResearchObjective::is_wallet")]
    pub objective: ResearchObjective,
    pub name: String,
    pub description: String,
    pub rules: Vec<StrategyRule>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchObjective {
    #[default]
    WalletConversion,
    Holding,
}
impl ResearchObjective {
    fn is_wallet(&self) -> bool {
        *self == Self::WalletConversion
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CvBasis {
    Raw,
    IqrCleaned,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "module", rename_all = "snake_case", deny_unknown_fields)]
pub enum StrategyRule {
    StrictRatio {
        max_ratio_bps: u32,
    },
    PositionCapacity,
    HistoryCoverage {
        min_usable_days: u32,
        max_missing_days: u32,
        max_age_secs: u64,
        max_reference_lag_days: u32,
    },
    SampleCv {
        basis: CvBasis,
        max_cv_bps: u32,
    },
    Trend7 {
        min_points: u32,
        min_slope_bps_per_day: i64,
        max_slope_bps_per_day: i64,
        min_r_squared_bps: u32,
    },
    CalendarP10 {
        min_pairs: u32,
        min_return_bps: i64,
        slippage_bps: u32,
    },
    StressExitRatio {
        max_ratio_bps: u32,
        parameters: super::exit_valuation::ExitValuationPolicy,
    },
}
impl StrategyRule {
    pub fn key(&self) -> &'static str {
        match self {
            Self::StrictRatio { .. } => "strict_ratio",
            Self::PositionCapacity => "position_capacity",
            Self::HistoryCoverage { .. } => "history_coverage",
            Self::SampleCv { .. } => "sample_cv",
            Self::Trend7 { .. } => "trend7",
            Self::CalendarP10 { .. } => "calendar_p10",
            Self::StressExitRatio { .. } => "stress_exit_ratio",
        }
    }
    fn rank(&self) -> u8 {
        match self {
            Self::StrictRatio { .. } => 0,
            Self::PositionCapacity => 1,
            Self::HistoryCoverage { .. } => 2,
            Self::SampleCv { .. } => 3,
            Self::Trend7 { .. } => 4,
            Self::CalendarP10 { .. } => 5,
            Self::StressExitRatio { .. } => 6,
        }
    }
}
impl StrategyDraft {
    pub fn validate(&self) -> Result<(), String> {
        if ![1, 2, 3].contains(&self.schema_version)
            || self.name.trim().is_empty()
            || self.name.len() > 120
            || self.description.len() > 2000
            || self.name.chars().any(char::is_control)
            || self.rules.is_empty()
            || self.rules.len() > if self.schema_version == 1 { 6 } else { 7 }
        {
            return Err(
                "策略需schema_version=1（至多6规则）或2/3（至多7规则）、1–120字节名称及至多2000字节说明"
                    .into(),
            );
        }
        if self.objective == ResearchObjective::Holding {
            if self.schema_version != 3
                || !self
                    .rules
                    .iter()
                    .any(|r| matches!(r, StrategyRule::HistoryCoverage { .. }))
                || self.rules.iter().any(|r| {
                    matches!(
                        r,
                        StrategyRule::StrictRatio { .. } | StrategyRule::StressExitRatio { .. }
                    )
                })
            {
                return Err(
                    "持有观察需要schema_version=3与历史覆盖规则，不能用钱包折算率判定持有价值"
                        .into(),
                );
            }
        } else if !matches!(self.rules.first(), Some(StrategyRule::StrictRatio { .. })) {
            return Err("购入研究必须以 strict_ratio 开始".into());
        }
        let mut previous = None;
        let mut coverage = false;
        for rule in &self.rules {
            if previous.is_some_and(|p| p >= rule.rank()) {
                return Err(
                    "规则不可重复，须按折算率、仓位、历史覆盖、CV、趋势、P10、压力净卖出顺序排列"
                        .into(),
                );
            }
            previous = Some(rule.rank());
            let valid = match rule {
                StrategyRule::StressExitRatio {
                    max_ratio_bps,
                    parameters,
                } => {
                    parameters.validate()?;
                    self.schema_version >= 2 && coverage && (1..=20000).contains(max_ratio_bps)
                }
                StrategyRule::StrictRatio { max_ratio_bps } => (1..=20000).contains(max_ratio_bps),
                StrategyRule::PositionCapacity => true,
                StrategyRule::HistoryCoverage {
                    min_usable_days,
                    max_missing_days,
                    max_age_secs,
                    max_reference_lag_days,
                } => {
                    coverage = true;
                    (2..=365).contains(min_usable_days)
                        && *max_missing_days <= 365
                        && (60..=86400).contains(max_age_secs)
                        && (1..=30).contains(max_reference_lag_days)
                }
                StrategyRule::SampleCv { max_cv_bps, .. } => {
                    coverage && (1..=100000).contains(max_cv_bps)
                }
                StrategyRule::Trend7 {
                    min_points,
                    min_slope_bps_per_day,
                    max_slope_bps_per_day,
                    min_r_squared_bps,
                } => {
                    coverage
                        && (3..=7).contains(min_points)
                        && *min_slope_bps_per_day >= -10000
                        && *max_slope_bps_per_day <= 10000
                        && min_slope_bps_per_day <= max_slope_bps_per_day
                        && *min_r_squared_bps <= 10000
                }
                StrategyRule::CalendarP10 {
                    min_pairs,
                    min_return_bps,
                    slippage_bps,
                } => {
                    coverage
                        && (1..=358).contains(min_pairs)
                        && (-10000..=100000).contains(min_return_bps)
                        && *slippage_bps <= 10000
                }
            };
            if !valid {
                return Err(format!(
                    "{} 参数越界或缺少前置 history_coverage",
                    rule.key()
                ));
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyVersion {
    pub strategy_id: String,
    pub revision: u32,
    pub created_at: u64,
    pub draft: StrategyDraft,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StrategyEvaluationRequest {
    pub strategy_id: String,
    pub revision: u32,
    pub plan_id: String,
    pub account_id: Option<String>,
    #[serde(default)]
    pub history_snapshot_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order_book_snapshot_ids: Vec<String>,
}
impl StrategyEvaluationRequest {
    pub fn validate(&self) -> Result<(), String> {
        let valid_id = |s: &str| !s.is_empty() && s.len() <= 64 && !s.chars().any(char::is_control);
        let mut seen = std::collections::HashSet::new();
        let mut books = std::collections::HashSet::new();
        if !valid_id(&self.strategy_id)
            || !valid_id(&self.plan_id)
            || self.revision == 0
            || self.account_id.as_deref().is_some_and(|a| !valid_id(a))
            || self.history_snapshot_ids.len() > 100
            || self
                .history_snapshot_ids
                .iter()
                .any(|id| !valid_id(id) || !seen.insert(id))
            || (!self.history_snapshot_ids.is_empty() && self.account_id.is_none())
            || self.order_book_snapshot_ids.len() > 100
            || self
                .order_book_snapshot_ids
                .iter()
                .any(|id| !valid_id(id) || !books.insert(id))
        {
            return Err(
                "需明确策略版本、计划及有效引用；历史最多 100 份唯一快照并指定所属账户".into(),
            );
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchStatus {
    Pass,
    Fail,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuleDecision {
    pub module: String,
    pub status: ResearchStatus,
    pub reason: String,
    pub observed: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemDecision {
    pub item_key: String,
    pub status: ResearchStatus,
    pub decisions: Vec<RuleDecision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<super::strategy_review::DecisionReview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_valuation: Option<super::exit_valuation::ExitValuation>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyEvaluation {
    pub id: String,
    pub created_at: u64,
    pub algorithm: String,
    pub status: ResearchStatus,
    pub preview_only: bool,
    pub execution_allowed: bool,
    pub request: StrategyEvaluationRequest,
    pub strategy: StrategyVersion,
    /// Embed immutable inputs: later edits/reads cannot silently change the result.
    pub plan: PlanPreview,
    pub histories: Vec<HistorySnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub order_books: Vec<super::orderbook::OrderBookSnapshot>,
    pub items: Vec<ItemDecision>,
    pub execution_blockers: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StrategyEvaluationSummary {
    pub id: String,
    pub created_at: u64,
    pub strategy_id: String,
    pub strategy_name: String,
    pub revision: u32,
    pub plan_id: String,
    pub status: ResearchStatus,
    pub item_count: usize,
}
impl From<StrategyEvaluation> for StrategyEvaluationSummary {
    fn from(e: StrategyEvaluation) -> Self {
        Self {
            id: e.id,
            created_at: e.created_at,
            strategy_id: e.strategy.strategy_id,
            strategy_name: e.strategy.draft.name,
            revision: e.strategy.revision,
            plan_id: e.plan.id,
            status: e.status,
            item_count: e.items.len(),
        }
    }
}
fn aggregate(values: impl Iterator<Item = ResearchStatus>) -> ResearchStatus {
    let values: Vec<_> = values.collect();
    if values.contains(&ResearchStatus::Fail) {
        ResearchStatus::Fail
    } else if values.is_empty() || values.contains(&ResearchStatus::Unknown) {
        ResearchStatus::Unknown
    } else {
        ResearchStatus::Pass
    }
}
fn decision(module: &str, outcome: Option<bool>, reason: &str, observed: Value) -> RuleDecision {
    RuleDecision {
        module: module.into(),
        status: match outcome {
            Some(true) => ResearchStatus::Pass,
            Some(false) => ResearchStatus::Fail,
            None => ResearchStatus::Unknown,
        },
        reason: reason.into(),
        observed,
    }
}
fn fresh(observed: u64, at: u64, age: u64) -> bool {
    observed <= at && at - observed <= age
}

pub fn evaluate(
    strategy: StrategyVersion,
    plan: PlanPreview,
    histories: Vec<HistorySnapshot>,
    request: StrategyEvaluationRequest,
    at: u64,
) -> Result<StrategyEvaluation, String> {
    evaluate_with_order_books(strategy, plan, histories, vec![], request, at)
}
pub fn evaluate_with_order_books(
    strategy: StrategyVersion,
    plan: PlanPreview,
    histories: Vec<HistorySnapshot>,
    order_books: Vec<super::orderbook::OrderBookSnapshot>,
    request: StrategyEvaluationRequest,
    at: u64,
) -> Result<StrategyEvaluation, String> {
    strategy.draft.validate()?;
    request.validate()?;
    if strategy.strategy_id != request.strategy_id
        || strategy.revision != request.revision
        || plan.id != request.plan_id
        || plan.status != "preview_only"
        || plan.lines.len() > 100
    {
        return Err("策略版本或计划引用不匹配，且只接受预算预览".into());
    }
    if plan.source == super::selection::SourceKind::Demo
        && (!histories.is_empty() || !order_books.is_empty())
    {
        return Err("演示计划不能引用真实平台历史或盘口".into());
    }
    for account in plan
        .request
        .position_snapshot
        .as_ref()
        .map(|p| &p.account_id)
        .into_iter()
        .chain(plan.request.buff_account_id.as_ref())
    {
        if request.account_id.as_ref() != Some(account) {
            return Err("研究账户必须与计划内仓位/BUFF 账户一致".into());
        }
    }
    let mut seen = std::collections::HashSet::new();
    if histories.len() != request.history_snapshot_ids.len()
        || histories.iter().any(|h| {
            !request.history_snapshot_ids.contains(&h.id)
                || request.account_id.as_ref() != Some(&h.account_id)
                || h.request.scan_id != plan.request.scan_id
                || !plan
                    .lines
                    .iter()
                    .any(|l| l.candidate.item.key == h.request.item_key)
                || !seen.insert(&h.request.item_key)
        })
    {
        return Err("历史快照必须显式引用并匹配账户、扫描和计划商品，每件仅一份".into());
    }
    let mut book_items = std::collections::HashSet::new();
    if order_books.len() != request.order_book_snapshot_ids.len()
        || (!order_books.is_empty()
            && !strategy
                .draft
                .rules
                .iter()
                .any(|r| matches!(r, StrategyRule::StressExitRatio { .. })))
        || order_books.iter().any(|b| {
            !request.order_book_snapshot_ids.contains(&b.id)
                || !plan
                    .lines
                    .iter()
                    .any(|l| l.candidate.item.key == b.request.market_hash_name)
                || !book_items.insert(&b.request.market_hash_name)
        })
    {
        return Err(
            "出售盘口需显式引用并匹配计划商品，每件一份且仅供启用的压力净卖出规则使用".into(),
        );
    }
    let mut items = vec![];
    for line in &plan.lines {
        let item = &line.candidate.item;
        let history = histories.iter().find(|h| h.request.item_key == item.key);
        let mut exit_valuation = None;
        let mut decisions = vec![decision(
            "quote_freshness",
            if fresh(item.observed_at, at, plan.request.max_quote_age_secs)
                && fresh(plan.created_at, at, plan.request.max_quote_age_secs)
            {
                Some(true)
            } else {
                None
            },
            "采用计划原始报价时间；过期须重新获取报价和生成计划",
            json!({"observed_at":item.observed_at,"plan_created_at":plan.created_at,"max_age_secs":plan.request.max_quote_age_secs}),
        )];
        if let Some(fx) = &plan.fx {
            decisions.push(decision(
                "fx_freshness",
                fx.validate(at).ok().map(|_| true),
                "沿用计划的汇率证据；过期不在研究中静默刷新",
                json!({"snapshot_id":fx.id,"fetched_at":fx.fetched_at}),
            ));
        }
        let mut covered = false;
        for rule in &strategy.draft.rules {
            let result = match rule {
                StrategyRule::StrictRatio { max_ratio_bps } => decision(
                    rule.key(),
                    (item.cash_cents > 0 && line.candidate.estimated_wallet_cents > 0).then(|| {
                        u128::from(item.cash_cents) * 10000
                            < u128::from(line.candidate.estimated_wallet_cents)
                                * u128::from(*max_ratio_bps)
                    }),
                    "现金单价 / 计划费用模型估算净钱包严格小于阈值；整数交叉乘法，不用显示舍入值比较",
                    json!({"cash_cents":item.cash_cents,"estimated_wallet_cents":line.candidate.estimated_wallet_cents,"max_ratio_bps":max_ratio_bps}),
                ),
                StrategyRule::PositionCapacity => match plan.request.position_snapshot.as_ref() {
                    Some(p) if fresh(p.observed_at, at, plan.request.max_position_age_secs) => {
                        match p.entries.iter().find(|e| e.key == item.key) {
                            Some(entry) => match remaining_units(
                                item.cash_cents,
                                plan.request.max_units_per_item,
                                Some(entry),
                                plan.request.liquidity_policy.as_ref(),
                            ) {
                                Ok(cap) => decision(
                                    rule.key(),
                                    Some(line.quantity > 0 && line.quantity <= cap),
                                    "复用剩余持仓与日量规则；手工仓位仍未完成账户对账",
                                    json!({"remaining_units":cap,"planned_quantity":line.quantity,"account_id":p.account_id,"observed_at":p.observed_at}),
                                ),
                                Err(reason) => decision(rule.key(), None, &reason, Value::Null),
                            },
                            None => decision(
                                rule.key(),
                                None,
                                "仓位未覆盖此商品，不能按零持仓通过",
                                Value::Null,
                            ),
                        }
                    }
                    _ => decision(rule.key(), None, "缺少有效期内的仓位快照", Value::Null),
                },
                StrategyRule::HistoryCoverage {
                    min_usable_days,
                    max_missing_days,
                    max_age_secs,
                    max_reference_lag_days,
                } => {
                    if let Some(h) = history.filter(|h| {
                        h.source == "steam_authenticated_pricehistory"
                            && h.identity_binding == "scan_candidate_and_request_parameters"
                            && h.series.currency.is_some()
                            && h.series.currency_basis == "explicit_code_or_unique_marker"
                            && h.started_at <= h.observed_at
                            && fresh(h.observed_at, at, *max_age_secs)
                            && h.series.until_utc_day_exclusive == at / 86400
                    }) {
                        if let Some(s) = h
                            .analysis
                            .stability
                            .as_ref()
                            .filter(|s| s.algorithm == "calendar_stability_v1")
                        {
                            let lag = s
                                .reference_utc_day
                                .and_then(|day| (at / 86400).checked_sub(day));
                            let pass = s.usable_days >= *min_usable_days as usize
                                && s.missing_or_unusable_days <= *max_missing_days
                                && lag.is_some_and(|d| {
                                    d >= 1 && d <= u64::from(*max_reference_lag_days)
                                });
                            covered = pass;
                            decision(
                                rule.key(),
                                Some(pass),
                                "按结束的 UTC 自然日校验有效样本、缺口和最后有效日",
                                json!({"snapshot_id":h.id,"observed_at":h.observed_at,"currency":h.series.currency,"usable_days":s.usable_days,"missing_days":s.missing_or_unusable_days,"reference_lag_days":lag}),
                            )
                        } else {
                            decision(
                                rule.key(),
                                None,
                                "旧快照缺少受支持的稳定性指标；回看不重算",
                                Value::Null,
                            )
                        }
                    } else {
                        decision(
                            rule.key(),
                            None,
                            "历史缺失、币种/来源不明、过期或未覆盖当前结束日；不猜测安全",
                            Value::Null,
                        )
                    }
                }
                _ if !covered => decision(
                    rule.key(),
                    None,
                    "历史覆盖门未通过，不能据此放行指标规则",
                    Value::Null,
                ),
                StrategyRule::SampleCv { basis, max_cv_bps } => {
                    let s = history.unwrap().analysis.stability.as_ref().unwrap();
                    let cv = match basis {
                        CvBasis::Raw => s.raw_sample_cv_bps,
                        CvBasis::IqrCleaned => s.clean_sample_cv_bps,
                    };
                    decision(
                        rule.key(),
                        cv.map(|v| v <= u64::from(*max_cv_bps)),
                        "所选样本 CV 不高于上限；清洗后结果不替代原始尾部风险",
                        json!({"basis":basis,"cv_bps":cv,"max_cv_bps":max_cv_bps,"raw_cv_bps":s.raw_sample_cv_bps,"excluded_days":s.iqr.excluded_utc_days}),
                    )
                }
                StrategyRule::Trend7 {
                    min_points,
                    min_slope_bps_per_day,
                    max_slope_bps_per_day,
                    min_r_squared_bps,
                } => {
                    let trend = &history.unwrap().analysis.stability.as_ref().unwrap().trend7;
                    let outcome = if trend.usable_days < *min_points as usize {
                        None
                    } else {
                        trend
                            .relative_slope_bps_per_day
                            .zip(trend.r_squared_bps)
                            .map(|(slope, r2)| {
                                slope >= *min_slope_bps_per_day
                                    && slope <= *max_slope_bps_per_day
                                    && r2 >= *min_r_squared_bps
                            })
                    };
                    decision(
                        rule.key(),
                        outcome,
                        "近七个自然日的相对斜率范围与 R² 门槛；常量序列 R² 未定义",
                        json!({"trend":trend,"min_points":min_points,"min_slope_bps_per_day":min_slope_bps_per_day,"max_slope_bps_per_day":max_slope_bps_per_day,"min_r_squared_bps":min_r_squared_bps}),
                    )
                }
                StrategyRule::CalendarP10 {
                    min_pairs,
                    min_return_bps,
                    slippage_bps,
                } => {
                    let risk = history.unwrap().analysis.risk.as_ref();
                    let result = risk
                        .filter(|r| {
                            r.algorithm == "calendar_p10_v1"
                                && r.sample_count >= *min_pairs as usize
                                && r.slippage_bps == *slippage_bps
                        })
                        .and_then(|r| r.return_p10_bps)
                        .map(|p10| p10 >= *min_return_bps);
                    decision(
                        rule.key(),
                        result,
                        "七自然日历史收益 P10 下限；需足够配对与相同压力滑点，P10 不是未来损失保证",
                        json!({"risk":risk,"min_pairs":min_pairs,"min_return_bps":min_return_bps,"required_slippage_bps":slippage_bps}),
                    )
                }
                StrategyRule::StressExitRatio {
                    max_ratio_bps,
                    parameters,
                } => {
                    let book = order_books
                        .iter()
                        .find(|b| b.request.market_hash_name == item.key);
                    let valuation = (|| {
                        let b = book.ok_or("缺少此商品的显式出售盘口".to_string())?;
                        b.native_ask(at, parameters.max_book_age_secs)?;
                        let h = history.ok_or("缺少历史证据")?;
                        let currency = h.series.currency.as_ref().ok_or("历史币种未知")?;
                        let risk = h.analysis.risk.as_ref().ok_or("历史P10未知")?;
                        super::exit_valuation::estimate(
                            &b.book,
                            currency,
                            risk,
                            parameters,
                            plan.fx.as_ref(),
                            at,
                        )
                    })();
                    match valuation {
                        Ok(v) => {
                            let outcome = (item.cash_cents > 0).then(|| {
                                u128::from(item.cash_cents) * 10000
                                    < u128::from(v.stressed_net_cny.minor)
                                        * u128::from(*max_ratio_bps)
                            });
                            let result = decision(
                                rule.key(),
                                outcome,
                                "现金单价 / 压力净卖出估值严格小于阈值；先对当前壁垒参考施加历史跌幅/一次滑点，再扣原币费用和换算",
                                json!({"book_snapshot_id":book.unwrap().id,"history_snapshot_id":history.unwrap().id,"cash_cents":item.cash_cents,"stress_net_cny_cents":v.stressed_net_cny.minor,"max_ratio_bps":max_ratio_bps}),
                            );
                            exit_valuation = Some(v);
                            result
                        }
                        Err(reason) => decision(rule.key(), None, &reason, Value::Null),
                    }
                }
            };
            decisions.push(result);
        }
        items.push(ItemDecision {
            item_key: item.key.clone(),
            status: aggregate(decisions.iter().map(|d| d.status)),
            review: Some(super::strategy_review::review(&strategy.draft, &plan, line)),
            decisions,
            exit_valuation,
        });
    }
    let status = aggregate(items.iter().map(|i| i.status));
    let mut blockers = plan.execution_blockers.clone();
    blockers.push(
        "策略研究只解释已保存的计划条目，未重新扫描、分配预算或获取行情；通过不代表允许交易".into(),
    );
    blockers.push("真实平台、费用结算与完整持仓未验收；预检、订单和对账尚未接入".into());
    Ok(StrategyEvaluation {
        id: uuid::Uuid::new_v4().to_string(),
        created_at: at,
        algorithm: if strategy.draft.schema_version == 3 {
            "purchase_research_v3"
        } else if strategy.draft.schema_version == 2 {
            "purchase_research_v2"
        } else {
            "purchase_research_v1"
        }
        .into(),
        status,
        preview_only: true,
        execution_allowed: false,
        request,
        strategy,
        plan,
        histories,
        order_books,
        items,
        execution_blockers: blockers,
    })
}

#[cfg(test)]
#[path = "strategy_tests.rs"]
pub(crate) mod tests;
