# UI data and motion maintenance

2026-11-02. Integrated from `f4b76e3`; no new frontend package or runtime dependency.

## Ledger: facts stay in the backend

`web/ledger.js` loads the account-scoped `ledgerSummary` and `ledgerRows` APIs. `web/lib/ledger-view.js` only formats the projection. It never infers market prices from cost, computes Steam fees, converts currencies or invents profit. Row cost covers the row quantity, including aggregated pending receipts; asset state comes from each backend row. Unavailable metrics show **未知**, with the backend reason as the cell title. Summary counts distinguish held/listed/pending/sold; open cost is not historical total spend. Wallet receipts retain their native currency (including UAH).

Rows are paged at 50, with explicit previous/next controls. Refresh failures preserve the current account's last projection and show an error. Switching accounts clears that projection immediately. Summary and rows remain separate local reads, not an atomic cross-request snapshot. This is reported manual ledger data, not complete platform inventory or verified market valuation.

Quick registration keeps an identical request/client_key after uncertain failure. Decimal input is converted to integer minor units before multiplying quantity. A successful request clears that retry identity. Unsupported batch listing/deletion controls are disabled and labelled; no execution API was introduced.

## Motion: one lifecycle, separate effects

`shell.js` mounts the five existing effects through `initMotion` in `web/lib/motion.js`. Each effect returns a disposer; `motionScope` owns listeners, animation frames and DOM cleanup. Route changes, hidden pages, pagehide and runtime reduced-motion preference changes dispose effects. Visible/enabled pages rebuild them without duplicate canvas, SVG or button wrappers.

The existing magnet and page-curl reuse `springStep` and `frameClock`: real rAF timestamps, delayed frames clamped to 50 ms, integration substeps at most 1/120 s, resting springs stop requesting frames. Magnet and spotlight coalesce pointer updates per animation frame. Magnet uses the independent CSS `translate` property so it does not replace hover rotation/scale, and all previous targets return to rest. ClickSpark caps canvas DPR at 2. These are resource controls, **not measured performance or zero-memory claims**.

HoldButton only draws charging feedback. It never calls click/requestSubmit or dispatches a second business activation. Short, intermediate, long and keyboard activation use the same native click/submit path, at release for pointer interactions. Reduced motion leaves native activation available.

## Styles

Shared button defaults now use low-specificity `:where(...)`; excluded close/navigation buttons are excluded from hover/active too. Secondary/compact/account styles no longer require many escalation overrides. Shared cards and account/Guard surfaces use existing border/shadow tokens. Permanent list-wide will-change, obsolete hold wrapper/jitter styles and an overlapping origami base rule were removed. Common font sizes reuse theme tokens; Guard expiry text does not pulse.

This is a bounded cleanup, **not a completed whole-site design-system migration**. Legacy selectors, inline layout declarations and some `!important` remain. Keep required hidden/reduced-motion invariants; do not blanket-delete all important declarations. Preserve the current visual direction; avoid appending a new override block for every page.

## Verification

- `cargo test --locked --offline`: 224 passed, 0 failed, 2 ignored on host. The existing loopback CONNECT fixture stalled inside the sandbox; host suite passed.
- `node --test tools/tests/*.test.mjs`: 9 passed, covering projection rendering, unknown/zero, spring time consistency and lifecycle ownership.
- `node tools/check-ui-architecture.mjs`: independent headless Edge + local synthetic API. Covers pagination, failed refresh preservation, account isolation, exact idempotent retry, short/medium/long/keyboard native submission, all five effects and runtime reduced-motion/route remount cleanup. Requires installed Playwright; `PARALLAX_PLAYWRIGHT_MODULE` may point to an existing installation. No dependency installation and no real account writes.
- Browser smoke is functional regression, not a complete visual or accessibility audit, nor a frame-time benchmark. Live market/auth/proxy acceptance remains separate.
