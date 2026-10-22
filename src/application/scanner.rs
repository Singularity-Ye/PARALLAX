use crate::{
    application::ports::MarketSource,
    domain::analysis::evaluate,
    domain::selection::{ScanReport, SourceKind, now},
};
use std::{
    collections::HashSet,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

pub async fn scan(
    source: &impl MarketSource,
    mut report: ScanReport,
    cancel: &AtomicBool,
) -> ScanReport {
    let mut seen = HashSet::new();
    report.stop_reason = "page_budget".into();
    for page in 1..=report.request.max_pages {
        if cancel.load(Ordering::Relaxed) {
            report.stop_reason = "cancelled".into();
            break;
        }
        // Pace live source access. No unbounded concurrent marketplace traffic.
        if page > 1 && report.request.source == SourceKind::Steamdt {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if cancel.load(Ordering::Relaxed) {
                report.stop_reason = "cancelled".into();
                break;
            }
        }
        report.coverage.pages_requested += 1;
        let rows = match source.page(&report.request, page).await {
            Ok(rows) => rows,
            Err(error) => {
                report.error = Some(error);
                report.stop_reason = "source_error".into();
                break;
            }
        };
        report.coverage.pages_received += 1;
        report.coverage.rows_received += rows.len();
        let empty = rows.is_empty();
        // A short page is not proof of exhaustion: request the next page within budget.
        for item in rows {
            if !seen.insert(item.key.clone()) {
                report.coverage.duplicates += 1;
                continue;
            }
            report.coverage.unique_received += 1;
            if report.coverage.examined >= report.request.max_items
                || cancel.load(Ordering::Relaxed)
            {
                report.coverage.unexamined_received += 1;
                continue;
            }
            let candidate = evaluate(item, &report.request);
            report.coverage.examined += 1;
            if candidate.passed {
                report.coverage.accepted += 1;
            } else {
                report.coverage.rejected += 1;
            }
            report.candidates.push(candidate);
        }
        if cancel.load(Ordering::Relaxed) {
            report.stop_reason = "cancelled".into();
            break;
        }
        if empty {
            report.source_exhausted = true;
            report.stop_reason = "source_exhausted".into();
            break;
        }
        if report.coverage.examined >= report.request.max_items {
            report.stop_reason = "item_budget".into();
            break;
        }
    }
    report.status = match report.stop_reason.as_str() {
        "source_error" => "failed",
        "cancelled" => "cancelled",
        _ => "completed",
    }
    .into();
    report.finished_at = Some(now());
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        adapters::market::Provider,
        domain::selection::{MarketItem, ScanRequest},
    };
    #[tokio::test]
    async fn scans_beyond_rejected_rows_and_accounts_for_duplicates() {
        let report = scan(
            &Provider::new().unwrap(),
            ScanReport::new(ScanRequest::default()),
            &AtomicBool::new(false),
        )
        .await;
        assert_eq!(report.coverage.rows_received, 13);
        assert_eq!(report.coverage.duplicates, 1);
        assert_eq!(report.coverage.examined, 12);
        assert_eq!(report.coverage.accepted, 7);
        assert_eq!(report.coverage.rejected, 5);
        assert!(report.source_exhausted);
    }
    #[tokio::test]
    async fn budget_is_explicit_and_never_claims_whole_market() {
        let req = ScanRequest {
            max_items: 3,
            ..Default::default()
        };
        let report = scan(
            &Provider::new().unwrap(),
            ScanReport::new(req),
            &AtomicBool::new(false),
        )
        .await;
        assert_eq!(report.stop_reason, "item_budget");
        assert_eq!(report.coverage.examined, 3);
        assert_eq!(report.coverage.unexamined_received, 7);
        assert!(!report.source_exhausted);
        assert!(
            ScanRequest {
                max_items: 0,
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    struct FailsSecondPage;
    impl MarketSource for FailsSecondPage {
        async fn page(&self, _: &ScanRequest, page: usize) -> Result<Vec<MarketItem>, String> {
            if page == 1 {
                Ok(crate::adapters::market::demo_items()
                    .into_iter()
                    .take(1)
                    .collect())
            } else {
                Err("HTTP 429".into())
            }
        }
    }
    #[tokio::test]
    async fn partial_results_survive_upstream_failure() {
        let report = scan(
            &FailsSecondPage,
            ScanReport::new(ScanRequest::default()),
            &AtomicBool::new(false),
        )
        .await;
        assert_eq!(report.status, "failed");
        assert_eq!(report.coverage.examined, 1);
        assert_eq!(report.error.as_deref(), Some("HTTP 429"));
    }
    #[tokio::test]
    async fn cancellation_does_not_fetch() {
        let report = scan(
            &FailsSecondPage,
            ScanReport::new(ScanRequest::default()),
            &AtomicBool::new(true),
        )
        .await;
        assert_eq!(report.status, "cancelled");
        assert_eq!(report.coverage.pages_requested, 0);
    }
}
