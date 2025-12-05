<h1 style="text-align: left;">
  <img src="res/rtimelogger.svg" width="90" style="vertical-align: middle; margin-right: 8px;" alt="rTimelogger Logo"/>
  rTimelogger
</h1>

[![Build Status](https://github.com/umpire274/rTimelogger/actions/workflows/ci.yml/badge.svg)](https://github.com/umpire274/rTimelogger/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/v/release/umpire274/rTimelogger)](https://github.com/umpire274/rTimelogger/releases)
[![codecov](https://codecov.io/gh/umpire274/rTimelogger/graph/badge.svg?token=41167c42-54af-4d6a-a9ba-8c2dbef4107d)](https://codecov.io/gh/umpire274/rTimelogger)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

`rTimelogger` is a simple, cross-platform **command-line tool** written in Rust to track daily working sessions,
including working position, start and end times, and lunch breaks.  
The tool calculates the expected exit time and the surplus of worked minutes.

---

## ✨ New in v0.8.0-beta1

### 🚀 Major changes (command `list`)

- The `list` command has been fully rewritten to use the new **timeline-based model**.
- Rendering no longer depends on the legacy `work_sessions` table; it now uses  
  `events → timeline → pairs`.
- Improved readability with a **clean, aligned layout**, color-coded values, and consistent terminology.
- Added **automatic month separators** when `--period` spans multiple months.
- Reworked behavior of `--now`, `--details`, and `--events` for consistency.

### 🕒 Computation improvements

- **Expected Exit** is now correctly computed as:  
  `expected = start_time + work_duration + lunch_minutes`.
- **Daily surplus** is now strictly:  
  `surplus = end_time – expected`,  
  avoiding double-counting issues from previous versions.
- Enhanced `mins2readable()`:
    - short format: `+02:25`
    - long format: `+02h 25m`
    - supports optional ANSI coloring via helper functions.

### 🎨 Output & formatting enhancements

- Intelligent ANSI coloring:
    - **green** → positive surplus
    - **red** → negative surplus
    - **gray** → missing or undefined fields (`--:--`, `-`, `- min`)
- Consistent column alignment for a professional, stable layout.
- Centralized color helpers (`gray()`, `green()`, `red()`, `reset()`).

### 🔧 Internal refactoring

- Complete rewrite of `print_daily_summary_row()` with timeline logic.
- Unified color utilities in `formatting.rs`.
- Improved handling of partial and incomplete event pairs.
- Cleanup of legacy paths and duplicated code.

### 🧹 Fixes

- Fixed Expected Exit not correctly reflecting lunch duration.
- Fixed lunch display being shown as `--:--` even when present.
- Fixed formatting bugs in total surplus output.
- Fixed inconsistencies between `list` and `list --events`.

---

### ✨ New in 0.8.0-alpha2

**✔ Fully redesigned `add` command**

The `add` command now uses a clean and explicit syntax based entirely on flags.
The only positional argument is now the **date**.

Old syntax (deprecated):

```bash
rtimelogger add 2025-09-13 O 09:00 60 17:30
```

New syntax (stable):

```bash
rtimelogger add 2025-09-13 --pos O --in 09:00 --lunch 60 --out 17:30
```

This redesign removes ambiguity, improves clarity, and aligns the command with
the rest of the CLI.

Key improvements:

- No more positional parameters for position, IN, OUT, lunch
- All parameters must now be passed via flags (`--pos`, `--in`, `--out`, `--lunch`)
- Editing is cleaner and explicit via `--edit --pair N`
- Future-proof: easy to extend without breaking changes

---

## 🚀 What's New in **v0.8.0-alpha1**

This is the largest rewrite of **rtimelogger** to date.  
The internal event model, database logic, and CLI behavior have all been redesigned for correctness, consistency, and
long-term maintainability.

### ✅ Major Changes

- **New event model**
    - `timestamp` removed → now separated into `date` + `time`.
    - New fields: `pair`, `location`, `lunch_break`, `meta`, `source`, `created_at`.
    - All parsing/formatting logic updated across the CLI.

- **Database redesign**
    - Fully rewritten queries using the new schema.
    - Automatic **pair recalculation** after each insert/edit/delete.
    - Guaranteed chronological ordering and consistent pairing logic.

- **Improved `add` command**
    - All parameters are now **fully optional** except the date.
    - Smart defaults:
        - Position defaults to previous event’s position.
        - Lunch and end time can be added independently.
    - Full **edit mode** with `--edit --pair N`.

- **Improved `del` command**
    - Safe deletion with interactive **confirmation prompts**.
    - Can remove either a full day or a specific pair.
    - Automatic pair number recalculation after deletion.

### 📄 New & Improved Commands

- **`log`**
    - ANSI-colored output.
    - Dynamic column width.
    - Automatic timestamp normalization (`%FT%T`).
    - Combined `operation + target` column with intelligent alignment.

- **`backup`**
    - Full rewrite.
    - Supports ZIP compression.
    - Logs backup operations internally.

- **`export`**
    - Complete refactor: CSV, JSON, XLSX, PDF.
    - XLSX export rewritten with styling, date/time detection, column autosizing.
    - PDF export stabilized.

### 🛠 Internal Improvements

- New helpers for date/time parsing and validation.
- Consistent error model across modules.
- Better `DbPool` usage and cleanup.
- Large reduction in duplicated logic.
- Much cleaner separation between CLI layer, core logic, and DB layer.

---

## ✨ Features

- Add, update, delete and list work sessions.
- Track **start time**, **lunch duration**, and **end time**.
- Calculate **expected exit time** and **surplus** automatically.
- Manage multiple **working positions**:
    - `O` = **Office**
    - `R` = **Remote**
    - `C` = **On-Site (Client)**
    - `H` = **Holiday**
    - `M` = **Mixed** (multiple working positions on the same day)
- Colorized output for better readability:
    - **Blue** = Office
    - **Cyan** = Remote
    - **Yellow** = On-Site (Client)
    - **Purple background + white bold** = Holiday
- Configurable default DB path via configuration file or `--db` parameter.
- Automatic DB migrations with version tracking (`schema_migrations` table).
- Configurable daily working time (`min_work_duration`, default `8h`).
- Automatic expected exit calculation based on:
    - Start time
    - Lunch break duration
    - Configured working time
- Automatic handling of lunch break rules:
    - Minimum 30 minutes
    - Maximum 1h 30m
    - Required only for `Office` position (`O`)
- View surplus/deficit of worked time compared to expected
- Display of the **total surplus** at the bottom of `list` output.
- **Event mode** with: Pair grouping, per-pair summary, JSON enrichment, unmatched detection, filtering by position &
  pair id.
- Automatic database migration for schema changes
- Cross-platform configuration file management:
    - Linux/macOS: `$HOME/.rtimelogger/rtimelogger.conf`
    - Windows: `%APPDATA%\rtimelogger\rtimelogger.conf`

---

## 📦 Installation

### 🐧 AUR (Arch Linux)

[![AUR](https://img.shields.io/aur/version/rtimelogger)](https://aur.archlinux.org/packages/rtimelogger)

```bash
yay -S rtimelogger
# or
paru -S rtimelogger
```

### 🍺 Homebrew (macOS/Linux)

[![Homebrew](https://img.shields.io/badge/Homebrew-rTimelogger-orange.svg?logo=homebrew)](https://github.com/umpire274/homebrew-tap)

```bash
brew tap umpire274/tap
brew install rtimelogger
```

### 🦀 Crates.io (Rust)

[![Crates.io](https://img.shields.io/crates/v/rtimelogger)](https://crates.io/crates/rtimelogger)

```bash
cargo install rtimelogger
```

---

## ⚙️ Configuration

When you run:

```bash
rtimelogger init
```

a configuration file is created in the user’s config directory (`rtimelogger.conf`).  
It includes for current releases (≥ 0.4.0):

```yaml
database: /home/user/.rtimelogger/rtimelogger.sqlite
default_position: O
min_work_duration: 8h
min_duration_lunch_break: 30
max_duration_lunch_break: 90
separator_char: "-"
show_weekday: None   # Options: None | Short | Medium | Long
```

Key fields:

- **database** → path to the SQLite DB file
- **default_position** → default working position (`O`, `R`, `C`, `H`, `M`)
- **min_work_duration** → daily expected working time (e.g. `7h 36m`, `8h`)
- **min_duration_lunch_break** / **max_duration_lunch_break** → lunch constraints (minutes)
- **separator_char** → character used for month-end separator lines
- **show_weekday** → controls weekday format in list output (`None`, `Short`, `Medium`, `Long`)

> NOTE: Older docs referenced `working_time`; it has been unified as `min_work_duration`.

Override DB path at runtime:

```bash
rtimelogger --db /custom/path/mydb.sqlite <command>
```

---

## ➕ Add a Work Session — `rtimelogger add`

Starting from version **0.8.0-alpha2**, the `add` command has been fully
redesigned to use a clean, explicit, and modern flag-based syntax.

The only positional argument is now the **date**.  
All other values (position, IN time, OUT time, lunch break, edits) must
be specified using flags.

This provides clearer semantics, eliminates ambiguity, and enables future
extensions without breaking the CLI.

---

### 🔧 Usage

```bash
rtimelogger add <DATE> [OPTIONS]
```

Where:

- `<DATE>` is mandatory
- all other parameters are optional flags

### 🏷 Supported Options

| Flag                | Description                                    | Example     |
|---------------------|------------------------------------------------|-------------|
| `--pos <POSITION>`  | Work position (O, R, H, C, M)                  | --pos O     |
| `--in <HH:MM>`      | Clock-in time (HH:MM                           | --in 08:50  |
| `--out <HH:MM>`     | Clock-out time (HH:MM)                         | --out 17:30 |
| `--lunch <MINUTES>` | Lunch break in minutes                         | --lunch 45  |
| `--edit`            | Edit existing pair (requires `--pair`)         | --edit      |
| `--pair <N>`        | Select the N-th IN/OUT pair of the day to edit | --pair 1    |

### 🧠 Behavioral Rules

#### ▶ IN only

Adds a new open work session:

```bash
rtimelogger add 2025-09-15 --in 09:00
```

#### ⏸ OUT only

Closes the last open work session:

```bash
rtimelogger add 2025-09-15 --out 17:30
```

#### ▶ IN + OUT

Creates a full IN/OUT pair:

```bash
rtimelogger add 2025-09-15 --in 09:00 --out 17:30
```

#### 🏷️ Position only

Sets or updates working position for the last pair:

```bash
rtimelogger add 2025-09-15 --pos R
```

#### 🍽️ Lunch only

Sets or updates lunch break for the last pair:

```bash
rtimelogger add 2025-09-15 --lunch 60
```

#### ✏️ Edit existing pair

Modify an existing pair on the selected date (requires `--pair N`):

```bash
rtimelogger add 2025-12-03 --edit --pair 1 --out 15:30
```

### 📝 Examples

#### Add a regular IN

```bash
rtimelogger add 2025-09-15 --in 09:00
```

#### Add a regular OUT

```bash
rtimelogger add 2025-09-15 --out 17:30
```

#### Add a full IN/OUT pair with lunch

```bash
rtimelogger add 2025-09-15 --in 09:00 --lunch 60 --out 17:30
```

#### Set position to Remote

```bash
rtimelogger add 2025-09-15 --pos R
```

#### Edit pair 1 to change OUT time

```bash
rtimelogger add 2025-12-03 --edit --pair 1 --out 15:30
```

### 🚀 Why this change?

The previous syntax mixed positional arguments and flags:

```bash
rtimelogger add 2025-09-13 O 09:00 60 17:30
```

This created ambiguity and made the command hard to extend.

The new syntax:

- is cleaner
- is explicit
- avoids misinterpretation
- supports new features without breaking compatibility
- follows best practices of modern CLI tools

---

## 🖥️ Usage

### Initialize DB and config

```bash
rtimelogger init
```

Custom DB file relative to config dir:

```bash
rtimelogger --db mydb.sqlite init
```

Absolute path:

```bash
rtimelogger --db "G:/My Drive/Work/Timelog/rtimelogger.sqlite" init
```

### Add a full work session

```bash
rtimelogger add 2025-09-13 O 09:00 60 17:30
```

Creates or updates the legacy session AND adds two events (in/out) for reporting.

### Partial updates (each creates/updates events when relevant)

```bash
rtimelogger add 2025-09-13 --pos R
rtimelogger add 2025-09-13 --in 09:00
rtimelogger add 2025-09-13 --lunch 45
rtimelogger add 2025-09-13 --out 17:30
```

### Add holiday

```bash
rtimelogger add 2025-09-14 --pos H
```

### List sessions (legacy view)

```bash
rtimelogger list                   # shows *current month* by default
rtimelogger list --today              # shows only today
rtimelogger list --today --details    # shows today with event details
rtimelogger list --period 2025     # year
rtimelogger list --period 2025-09  # month
rtimelogger list --period 2025-09-15  # specific day
rtimelogger list --period 2025-06-01:2025-06-10  # day range
rtimelogger list --period 2025-06:2025-08        # month range
rtimelogger list --pos o           # filter by position
rtimelogger list --period all         # entire history
```

### List raw events

```bash
rtimelogger list --events
rtimelogger list --events --pos r          # filter by position
rtimelogger list --events --pairs 2        # only pair 2 (per date)
rtimelogger list --events --json           # raw JSON with pair & unmatched
rtimelogger list --events --period 2025-06
rtimelogger list --events --period 2025-06-01:2025-06-30
rtimelogger list --events --period 2024:2025
```

### Summarize events per pair

```bash
rtimelogger list --events --summary
rtimelogger list --events --summary --pairs 1
rtimelogger list --events --summary --json
```

### Sample output of summary mode

```text
📊 Event pairs summary:
Date        Pair  Pos  Start  End    Lunch  Dur
----------  ----  ---  -----  -----  -----  --------
2025-12-01  1     O    09:00  12:00     30  2H 30M
2025-12-01  2     O    13:00  17:00      0  4H 00M
```

*Note: JSON output still contains `duration_minutes` expressed as integer minutes.*

### Delete a session by date

```bash
# Delete all records for a date (confirmation required)
rtimelogger del 2025-10-02
```

Example (interactive):

```bash
$ rtimelogger del 2025-10-02
Are you sure to delete the records of the date 2025-10-02 (N/y) ? y
🗑️  Deleted 2 event(s) and 1 work_session(s) for date 2025-10-02
```

### Delete a specific pair for a specific date

```bash
# Delete only pair 1 for a specific date (confirmation required)
rtimelogger del --pair 1 2025-10-02
```

Example (interactive):

```bash
$ rtimelogger del --pair 1 2025-10-02
Are you sure to delete the pair 1 of the date 2025-10-02 (N/y) ? y
🗑️  Deleted 1 event(s) for pair 1 on 2025-10-02
```

### Internal log

```bash
rtimelogger log --print
```

Example output of `rtimelogger log --print`:

```bash
📜 Internal log:
  1: 2025-10-03T12:00:00Z | init       | Database initialized at C:\Users\you\AppData\Roaming\rtimelogger\rtimelogger.sqlite
  2: 2025-10-03T12:05:00Z | del        | Deleted date=2025-10-02 events=2 work_sessions=1
  3: 2025-10-03T12:06:00Z | auto_lunch | auto_lunch 30 min for out_event 12 (date=2025-10-02)
```

### Backup database

```bash
# Simple copy
rtimelogger backup --file "/path/to/backup.sqlite"

# With compression
rtimelogger backup --file "/path/to/backup.sqlite" --compress
```

- On Windows creates /path/to/backup.zip
- On Linux/macOS creates /path/to/backup.tar.gz

Notes:

- When `--compress` is provided, the CLI now removes the original uncompressed backup file after successful
  compression (e.g. `my_db.sqlite.bck` -> `my_db.sqlite.zip`); a non-fatal warning is printed if the removal fails. This
  avoids leaving redundant files in the backup directory.

---

### Export data

You can export recorded events or aggregated work sessions to **CSV**, **JSON**, **XLSX**, or **PDF**.  
The `export` subcommand supports date-range filtering with multiple formats and writes to an absolute output path.

### 📅 Supported date formats for `--range` and `--period`

| Input format            | Meaning      | Expanded range (start → end)           |
|-------------------------|--------------|----------------------------------------|
| `YYYY`                  | Full year    | `YYYY-01-01` → `YYYY-12-31`            |
| `YYYY-MM`               | Full month   | `YYYY-MM-01` → `YYYY-MM-lastday`       |
| `YYYY-MM-DD`            | Specific day | `YYYY-MM-DD` → `YYYY-MM-DD`            |
| `YYYY:YYYY`             | Year range   | `YYYY-01-01` → `YYYY-12-31`            |
| `YYYY-MM:YYYY-MM`       | Month range  | `start-month-01` → `end-month-lastday` |
| `YYYY-MM-DD:YYYY-MM-DD` | Day range    | `start-day` → `end-day`                |

> Note: All formats must use the **same pattern** on both sides of a range (e.g., `2025-06:2025-07`, not
`2025-06:2025-07-10`).

Examples:

```bash
# Export all events as CSV
rtimelogger export --format csv --file /abs/path/events.csv --events --range all

# Export all sessions as JSON
rtimelogger export --format json --file /abs/path/sessions.json --sessions --range all


# --- Single period formats ----------------------------------------------

# Export full year 2025 (events)
rtimelogger export --format csv --file /abs/path/2025-events.csv --events --range 2025

# Export September 2025 sessions
rtimelogger export --format json --file /abs/path/sep-2025-sessions.json --sessions --range 2025-09

# Export a single specific day
rtimelogger export --format csv --file /abs/path/day.csv --events --range 2025-06-01


# --- Full range formats --------------------------------------------------

# Export a year range: 2024–2025
rtimelogger export --format json --file /abs/path/years.json --events --range 2024:2025

# Export a month range: June to August 2025
rtimelogger export --format csv --file /abs/path/summer.csv --sessions --range 2025-06:2025-08

# Export a day range: 1–10 June 2025
rtimelogger export --format csv --file /abs/path/days.csv --events --range 2025-06-01:2025-06-10


# --- XLSX and PDF examples ----------------------------------------------

# Export sessions to XLSX for March 2025
rtimelogger export --format xlsx --file /abs/path/march.xlsx --sessions --range 2025-03

# Export events to PDF for a whole year
rtimelogger export --format pdf --file /abs/path/events-2024.pdf --events --range 2024


# --- Mixed usage ---------------------------------------------------------

# Export only events for a month range and overwrite file if needed
rtimelogger export --format csv --file /abs/path/jun-jul.csv --events \
    --range 2025-06:2025-07 --force

# Export all sessions to XLSX for a specific day range
rtimelogger export --format xlsx --file /abs/path/last-week.xlsx --sessions \
    --range 2025-10-20:2025-10-24
```

Notes:

- `--range` supports:
    - `YYYY` (full year)
    - `YYYY-MM` (full month)
    - `YYYY-MM-DD` (specific day)
    - `YYYY:YYYY` (year range)
    - `YYYY-MM:YYYY-MM` (month range)
    - `YYYY-MM-DD:YYYY-MM-DD` (day range)
    - `all` (entire dataset)
- The output `--file` must be an absolute path.
- If the output file already exists, the CLI prompts for confirmation unless `--force` is used.
- Supported formats: `csv`, `json`, `xlsx`, `pdf`

---

### Event mode – behavior details

- **Pair numbering** restarts each date.
- **Unmatched** rows (only `in` or only `out`) show `*` and `duration_minutes = 0` in summary.
- **Lunch minutes** shown on the `out` event (and propagated to summary) if provided or auto-deduced.
- **Filtering precedence**: `--pairs` applies *after* computing pairs; combining with `--summary` reduces summary rows.
- **JSON schemas**:
    - Raw events: fields from DB + `pair`, `unmatched`.
    - Summary: `date, pair, position, start, end, lunch_minutes, duration_minutes, unmatched`.

---

## 🗄️ Database migrations

*(unchanged – see CHANGELOG for past versions)*

---

## ⚠️ Notes

- Lunch validation: min 30, max 90 (Office only mandatory). Remote can specify 0.
- Holidays ignore start/end/lunch; still appear in sessions listing.
- `--db` allows isolated datasets (useful for testing).

---

## 📊 Legacy session output example

```text
📅 Saved sessions for September 2025:
  1: 2025-09-01 | Remote           | Start 09:08 | Lunch 00:30 | End 17:30 | Expected 17:14 | Surplus  +16 min
  2: 2025-09-04 | Office           | Start 09:35 | Lunch 00:30 | End 17:44 | Expected 17:41 | Surplus   +3 min
  3: 2025-09-05 | Remote           | Start 09:11 | Lunch 00:30 | End 17:01 | Expected 17:17 | Surplus  -16 min
  4: 2025-09-11 | Remote           | Start 08:08 | Lunch   -   | End 12:16 | Worked  4 h 08 min
  5: 2025-09-17 | Office           | Start 09:42 | Lunch 00:30 | End 17:28 | Expected 17:48 | Surplus  -20 min
  6: 2025-09-18 | Remote           | Start 10:50 | Lunch   -   | End   -   | Expected 18:56 | Surplus    -
  7: 2025-09-19 | Holiday          | Start   -   | Lunch   -   | End   -   | Expected   -   | Surplus    - min
  8: 2025-09-22 | Holiday          | Start   -   | Lunch   -   | End   -   | Expected   -   | Surplus    - min
```

---

## Output formatting: month-end separator

(See `separator_char` in configuration.)

---

## 🧪 Tests

Run all tests:

```bash
cargo test --all
```

Include coverage for: sessions CRUD, events pairing, summary, JSON, holidays, migrations.

---

## 📦 Installation

```bash
git clone https://github.com/umpire274/rTimelogger.git
cd rtimelogger
cargo build --release
```

Binaries in `target/release/` or use releases page.

---

## 📜 License

MIT License – see [LICENSE](LICENSE).

---

### Internal Log Recap

```bash
rtimelogger log --print
```

Records concise audit lines for `init`, `add`, `del` and auto-lunch adjustments.
