## Why

CSL's rate limit display currently shows raw API `window_label` values with uniform percentage bars, making it unclear which limit is which. The original Claude Code statusline categorizes limits into **current**, **weekly**, and **extra** with distinct formatting for each (percentages for current/weekly, dollar amounts for extra, plus a separate reset line). Users expect parity with the original.

## What Changes

- Categorize API rate limit entries into **current**, **weekly**, and **extra** based on `window_label` semantics
- Display **current** line: `current ●●○○○○○○○○ 23% ↻6:00pm` — percentage + reset time
- Display **weekly** line: `weekly  ●○○○○○○○○○ 12% ↻mar 19, 8:00am` — percentage + reset date
- Display **extra** line: `extra   ○○○○○○○○○○ $0.00/$0.00` — dollar amounts (used/total) instead of percentage bar
- Display **resets** line below extra: `resets apr 1` — when extra allowance resets
- Align label widths so all three categories line up visually
- Parse additional API response fields needed for dollar amounts and reset dates

## Capabilities

### New Capabilities
- `categorized-limit-display`: Categorize raw API rate limits into current/weekly/extra with type-specific formatting (percentage vs dollar), aligned labels, and reset info

### Modified Capabilities

## Impact

- `src/usage.rs`: Parse additional API response fields (dollar amounts, reset timestamps) and categorize rate limits
- `src/statusline.rs`: Render categorized limit lines with type-specific formatting
- `src/format.rs`: Possibly add dollar-amount formatting and date formatting helpers
- No dependency changes expected — date formatting can use standard library
