## 1. Investigate API Response

- [x] 1.1 Inspect actual API response from `https://api.anthropic.com/api/oauth/usage` to confirm exact field names for bonus/extra limits, reset dates, and dollar amounts
- [x] 1.2 Document the mapping from API fields to current/weekly/extra categories

## 2. Extend API Types

- [x] 2.1 Add optional fields to `ApiRateLimit` struct for dollar amounts (`bonus_usage`, `bonus_limit`, `bonus_reset_date`, or actual field names discovered in 1.1)
- [x] 2.2 Add a `limit_type` or `category` enum to `RateLimit` (Current, Weekly, Extra, Unknown)
- [x] 2.3 Extend `RateLimit` struct with optional dollar fields and reset date

## 3. Categorization Logic

- [x] 3.1 Implement categorization function that maps `ApiRateLimit` entries to Current/Weekly/Extra based on API response fields
- [x] 3.2 Add fallback: uncategorized limits display with raw `window_label` (preserve current behavior)
- [x] 3.3 Write unit tests for categorization with various API response shapes

## 4. Rendering

- [x] 4.1 Add label alignment constants (fixed-width column for "current", "weekly ", "extra  ")
- [x] 4.2 Implement current/weekly line rendering: `<label> <bar> <pct>% ↻<reset_info>`
- [x] 4.3 Implement extra line rendering: `<label> <bar> $<used>/$<total>`
- [x] 4.4 Implement "resets" line rendering below extra: `resets <date>`
- [x] 4.5 Update `statusline::render()` to use categorized rendering instead of raw loop
- [x] 4.6 Write unit tests for each line format variant

## 5. Integration & Verification

- [x] 5.1 Run full test suite (`cargo test`) and fix any regressions
- [ ] 5.2 Manual test with live API to verify output matches original Claude Code format
- [x] 5.3 Test graceful degradation when API returns unexpected shapes
