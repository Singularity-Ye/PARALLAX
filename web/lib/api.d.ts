/** API v1. Scan/plan money is CNY cents; public books carry their native currency.
 * Ratios are basis points; dates are Unix seconds.
 * Runtime validation belongs to Rust. This declaration is for frontend consumers.
 */
export type SourceKind = 'demo' | 'steamdt';
export interface ScanRequest {
  source: SourceKind; max_pages: number; page_size: number; max_items: number;
  min_price_cents: number; max_price_cents: number; min_volume: number;
  max_ratio_bps: number; exclude_keywords: string[];
  wallet_policy:WalletPolicy;
}
export type WalletPolicy = {kind:'legacy115'} | {kind:'parameterized_fees';schedule:{currency:'CNY';platform_bps:number;publisher_bps:number;platform_min_minor:number;publisher_min_minor:number;increment_minor:number}};
export interface FeeQuote {net:{minor:number;currency:string};fees_minor:number;gross_used_minor:number;unallocated_gross_minor:number;}
export interface MarketItem {
  key: string; name: string; goods_id: number | null; cash_cents: number;
  steam_gross_cents: number; volume: number; observed_at: number;
}
export interface Candidate {
  item: MarketItem; passed: boolean; estimated_wallet_cents: number;
  ratio_bps: number | null; rules: {rule:string; passed:boolean; reason:string}[];
  valuation?:FeeQuote;
}
export interface Coverage {
  pages_requested:number; pages_received:number; rows_received:number;
  duplicates:number; unique_received:number; examined:number;
  accepted:number; rejected:number; unexamined_received:number;
}
export interface ScanReport {
  id:string; request:ScanRequest;
  status:'running'|'completed'|'failed'|'cancelled'|'interrupted';
  stop_reason:string; source_exhausted:boolean; created_at:number;
  finished_at:number|null; coverage:Coverage; candidates:Candidate[]; error:string|null;
}
export interface PlanRequest {
  scan_id:string; budget_cents:number; target_wallet_cents:number|null;
  max_units_per_item:number; max_positions:number; max_quote_age_secs:number;
  position_snapshot?:PositionSnapshot|null; liquidity_policy?:LiquidityPolicy|null;max_position_age_secs?:number;
  steam_snapshot_ids?:string[];
}
export interface PositionSnapshot {account_id:string;observed_at:number;reserved_cash_cents:number;entries:{key:string;held_units:number;pending_buy_units:number;available_units:number|null;daily_volume:number|null}[];}
export interface LiquidityPolicy {participation_bps:number;low_price_threshold_cents:number;low_price_penalty_bps:number;low_price_cap:number;}
export interface PlanPreview {
  id:string; created_at:number; status:'preview_only'; algorithm:'ratio_greedy_v1'|'ratio_greedy_v2';
  source:SourceKind; source_coverage:Coverage; source_stop_reason:string; request:PlanRequest;
  lines:{candidate:Candidate; quantity:number; cash_total_cents:number; estimated_wallet_total_cents:number;max_additional_units?:number}[];
  skipped:{key:string; reason:string}[];
  cash_total_cents:number; remaining_budget_cents:number; estimated_wallet_total_cents:number;
  target_met:boolean|null; execution_blockers:string[];
  reserved_cash_cents?:number;wallet_policy?:WalletPolicy;position_source?:'manual_unverified';
  steam_valuations?:{snapshot_id:string;market_hash_name:string;observed_at:number;gross_listing_estimate_cents:number;best_bid_cents:number|null}[];
}
export interface NativeMoney {minor:number;currency:string;}
export interface OrderBookSnapshot {
  id:string;request:{app_id:number;market_hash_name:string};observed_at:number;
  source:'steam_public_query_action';identity_binding:'request_parameters';requested_currency:string;
  book:{currency:string;asks:{price_minor:number;quantity:number}[];bids:{price_minor:number;quantity:number}[];reported_sell_orders:number|null;reported_buy_orders:number|null};
  summary:{best_ask:NativeMoney|null;best_bid:NativeMoney|null;spread_minor:number|null;visible_ask_quantity:number;visible_bid_quantity:number};
  warnings:string[];
}
export class ApiError extends Error { status:number; constructor(message:string, status:number); }
export type Platform = 'steam' | 'buff';
export interface Account { id:string; label:string; steam_id:string; created_at:number; }
export interface SessionView {
  platform:Platform; generation:string;
  status:'unverified'|'valid'|'invalid'|'unknown'|'rate_limited'|'verification_required'|'missing';
  checked_at:number|null; retry_at:number; message:string; subject:string|null;
}
export interface BrowserLogin {
  id:string; account_id:string; platform:Platform;
  status:'pending'|'completed'|'cancelled'|'expired'; expires_at:number;
  message:string; session:SessionView|null;
}
export const parallax: {
  orderBooks():Promise<{items:OrderBookSnapshot[]}>;
  orderBook(id:string):Promise<OrderBookSnapshot>;
  fetchOrderBook(value:{app_id:730;market_hash_name:string}):Promise<OrderBookSnapshot>;
  health():Promise<{name:string; version:string; api_version:string; capabilities:Record<string,boolean>}>;
  accounts():Promise<{items:Account[]}>;
  createAccount(value:{label:string;steam_id:string}):Promise<Account>;
  sessions(id:string):Promise<{items:SessionView[]}>;
  importSession(id:string,platform:Platform,value:{cookies:string;user_agent?:string}):Promise<SessionView>;
  verifySession(id:string,platform:Platform):Promise<SessionView>;
  revokeSession(id:string,platform:Platform):Promise<SessionView>;
  browserLogin():Promise<{item:BrowserLogin|null}>;
  startBrowserLogin(id:string,platform:Platform):Promise<BrowserLogin>;
  finishBrowserLogin(id:string):Promise<BrowserLogin>;
  cancelBrowserLogin(id:string):Promise<BrowserLogin>;
  scans():Promise<{items:Omit<ScanReport,'candidates'>[]}>;
  scan(id:string):Promise<ScanReport>;
  startScan(request:Partial<ScanRequest>):Promise<ScanReport>;
  cancelScan(id:string):Promise<{status:string; message:string}>;
  plans():Promise<{items:PlanPreview[]}>;
  plan(id:string):Promise<PlanPreview>;
  createPlan(request:PlanRequest):Promise<PlanPreview>;
};
