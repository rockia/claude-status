## ADDED Requirements

### Requirement: Rate limits are categorized into current, weekly, and extra
The system SHALL categorize API rate limit entries into three display categories: **current** (short-term usage), **weekly** (weekly usage), and **extra** (bonus/dollar-based allowance). Categorization SHALL be based on API response fields indicating the limit type or window duration.

#### Scenario: API returns current and weekly percentage limits
- **WHEN** the API response contains rate limits with short-term and weekly windows
- **THEN** the system displays them as "current" and "weekly" rows respectively, each with a progress bar and percentage

#### Scenario: API returns an extra/bonus limit
- **WHEN** the API response contains a rate limit with dollar-based bonus fields
- **THEN** the system displays it as an "extra" row with dollar formatting instead of percentage

#### Scenario: API returns unrecognized limit types
- **WHEN** the API response contains rate limits that cannot be categorized
- **THEN** the system falls back to displaying them with their raw `window_label` (current behavior)

### Requirement: Current line displays percentage with inline reset time
The system SHALL render the current limit line as: `current <progress_bar> <percentage>% ↻<reset_time>` where reset_time is a short time format (e.g., "6:00pm").

#### Scenario: Current limit at 23% resetting at 6pm
- **WHEN** the current limit has usage_percentage=23 and reset_info indicates 6:00pm
- **THEN** the output line SHALL be `current ▰▰▱▱▱▱▱▱▱▱ 23% ↻6:00pm` (with appropriate coloring)

#### Scenario: Current limit at 0%
- **WHEN** the current limit has usage_percentage=0
- **THEN** the progress bar shows all empty segments with 0%

### Requirement: Weekly line displays percentage with inline reset date
The system SHALL render the weekly limit line as: `weekly  <progress_bar> <percentage>% ↻<reset_date>` where reset_date includes a date and time (e.g., "mar 19, 8:00am").

#### Scenario: Weekly limit at 12% resetting on mar 19
- **WHEN** the weekly limit has usage_percentage=12 and reset_info indicates mar 19, 8:00am
- **THEN** the output line SHALL be `weekly  ▰▱▱▱▱▱▱▱▱▱ 12% ↻mar 19, 8:00am` (with appropriate coloring)

### Requirement: Extra line displays dollar amounts
The system SHALL render the extra limit line as: `extra   <progress_bar> $<used>/$<total>` showing dollar amounts instead of a percentage.

#### Scenario: Extra limit with no usage
- **WHEN** the extra limit has $0.00 used of $0.00 total
- **THEN** the output line SHALL be `extra   ▱▱▱▱▱▱▱▱▱▱ $0.00/$0.00`

#### Scenario: Extra limit with partial usage
- **WHEN** the extra limit has $5.00 used of $20.00 total
- **THEN** the output line SHALL show `extra   ▰▰▰▱▱▱▱▱▱▱ $5.00/$20.00` with appropriate bar fill

### Requirement: Extra reset date displayed on separate line
The system SHALL display a separate line `resets <date>` below the extra line showing when the extra allowance resets.

#### Scenario: Extra resets on apr 1
- **WHEN** the extra limit has a reset date of April 1
- **THEN** a line `resets apr 1` SHALL appear after the extra line

#### Scenario: No extra limit or no reset date
- **WHEN** no extra limit exists or no reset date is available
- **THEN** the "resets" line SHALL be omitted

### Requirement: Labels are visually aligned
The system SHALL align the category labels (current, weekly, extra) so that progress bars start at the same column. Labels SHALL be right-padded to a consistent width.

#### Scenario: All three limit types displayed
- **WHEN** current, weekly, and extra limits are all present
- **THEN** the progress bars on all three lines start at the same horizontal position
