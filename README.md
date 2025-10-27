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

## What's new in 0.6.6

**🪟 Windows Integration**

- The Windows executable now includes an embedded icon (`res/rTimelogger.ico`), visible in Explorer and taskbar.
- The embedding process is fully automated using the `winres` build dependency — no manual steps required.
- The `.res` file generated during compilation is temporary and not part of the repository.

**⚙️ CLI Consistency Update**

- The top-level subcommand `conf` has been renamed to `config` to align naming conventions across all Rust CLI tools.
    - Subcommands remain unchanged (`--print`, `--edit`, `--editor`).
    - Corresponding handler renamed from `handle_conf` → `handle_config`.
    - ⚠️ **Breaking change:** users must now call:
      ```bash
      rtimelogger config --print
      ```

**🗂️ Resource Organization**

- Added a new `res/` directory for graphical assets (SVG, PNG, ICO).
- The build process automatically compiles and embeds these assets during `cargo build --release`.

**🧹 Backup Improvements**

- When `--compress` is used, the uncompressed backup file (e.g. `my_db.sqlite.bck`) is automatically removed after a
  successful compression.
- A non-fatal warning is displayed if deletion fails.

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
rtimelogger list                # all
rtimelogger list --period 2025  # year
rtimelogger list --period 2025-09  # year-month
rtimelogger list --pos o        # position (case-insensitive)
```

### List raw events

```bash
rtimelogger list --events
rtimelogger list --events --pos r          # filter by position
rtimelogger list --events --pairs 2        # only pair 2 (per date)
rtimelogger list --events --json           # raw JSON with pair & unmatched
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

Examples:

```bash
# Export all events as CSV to an absolute path
rtimelogger export --format csv --file /absolute/path/events.csv --events

# Export sessions as JSON for September 2025
rtimelogger export --format json --file /absolute/path/sessions.json --sessions --range 2025-09

# Export events for a specific day range using brace syntax
rtimelogger export --format csv --file /absolute/path/sep28.csv --events --range 2025-02-{28..28}

# Export sessions as XLSX to an absolute path
rtimelogger export --format xlsx --file /absolute/path/sessions.xlsx --sessions

# Export events as PDF for October 2025
rtimelogger export --format pdf --file /absolute/path/events.pdf --events --range 2025-10
```

Notes:

- `--range` supports: `YYYY` (whole year), `YYYY-MM` (month), and `YYYY-MM-{dd..dd}` (day range inside a month).
- The output `--file` must be an absolute path. If the file exists the CLI will prompt for confirmation unless you
  pass `--force` to overwrite without prompting.
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

## ⚙️ Configuration (duplicate quick ref)

(See above primary configuration section.)

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
