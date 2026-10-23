/** API v1. Money is CNY cents; ratios are basis points; dates are Unix seconds.
 * Runtime validation belongs to Rust. This declaration is for frontend consumers.
 */
export type SourceKind = 'demo' | 'steamdt';
export interface ScanRequest {
  source: SourceKind; max_pages: number; page_size: number; max_items: number;
  min_price_cents: number; max_price_cents: number; min_volume: number;
  max_ratio_bps: number; exclude_keywords: string[];
}
export interface MarketItem {
  key: string; name: string; goods_id: number | null; cash_cents: number;
  steam_gross_cents: number; volume: number; observed_at: number;
}
export interface Candidate {
  item: MarketItem; passed: boolean; estimated_wallet_cents: number;
  ratio_bps: number | null; rules: {rule:string; passed:boolean; reason:string}[];
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
}
export interface PlanPreview {
  id:string; created_at:number; status:'preview_only'; algorithm:'ratio_greedy_v1';
  source:SourceKind; source_coverage:Coverage; source_stop_reason:string; request:PlanRequest;
  lines:{candidate:Candidate; quantity:number; cash_total_cents:number; estimated_wallet_total_cents:number}[];
  skipped:{key:string; reason:string}[];
  cash_total_cents:number; remaining_budget_cents:number; estimated_wallet_total_cents:number;
  target_met:boolean|null; execution_blockers:string[];
}
export class ApiError extends Error { status:number; constructor(message:string, status:number); }
export const parallax: {
  health():Promise<{name:string; version:string; api_version:string; capabilities:Record<string,boolean>}>;
  scans():Promise<{items:Omit<ScanReport,'candidates'>[]}>;
  scan(id:string):Promise<ScanReport>;
  startScan(request:Partial<ScanRequest>):Promise<ScanReport>;
  cancelScan(id:string):Promise<{status:string; message:string}>;
  plans():Promise<{items:PlanPreview[]}>;
  plan(id:string):Promise<PlanPreview>;
  createPlan(request:PlanRequest):Promise<PlanPreview>;
};
