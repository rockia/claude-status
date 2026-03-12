## Context

CSL fetches rate limit data from `https://api.anthropic.com/api/oauth/usage` and displays each entry as-is using `window_label`, `usage_percentage`, and `reset_info`. The original Claude Code statusline categorizes these into three distinct rows — **current**, **weekly**, **extra** — each with its own formatting style. CSL needs to match this categorization and formatting.

The API returns a `rateLimits` array where each entry has a `windowLabel` (e.g., model name for current, time period for weekly), `usagePercentage`, `resetInfo`, and potentially additional fields for dollar-based limits (`bonusUsage`, `bonusLimit`, `bonusResetDate` or similar).

## Goals / Non-Goals

**Goals:**
- Categorize API rate limits into current, weekly, and extra display rows
- Match the original Claude Code formatting: percentage bars for current/weekly, dollar amounts for extra
- Show reset info inline for current/weekly, and as a separate "resets" line for extra
- Align labels visually (fixed-width label column)

**Non-Goals:**
- Changing the API endpoint or caching strategy
- Supporting arbitrary new limit categories beyond current/weekly/extra
- Matching the exact progress bar characters (●/○ vs ▰/▱) — keeping existing bar style is fine

## Decisions

### 1. Categorization strategy

**Decision**: Categorize by inspecting the API response structure rather than hardcoding label matching.

The API likely distinguishes limit types through fields beyond just `windowLabel`. We need to inspect the actual API response to determine the correct categorization. The approach:
- First rate limit entry with a percentage → **current** (typically the per-model or hourly limit)
- Entry with a weekly/longer window → **weekly**
- Entry with dollar amounts or bonus fields → **extra**

If the API doesn't provide clear type markers, fall back to `windowLabel` pattern matching (e.g., labels containing "weekly" or time periods > 1 day).

**Alternative considered**: Hardcoding exact `windowLabel` strings — rejected because labels may vary by plan or model.

### 2. Extra line formatting

**Decision**: The extra line shows `$used/$total` instead of a progress bar percentage. The reset date appears on a separate line below as `resets <date>`.

This matches the original exactly. The progress bar for extra still renders but shows 0% when no bonus is used, using dollar amounts as the primary metric.

### 3. Label alignment

**Decision**: Use fixed-width label formatting. Labels are left-padded to align: `current`, `weekly `, `extra  ` (7 chars). The reset info uses `↻` (U+21BB) prefix for current/weekly inline resets.

### 4. API response parsing

**Decision**: Extend `ApiRateLimit` and `RateLimit` structs to capture additional optional fields that the API may return (dollar amounts, reset dates, limit type/category). Parse these as `Option` fields so the code degrades gracefully if they're absent.

## Risks / Trade-offs

- **[API response shape unknown]** → We haven't confirmed the exact fields the API returns for bonus/extra limits. Mitigation: Make all new fields optional; log/inspect actual API response during development to confirm shape.
- **[Categorization heuristic may be fragile]** → If the API changes labels or adds new limit types, categorization could break. Mitigation: Fall back to displaying raw entries (current behavior) if categorization fails.
- **[Dollar formatting locale]** → Using `$` prefix assumes USD. Mitigation: The original also uses `$`, so this matches expected behavior.
