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
  buff_snapshot_ids?:string[];buff_account_id?:string|null;
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
  buff_valuations?:{snapshot_id:string;account_id:string;credential_generation:string;item_key:string;goods_id:number;observed_at:number;unit_cash_cents:number|null;lowest_tier_visible_units:number}[];
}
export interface BuffQuoteSnapshot {
  id:string;account_id:string;credential_generation:string;
  request:{scan_id:string;item_key:string};observed_at:number;currency:'CNY';currency_basis:'buff_cny_protocol_not_live_verified';
  source:'buff_sell_orders_readonly';coverage:'first_page_lowest_tier_only';
  page:{goods_id:number;market_hash_name:string;levels:{price_minor:number;quantity:number}[];reported_total:number|null};
  lowest_cash_cents:number|null;lowest_tier_visible_units:number;
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
export interface InventoryRequest {max_pages_per_context?:number;}
export interface InventoryAsset {
  app_id:730;context_id:'2'|'16';asset_id:string;class_id:string;instance_id:string;amount:number;
  market_hash_name:string|null;reported_tradable:boolean;reported_marketable:boolean;restriction_note_present:boolean;
}
export interface InventorySnapshot {
  id:string;account_id:string;steam_id:string;credential_generation:string;request:{max_pages_per_context:number};
  started_at:number;observed_at:number;source:'steam_authenticated_inventory';platform_verified:false;
  coverage:{context_id:'2'|'16';pages_read:number;reported_total:number;observed_assets:number;exhausted:boolean;stop_reason:'page_budget'|'source_exhausted'}[];
  assets:InventoryAsset[];warnings:string[];
}
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
  priceHistories(account:string):Promise<{items:HistorySnapshot[]}>;
  priceHistory(account:string,id:string):Promise<HistorySnapshot>;
  fetchPriceHistory(account:string,value:HistoryRequest):Promise<HistorySnapshot>;
  inventories(account:string):Promise<{items:InventorySnapshot[]}>;
  inventory(account:string,id:string):Promise<InventorySnapshot>;
  fetchInventory(account:string,value:InventoryRequest):Promise<InventorySnapshot>;
  buffQuotes(account:string):Promise<{items:BuffQuoteSnapshot[]}>;
  buffQuote(account:string,id:string):Promise<BuffQuoteSnapshot>;
  fetchBuffQuote(account:string,value:{scan_id:string;item_key:string}):Promise<BuffQuoteSnapshot>;
  orderBooks():Promise<{items:OrderBookSnapshot[]}>;
  orderBook(id:string):Promise<OrderBookSnapshot>;
  fetchOrderBook(value:{app_id:730;market_hash_name:string}):Promise<OrderBookSnapshot>;
  listingPricePreview(id:string,value:ListingPreviewRequest):Promise<ListingPricePreview>;
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
export interface HistoryRequest {scan_id:string;item_key:string;window_days:number;slippage_bps:number;}
export interface WallGapPolicy {currency:'CNY';wall_volume_threshold:number;max_ignore_volume:number;min_lowest_tier_volume:number;min_step_minor:number;min_floor_minor:number;offset_minor:number;}
export interface ListingPreviewRequest {policy:WallGapPolicy;max_quote_age_secs:number;fee_schedule?:{currency:'CNY';platform_bps:number;publisher_bps:number;platform_min_minor:number;publisher_min_minor:number;increment_minor:number}|null;}
export interface ListingPricePreview {
  preview_only:true;snapshot_id:string;market:{app_id:730;market_hash_name:string};source:string;identity_binding:string;
  observed_at:number;computed_at:number;age_secs:number;freshness:'within_selected_age'|'historical_only';
  request:ListingPreviewRequest;
  analysis:{algorithm:'wall_gap_cny_v1';gross_listing_estimate:NativeMoney|null;reason:'empty_asks'|'single_level'|'regular_undercut'|'gap_jump';
    ignored_lowest_tiers:{price_minor:number;quantity:number}[];wall:{level_index:number;price_minor:number;cumulative_quantity:number;reached_threshold:boolean}|null;
    gap_checks:{lower_minor:number;upper_minor:number;blocking_quantity:number;absolute_threshold_minor:number;relative_threshold_bps:number;jumped:boolean}[];
    stopped_by_blocking_volume:boolean;floor_applied:boolean};fee_estimate:FeeQuote|null;warnings:string[];
}
export interface HistorySnapshot {
  id:string;account_id:string;credential_generation:string;request:HistoryRequest;started_at:number;observed_at:number;
  source:'steam_authenticated_pricehistory';identity_binding:'scan_candidate_and_request_parameters';platform_verified:false;
  series:{currency:string|null;currency_basis:'explicit_code_or_unique_marker'|'ambiguous_or_unsupported';
    points:{at:number;price_major_1e4:number;volume:number|null}[];response_rows:number;excluded_before_window:number;excluded_open_day:number;from_utc_day:number;until_utc_day_exclusive:number};
  analysis:{status:'unknown_currency'|'no_usable_daily_prices'|'insufficient_pairs'|'historical_pairs_available';
    daily_bars:{utc_day:number;vwap:NativeMoney|null;volume:number;zero_volume_samples:number;missing_volume_samples:number}[];
    missing_calendar_days:number;below_minor_unit_days:number;reference_utc_day:number|null;
    risk:{algorithm:'calendar_p10_v1';sample_count:number;return_p10_bps:number|null;conservative_gross:NativeMoney|null;slippage_bps:number}|null;warnings:string[]};
}
