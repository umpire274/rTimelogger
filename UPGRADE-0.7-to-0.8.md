# ⬆️ Upgrade Guide — rTimelogger 0.7.x → 0.8.0

This document describes **breaking changes**, **migration behavior**, and **recommended actions**
when upgrading **rTimelogger** from any **0.7.x** release to **0.8.0**.

> ⚠️ This upgrade introduces **major internal and CLI changes**.
> Reading this document is **strongly recommended** before upgrading.

---

## 📌 Overview of Major Changes

Version **0.8.0** represents a **structural rewrite** of rTimelogger.

Key changes include:

- New **event-based timeline model**
- Removal of the legacy `work_sessions` table
- Fully rewritten `list` command
- New **pair-based logic** (IN/OUT pairs)
- Accurate handling of **working and non-working gaps**
- Redesigned `add` command syntax
- Improved database migration system

---

## 🧱 Database Changes

### ✅ Removal of `work_sessions`

In `0.8.0`, the legacy table:

```sql
work_sessions
```

is **no longer used**.

All calculations are now derived from the **raw** `events` **table** via the **Timeline Model**:

```lua
events → timeline → pairs → gaps → totals
```

During migration:

- A **backup of the database** is automatically created
- The **work_sessions** table is **removed only if upgrading from < 0.8.0**
- All IN/OUT events are **re-paired and recalculated**

No manual action is required.

---

## 🧠 Pair Recalculation

All events are reprocessed to:

- Assign correct pair numbers
- Restore chronological consistency
- Handle incomplete pairs safely (IN without OUT)

A dedicated internal utility ensures correctness:

```rust
rebuild_all_pairs()
```

---

## 🔗 New work_gap Field

A new column has been added to the events table:

```sql
work_gap
BOOLEAN DEFAULT 0
```

This field applies to **OUT events only** and controls whether the gap
between this OUT and the next IN is considered **working time**.

---

## 🧮 Expected & Surplus Calculation Changes

### **Old behavior (0.7.x)**

- Worked time implicitly assumed gaps were working
- Complex days with multiple sessions could yield incorrect surplus
- Lunch handling was partially duplicated across code paths

### **New behavior (0.8.0)**

- **Non-working gaps are excluded** from worked time
- Only gaps explicitly marked with `--work-gap` are included
- Surplus is now calculated as:

```rust
surplus = actual_worked_minutes – expected_minutes;
```

This yields **correct results even for fragmented days**.

---

## ✏️ Work Gap Management

You can now explicitly control gaps between pairs.

### **Mark a gap as working**

```bash
rtimelogger add 2025-12-12 --out 09:30 --work-gap
```

### **Remove a working gap (edit mode)**

```bash
rtimelogger add 2025-12-12 --edit --pair 2 --no-work-gap
```

Each change:

- Updates the database
- Triggers a full recalculation
- Shows a clear confirmation message:

    - 🔗 _work gap added_
    - ✂️ _work gap removed_

---

## 🧾 CLI Breaking Changes

### 🔴 `add` **command — positional arguments removed**

#### `0.7.x` (deprecated):

```bash
rtimelogger add 2025-09-13 O 09:00 60 17:30
```

#### `0.8.0` (required):

```bash
rtimelogger add 2025-09-13 --pos O --in 09:00 --lunch 60 --out 17:30
```

Key rules:

- The **date is the only positional argument**
- All other values must be passed as flags
- Edit operations require `--edit --pair N`

---

### 🔄 Editing pairs

```bash
rtimelogger add 2025-12-03 --edit --pair 1 --out 15:30
```

Only the explicitly provided fields are modified.

---

### 📋 list Command Rewrite

The `list` command is now entirely based on the **Timeline Model**.

Improvements include:

- Accurate surplus computation
- Per-pair position display in `--details`
- Consistent output across:
    - list
    - list --today
    - list --events

- Automatic month separators for long ranges
- Cleaner ANSI color usage

---

### `--details` flag restrictions

Starting from **v0.8.0**, the `list --details` flag is restricted to
**single-day output only**.

Valid usages:

```bash
rtimelogger list --today --details
rtimelogger list --period 2025-12-12 --details
```

Invalid usages (no longer supported):

```bash
rtimelogger list --details
rtimelogger list --period 2025-12 --details
rtimelogger list --period 2025
rtimelogger list --period 2025-12-01:2025-12-10 --details
rtimelogger list --period all --details
```

#### Why this change?

In rTimelogger **0.8.x**, the `list` command is built on the **Timeline model**:

```lua
events → timeline → pairs → gaps → daily summary
```

The `--details` view operates at **pair-level**, showing:

- individual IN/OUT pairs
- worked time per pair
- lunch per pair
- working gaps between pairs
- pair-specific working position

Displaying this information across **multiple days** would be ambiguous and misleading.

For this reason, `--details` is now intentionally limited to cases where the output refers to **exactly one day**.

This is a **deliberate breaking change** compared to `0.7.x`, introduced to improve correctness and clarity.

---

## 🗄 Migration Safety

When running any command on an existing `0.7.x` database:

1. A backup is created automatically
2. Schema migrations are applied
3. Events are reprocessed
4. Legacy tables are removed safely

You can inspect migration status via:

```bash
rtimelogger db --info
```

---

## ✅ Recommended Post-Upgrade Checks

After upgrading, it is recommended to:

```bash
# Check database integrity
rtimelogger db --check

# Review a few historical days
rtimelogger list --period 2025-01 --details

# Verify surplus totals
rtimelogger list --period all
```

If results differ from `0.7.x`, this is **expected** and reflects more accurate gap and lunch handling.

---

## ⚠️ Known Differences vs 0.7.x

| Area               | 	Difference                             |
|--------------------|-----------------------------------------|
| Surplus total      | May change due to correct gap exclusion |
| Multi-session days | Now calculated accurately               |
| CLI syntax	        | Positional args removed                 |
| DB schema	         | Legacy tables removed                   |
| Internals	         | Fully rewritten                         |

These are **intentional and documented changes**.

---

## 🧭 Final Notes

- `0.8.0` is the **new stable baseline**
- Future versions will **not support 0.7.x behavior**
- This upgrade resolves long-standing calculation edge cases

If you encounter unexpected behavior, please:

- Verify the timeline with `list --details`
- Check gap flags (`work_gap`)
- Open an issue with example data if needed
