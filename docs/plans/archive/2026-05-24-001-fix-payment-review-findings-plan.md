---
name: fix-payment-review-findings
type: fix
status: completed
created: 2026-05-24
---

# Fix: Payment System Code Review Findings

## Problem

Code review of commit `d7b31e9` (DMM opensocial payment protocol) identified 3 P0 and 3 P1 issues across ownership validation, input sanitization, and test coverage. The payment flow works for the happy path but has state-consistency bugs, authorization gaps, and zero handler-level tests.

## Scope

Fix all P0 and P1 findings from the review. P2+ items (TTL/cleanup, response struct cleanup, formatting separation) are deferred.

### In Scope

- cancel_payment ownership validation (P0)
- Negative count/price rejection (P0)
- confirm_payment validate-before-consume pattern (P0)
- payment_html ownership check / IDOR fix (P1)
- Price validation against codex manifest (P1)
- Handler-level tests for payment_create, confirm_payment, cancel_payment (P1)

### Deferred to Follow-Up Work

- PaymentSession.token dead field removal
- Session TTL/eviction mechanism
- POST form vs GET route mismatch
- RPC error response shape inconsistencies (`"data"` vs `"error"`)
- Response struct Serialize derive cleanup
- Duplicate query struct consolidation
- Formatting changes separated into own commit

---

## Implementation Units

### U1. Fix cancel_payment ownership validation

**Goal:** Add `Extension<GameSession>` extraction and profile_id ownership check to cancel_payment handler, matching confirm_payment's pattern.

**Dependencies:** None

**Files:**
- `src/bin/net/router/social/cancel_payment.rs`

**Approach:** Extract `Extension<GameSession>` in handler signature. Use `get()` first to check `session_data.profile_id == session.profile.id`, then `take()` only if ownership matches. Return error on mismatch.

**Patterns to follow:** `confirm_payment.rs:44-49` ownership check pattern.

**Test scenarios:**
- Cancel by owning profile: session removed, response_code "CANCEL"
- Cancel by non-owning profile: session preserved, response_code "ERROR"
- Cancel nonexistent payment_id: response_code "CANCEL" (no-op)

**Verification:** `cargo test` passes. Manual: two users, one cancels the other's payment → error.

### U2. Add count/price input validation in payment_create

**Goal:** Reject non-positive count and price values before creating payment session.

**Dependencies:** None

**Files:**
- `src/bin/net/router/social/payment_create.rs`

**Approach:** After parsing count and price to i64, add `if count <= 0` and `if price <= 0` guards returning error_response. Optionally cap count to a reasonable maximum.

**Test scenarios:**
- count = "0": returns status -1
- count = "-5": returns status -1
- price = "0": returns status -1
- price = "-100": returns status -1
- count = "1", price = "500": success (existing happy path)

**Verification:** `cargo test` passes.

### U3. Fix confirm_payment validate-before-consume

**Goal:** Validate profile_id ownership before consuming session, so failed validation doesn't lose the session.

**Dependencies:** None

**Files:**
- `src/bin/net/router/social/confirm_payment.rs`

**Approach:** Replace `take()` with `get()` for initial lookup. Validate `profile_id` match. Only call `take()` after validation passes. If `add_pay_item` fails after `take()`, log the error and return the generic message (session is consumed — acceptable trade-off vs re-insert complexity).

**Patterns to follow:** U1's get-then-take pattern.

**Test scenarios:**
- Confirm by owning profile: item added, session removed
- Confirm by non-owning profile: session preserved, response_code "ERROR"
- Confirm nonexistent session: response_code "ERROR"
- Double-confirm: second attempt returns "session not found"

**Verification:** `cargo test` passes.

### U4. Fix payment_html IDOR

**Goal:** Validate session ownership before rendering payment confirmation page.

**Dependencies:** None

**Files:**
- `src/bin/net/router/game.rs`

**Approach:** After retrieving session via `get()`, check `session_data.profile_id == session.profile.id`. Return error HTML on mismatch.

**Test scenarios:**
- Render own payment: shows confirmation page
- Render another user's payment: shows error
- Render nonexistent payment_id: shows "not found"

**Verification:** `cargo test` passes.

### U5. Validate price against codex manifest

**Goal:** Use the codex's canonical price for the SKU instead of trusting client-supplied value.

**Dependencies:** None

**Files:**
- `src/bin/net/router/social/payment_create.rs`

**Approach:** After validating sku_id exists via `codex.find::<ApiMstPayitem>(&sku_id)`, read the canonical price from the manifest entry. Use that price for the PaymentSession instead of the client-supplied value. Retain client-supplied count (validated by U2).

**Patterns to follow:** `emukc_internal::prelude::ApiMstPayitem` — check what price field is available on the manifest entry.

**Test scenarios:**
- Client sends price="1" for 500pt item: stored price is 500 (from manifest)
- Client sends correct price: stored price is 500 (same result)
- sku_id not in manifest: returns error (existing behavior)

**Verification:** `cargo test` passes. Check `ApiMstPayitem` struct for price field name.

### U6. Add handler-level tests

**Goal:** Test the full payment flow at the handler level — create, confirm, cancel, error paths.

**Dependencies:** U1, U2, U3, U4, U5

**Files:**
- `src/bin/net/router/social/payment_create.rs` (test module)
- `src/bin/net/router/social/confirm_payment.rs` (test module)
- `src/bin/net/router/social/cancel_payment.rs` (test module)

**Approach:** Use existing `test_utils::new_test_context()` pattern from `kcsapi/mod.rs`. Call handler functions directly with constructed `State`, `GameSession`, and `RpcParams`/`Query`.

**Test scenarios:**

payment_create:
- Happy path: valid sku_id, returns status 1 with transactionUrl
- Missing params: returns status -1
- Invalid sku_id format: returns status -1
- sku_id not in manifest: returns status -1
- count = 0 or negative: returns status -1
- price from manifest overrides client value

confirm_payment:
- Happy path: item added to DB, session consumed
- Profile mismatch: session preserved, ERROR returned
- Missing session: ERROR returned
- Double-confirm: second returns "not found"

cancel_payment:
- Happy path: session consumed, CANCEL returned
- Profile mismatch: session preserved, ERROR returned
- Missing session: CANCEL returned (no-op)

**Verification:** `cargo test` passes with all new tests green.

---

## Key Technical Decisions

1. **get-then-take pattern for ownership validation:** Use `get()` for read + validate, `take()` only after validation passes. Avoids session loss on validation failure.

2. **Price override from manifest:** Trust codex, not client. The client-supplied price is cosmetic-only in this emulator, but overriding prevents display spoofing.

3. **cancel_payment error on mismatch:** Changed from "always CANCEL" to "ERROR on profile mismatch". This is a behavior change but aligns with confirm_payment's contract.

4. **Test approach:** Direct function calls via `test_utils::new_test_context()` rather than HTTP integration tests. Matches existing codebase pattern and avoids needing embedded assets.
