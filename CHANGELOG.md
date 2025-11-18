# Changelog

## [0.7.0] - 2025-11-18

### Added

- **Extended period filtering for `list`**  
  The `--period` option now supports advanced date formats and custom ranges:
    - Single values:
        - `YYYY` → full year
        - `YYYY-MM` → full month
        - `YYYY-MM-DD` → specific day
    - Ranges (`start:end`) in matching formats:
        - `YYYY:YYYY` → year range
        - `YYYY-MM:YYYY-MM` → month range
        - `YYYY-MM-DD:YYYY-MM-DD` → day range
    - Example usage:
        - `rtimelogger list --period 2025-06`
        - `rtimelogger list --period 2025-06-01:2025-06-10`
        - `rtimelogger list --period 2024:2025`

- **Default period for `list`**  
  Running `rtimelogger list` without any period or event filters now automatically shows the **current month**,
  improving usability for daily workflows.

- **Updated `export --range` option**  
  `--range` now supports the exact same date formats and ranges as `--period`, ensuring consistency between listing and
  exporting data.

### Changed

- Improved the help messages of `--period` and `export --range` with explicit descriptions and examples.
- Refactored and extended the internal date-range parsing logic (`build_filtered_query`) to support year, month, day,
  and custom ranges seamlessly for both sessions and events.

### Fixed

- Improved error handling for malformed `add` command input (Issue #22). When the user provides an invalid date or
  incorrect positional argument order, the CLI now displays a concise usage guide and examples directly in the error
  output, instead of only showing the validation error. This makes the `add` command more self-explanatory and avoids
  the need to run `rtimelogger add --help` after every mistake.

### Notes

`v0.7.0` introduces a new unified and powerful time-range filtering system, greatly improving the user experience for
querying and exporting historical data while keeping all previous usages backwards-compatible.

---

## [0.6.6] - 2025-10-13

### Added

- Embedded Windows application icon (`res/rTimelogger.ico`) directly into the executable.
    - Implemented using the `winres` build dependency.
    - The icon is now visible in Windows Explorer and taskbar.
- New project resource directory `res/` for graphical assets (SVG, PNG, ICO).

### Changed

- cli: rename top-level subcommand `conf` → `config` (and variant `Commands::Conf` → `Commands::Config`) to harmonize
  CLI naming across projects.
    - Subcommands remain the same: `--print`, `--edit`, `--editor`.
    - Handler function renamed from `handle_conf` to `handle_config` and corresponding call sites updated.
    - Note: this is a breaking CLI name change (users must call `rtimelogger config ...`). Consider adding a
      backwards-compat shim in a follow-up if desired.

### Changed

- Build process updated to automatically compile and embed Windows resources during `cargo build --release`.
- Improved project organization by moving graphical resources under `res/`.

### Fixed / Misc

- backup: when `--compress` is provided, the original uncompressed backup file (e.g. `my_db.sqlite.bck`) is now removed
  after successful compression (e.g. `my_db.sqlite.zip` or `my_db.sqlite.tar.gz`) to avoid leaving redundant files on
  disk; a
  non-fatal warning is emitted if removal fails.

### Notes

- The `.res` file generated during build is temporary and no longer stored in the repository.

---

## [0.6.5] - 2025-10-10

### Added

- tests: `tests/position_recompute_tests.rs` with:
    - functional coverage for single-position recompute (ensures `position` and `end_time` are correct after deleting
      pairs),
    - robustness loop (multiple independent runs) to catch potential flakiness related to test harness/FS state.

### Changed

- perf(db): recompute `work_sessions.position` using a single SQLite query (COUNT(DISTINCT position), MIN(position)) in
  `delete_events_by_ids_and_recompute_sessions` instead of materializing positions in Rust.
    - Moves distinct/count work to SQLite, avoids extra allocations and sorting, and provides an early-exit cheap path
      for mixed positions.
    - Preserves existing semantics: update `position` only when exactly one distinct position remains; otherwise leave
      unchanged.

### Fixed / Misc

- chore(main): removed redundant closure in `src/main.rs` (`unwrap_or_else(Config::load)` ->
  `unwrap_or_else(Config::load)`).
- All tests pass locally (`cargo test` ran successfully after changes).

---

## [0.6.0] - 2025-10-09

Release highlights: brand-new PDF export support and a reworked XLSX export with improved readability.

### Added

- New PDF export feature:
    - Introduced `PdfManager` module using `pdf-writer` for report generation.
    - Supports tabular layout with bold headers and zebra stripes for readability.
    - Integrated in `export.rs` as an alternative export format alongside CSV and JSON.
- New XLSX export feature:
    - Added `XlsxManager` module using `rust_xlsxwriter` for Excel-compatible exports.
    - Introduced tabular/graphical layout for better readability.
    - First row is now frozen as header.

### Changed

- Refactored time formatting utilities:
    - Updated `mins2hhmm()` to support both combined ("HH:MM") and split ("HH","MM") output via an optional parameter.
    - Added new helper functions `mins2readable()` for consistent human-readable duration formatting (e.g., `07h 45m`).
    - Replaced duplicated inline conversion logic with unified helper calls.
- Updated logic and unit tests to align with the new time formatting behavior.

### Fixed

- Corrected formatting of negative durations to display a single minus sign (e.g., `-01h 25m` instead of `-00h -25m`).
- Resolved minor inconsistencies in duration calculation and string conversion.

---

## [0.5.1] - 2025-10-09

### Added

- Comprehensive export test suite covering CSV/JSON outputs for `--events` and `--sessions`.
  Tests include range filtering (including brace day-range syntax), empty dataset behavior, overwrite/cancel flows,
  CSV structure checks and a performance smoke test.
- Shared test helpers in `tests/common.rs` (`setup_test_db`, `temp_out`, `init_db_with_data`, `populate_many_sessions`)
  to
  reduce duplication across tests and simplify integration test setup.

### Changed

- Implemented `--range` handling for `export` (supports `YYYY`, `YYYY-MM`, and `YYYY-MM-{dd..dd}` brace syntax) and
  applied it to both `events` and `work_sessions` exports.
- Refactored `src/export.rs`:
    - Extracted helper `build_query_with_range` to build SQL + owned parameters.
    - Pass owned date parameters to `stmt.query_map(...)` to ensure correct binding and lifetimes.
- Moved test helpers out of the library (`src/test_common.rs`) into `tests/common.rs` and updated tests to use the
  shared helpers.

### Fixed

- Fixed an export bug where SQL range parameters were not passed to `query_map` (caused incorrect/ignored ranges and
  lifetime errors E0597). Exports now correctly filter by date when `--range` is provided.
- Resolved Clippy warnings across the codebase; added a small allow for `dead_code` in test helpers where appropriate.

### Notes

- The `--range` option is now implemented and exercised by tests; removed the previous note stating it was a stub.
- Consider adding CSV headers in a follow-up if explicit headings are desired for exported CSV files.

---

## [0.5.0] - 2025-10-08

### Added

- New configuration option `show_weekday` to control weekday display in `list` command (`None`, `Short`, `Medium`,
  `Long`).
- Display weekday next to date in `list` command output.
- New `backup` command with `--file` option to create a copy of the database.
- Added `--compress` flag for backup:
    - On Windows creates a `.zip` archive.
    - On Linux/macOS creates a `.tar.gz` archive.
- Backup operations are logged in the database via `ttlog`.

### Changed

- Moved CLI definition (`Cli` and `Commands`) from `main.rs` into a dedicated `cli.rs` module.
  This improves project structure by keeping `main.rs` focused on the entrypoint logic.
- Improved `del` command output: now shows a clear warning when no events or sessions are found for a date.
- Updated tests to reflect new `del` command behavior.

### Fixed

- Corrected calculation of expected end time considering `min_duration_lunch_break` and `max_duration_lunch_break`.
- Fixed insertion of lunch break duration in `events` table.
- Fixed test alignment for delete of non-existent sessions.
- Deduplicated migration code (removed repeated SQL blocks).

---

## [0.4.5] - 2025-10-06

### Changed

- Project renamed from **`rtimelog`** to **`rtimelogger`**.
- No functional changes: this release only updates the crate name, repository links, badges, and documentation
  references.

---

# [0.4.2] - 2025-10-03

### Added

- `del` enhancements:
    - `del <date>` removes all events and legacy work_session rows for the given date (with interactive confirmation).
    - `del --pair <pair> <date>` removes only the events belonging to the specified pair for that date (with interactive
      confirmation); if no events remain for the date the legacy work_sessions row(s) are removed as well.
- Database helper functions to delete events by date/ids and sessions by date.
- `del` now records concise audit entries into the internal `log` table (visible with `rtimelog log --print`).

### Changed

- Introduced position value `M` (Mixed) to indicate days with multiple working positions; updated `describe_position` to
  display a friendly label for `M`.
- Extracted `create_missing_event` into `src/events.rs` and added a unit test to improve testability and reduce
  duplicate code in `commands.rs`.
- `list --events --summary` now displays Dur in a human-friendly "XH YYM" format; JSON output remains in minutes (
  `duration_minutes`).
- Updated README to document the above behavior and examples.

### Fixed

- Added a migration to extend position CHECKs to include `'M'` and to update existing tables where necessary.
- Fixed a Clippy warning in `db::delete_events_by_ids` (removed an unnecessary map_identity) so
  `cargo clippy -D warnings` passes.
- Updated integration tests to verify deletion-by-date and deletion-by-pair behavior.

---

## [0.4.1] - 2025-10-03

### Added

- Unit test(s) targeting the creation of missing events (exposed for unit testing).

### Changed

- Refactored event helper logic:
    - Extracted `create_missing_event` helper into a reusable function and moved it to `src/events.rs` for better
      testability and separation of concerns.
    - Removed duplicated code in `commands.rs` and consolidated the helper usage.
- Presentation improvement: in `list --events --summary` the Duration field is now displayed in a human-friendly "Xh Ym"
  format instead of raw minutes (display-only change).

### Fixed

- Resolved compilation issues caused by duplicate definitions in `commands.rs`.

---

## [0.4.0] - 2025-10-02

### Added

- Event pair aggregation features:
    - Derived **Pair** column when listing events (sequential pairing of `in` with next `out` per date, FIFO).
    - `--pairs <id>` filter to show only events (or summaries) for a specific pair id (per date).
    - `--summary` mode (only with `--events`) to display one aggregated row per pair (start, end, lunch, net duration,
      unmatched flag).
    - Enriched JSON output (`--events --json` and `--events --summary --json`) including fields: `pair`, `unmatched`,
      `lunch_minutes`, `duration_minutes` (summary mode).
- Unmatched event handling: lone `in` or `out` events are marked with an asterisk (`*`) after the pair id and
  `"unmatched": true` in JSON.
- Case‑insensitive normalization for `--pos` filter when listing events/sessions (`r` / `R` behave the same).
- Automatic dual-write: `add --in/--out` now (still) logs legacy session data and inserts punch events used by the new
  reporting features.
- New integration tests covering:
    - Pair calculation and filtering
    - Summary (basic, filtered, JSON, unmatched)
    - Enriched JSON schema
    - Case-insensitive position filter for events

### Changed

- Refactored event printing logic into helper functions (`compute_event_pairs`, `compute_event_summaries`,
  `summary/table`
  printers).
- Improved output alignment for event and summary tables.
- Internal minor cleanups (pattern matching adjustments for 2024 edition, separator printing, warning removal).

### Fixed

- Proper representation of unmatched events; they no longer appear merged with unrelated pairs.
- Prevented spurious formatting warnings in summary table output.

### Notes

- No breaking schema changes: all new capabilities are additive and backward‑compatible with existing databases.
- Legacy session listing (`list`) remains unchanged; new functionality is activated only with `--events`.

---

# [0.3.6] - 2025-09-30

### Added

- New `log` subcommand with the `--print` option to display rows from the internal `log` table (date, function,
  message). Useful for diagnostics and auditing internal operations.
- Application now records an entry into the internal `log` table on key user actions:
    - `init`: logs when a database/config is initialized (message: "Database initialized at <path>" or "Test DB
      initialized at <path>").
    - `add`: logs a concise summary of changes applied for the given date (message format:
      `date=YYYY-MM-DD | key=val, key=val, ...`, e.g. `date=2025-09-30 | start=09:00, lunch=30`).
    - `del`: logs deletions (message: `Deleted session id <id>`).

These log entries include a timestamp (ISO 8601) generated at insertion time and are intended for troubleshooting and
audit.

---

# [0.3.5] - 2025-09-30

### Added / Optimized

- Performance: use cached prepared statements (`Connection::prepare_cached`) for repeated queries and upserts (
  `db::list_sessions`, `db::get_session`, `db::upsert_*`, `db::ttlog`) to reduce SQL compilation overhead and speed up
  repeated CLI invocations.
- Migration to add new configuration parameter `separator_char` to the config file if missing; allows customizing the
  character used for month-end separators in list output.
- Integration test `test_separator_after_month_end` validating that a separator is printed after the month's last day.

### Changed

- Internal refactor and performance optimizations; bumped version to `v0.3.5`.
- Documentation updated: `README.md` now documents the `separator_char` configuration option and how to override it.

### Fixed

- Fixed a bug in the configuration migration (`migrate_to_033_rel`) where a variable was referenced out of scope,
  causing a compilation error in some environments; the migration now correctly serializes the updated configuration and
  writes it back.

---

# [0.3.4] - 2025-09-30

### Added

- Print the record inserted or updated when invoking the `add` command (the command now displays only the affected
  record).
- Configuration files for GitHub Copilot: `copilot-custom.json` (machine-readable) and `copilot-custom.md` (
  human-readable documentation).
- Bump project version to `v0.3.4` and update dependencies as required.

### Changed

- Updated dependencies and version metadata for the `v0.3.4` release.

---

# [0.3.3] - 2025-09-18

### Added

- New internal DB versioning system to handle schema evolution.
- New table schema_migrations to record each migration applied.
- Automatic check and execution of pending migrations every time a command is run.
- Automatic configuration file migration:
    - Adds missing parameters min_duration_lunch_break (default 30)
    - and max_duration_lunch_break (default 90)

### Changed

- The logic for expected exit time now uses configurable lunch break limits from the configuration file instead of
  hardcoded values.
- Improved conf --edit command:
    - If the requested editor does not exist, the application now falls back to the default editor ($EDITOR/nano on
      Linux/macOS, notepad on Windows) instead of panicking.

---

# [0.3.2] - 2025-09-17

### Added

- New command del to delete a work session by id from the work_sessions table.
- New working position C = On-Site (Client).
- Utility function to map working positions (O, R, C, H) into descriptive, colorized labels.
- Unit test for the new utility function.
- Integration tests for:
    - del command (successful and unsuccessful cases).
    - describe_position function.

### Changed

- Output of the list command updated:
- Supports the new working position C=On-Site (Client).
- Displays colorized working positions for better readability.
- Reformatted integration test outputs for consistency.
- Updated SQL in init command to support the new position C.
- Introduced migration function for release 0.3.2.

---

# [0.3.1] - 2025-09-17

### Added

- New global option `--pos` in the `list` command to filter sessions by working position:
    - `O` = Office
    - `R` = Remote
    - `H` = Holiday
- A function `make_separator` and `print_separator` in `utils.rs` to generate aligned separators with custom character,
  width, and alignment.
- Unit tests for `make_separator`.
- Integration test for the new `--pos` option of `list` command.
- Display of the **total surplus** (sum of daily surpluses) at the end of the `list` output.

### Changed

- Improved the output formatting of the `list` command, including:
    - aligned `Lunch` time using `HH:MM` or padded `-`
    - cleaner separator handling with the new utility functions.

---

# [0.3.0] - 2025-09-16

### Added

- New parameter `working_time` in application configuration file to define the daily working duration.
- Support for new position `H` (Holiday) in work sessions, with purple highlighted visualization in `list` command.
- Database migration mechanism to automatically upgrade schema when needed.
- Utility snippets in Rust to convert `NaiveDate` and `NaiveDateTime` to/from ISO 8601 text strings for SQLite
  compatibility.

### Changed

- Updated all calculation logic for expected exit time to use the configurable `working_time` parameter.
- Changed the visualization of lunchtime from number of minutes to `HH:MM` notation.
- Updated integration tests to validate:
    - usage of the new `working_time` parameter
    - new position `H` (Holiday)
    - DB migration functionality

---

## [0.2.5] - 2025-09-16

### Added

- New `conf` command to handle the configuration file
- `--print` option for `conf` to print the current configuration file
- `--edit` option for `conf` to edit the configuration file
- `--editor` option for `conf`, to be used with `--edit`, to specify which editor to use (supports `vim`, `nano`, or any
  custom path)
- Help messages for the new `conf` command and its options

### Changed

- Separated command implementations from `main.rs` into a new `commands.rs` source file

### Fixed

- Removed a stray debug print line

---

## [0.2.1] - 2025-09-15

### Added

- Support in `init` command for initializing a new database in:
    - an absolute path
    - directories containing spaces in their names

### Changed

- Updated `list` command: now shows the **expected end time** even when only the start time is provided for a given date
- Updated integration tests for the new version v0.2.1

### Fixed

- Prevented production config (`rtimelog.conf`) from being overwritten during integration tests by introducing `--test`
  global flag
- Ensured consistent DB path resolution when using `--db` together with `--test`

---

## [0.2.0] - 2025-09-14

### Added

- Creation of a configuration file in the user home (depending on platform) with:
    - DB filename
    - Default working position (`O`)
- New column `position` in SQLite DB to identify the working position of the day:
    - `O` = Office
    - `R` = Remote
- New options for `add` command:
    - `--pos` add the working position of the day, O = Office, R = Remote
    - `--in` add the start hour of work
    - `--lunch` add the duration of lunch
    - `--out` add the end hour of work
- Added global option `--db` to specify:
    - a DB name (created under rTimelog config directory)
    - or an absolute DB path
- Added a message when the DB is empty (`⚠️ No recorded sessions found`)

### Changed

- Reorganized the output of the `list` command
- Updated integration tests for new DB column `position`
- Updated the logic for opening the connection to the DB file
- Updated integration tests to use `--db` option

### Notes

- Previous intermediate changes introduced `--name` and config file handling,
  but they have been replaced by the new global `--db` approach for consistency.

---

## [v0.1.2] - 2025-09-12

### Added

- Added functionality to search records by year (`yyyy`) or year-month (`yyyy-mm`) using option `--period`.
- Added explicit `+` sign for positive surplus minutes.

### Changed

- Updated integration tests to cover new functionalities.

## [0.1.1] - 2025-09-12

### Added

- New workflow: `release.yml` for automated releases
- New workflow: `ci.yml` for multi-platform build and test (Linux, Windows, macOS Intel & ARM)
- Added Unit Test for Logic and Integration between DB and Logic

### Changed

- Updated `README.md` with badges and new documentation
- Fixed formatting issues detected by `cargo fmt`

### Removed

- Deleted obsolete workflow `.github/workflows/rust.yml`

## [0.1.0] - 2025-09-12

### Added

- Create CHANGELOG.md
- Create LICENSE
- Create rust.yml
- Create README.md
- Set origin language in English
- If the date parameter is empty, assume the current date
- Initial version of the project
