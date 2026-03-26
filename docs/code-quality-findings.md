# Code Quality Findings (Round 2)

Audit date: 2026-03-26. Previous findings from Round 1 are archived below.

## HIGH Severity

### H1. Wildcard match arm hides future enum variants in proto_annotations_to_json [DONE]
- **File:** `visualizer/src/app.rs:310-311`
- **Problem:** The match on `RegionType` uses `_ => continue` after explicitly listing `ProtoTag` and `ProtoLength`. This wildcard catches `ProtoMessage` -- which should never appear here since we are iterating children of a message -- but also silently swallows any future enum variants added to `RegionType`. This violates exhaustive switch handling.
- **Fix:** Replace the wildcard with an explicit `RegionType::ProtoMessage { .. } => continue` to make the match exhaustive. This ensures any new variant added to `RegionType` triggers a compile error here.

### H2. Duplicate match arms for ProtoBytes/ProtoString colors in region.rs [DONE]
- **File:** `visualizer/src/region.rs:68-69`
- **Problem:** `ProtoBytes` and `ProtoString` have identical color values `[218, 165, 32]` as separate match arms. This is either intentional (they share a color) or a copy-paste bug (they should have distinct colors). Either way the arms should be merged.
- **Fix:** Merge into `RegionType::ProtoBytes { .. } | RegionType::ProtoString { .. } => [218, 165, 32]`.

### H3. Redundant duplicate color computation in byte_style [DONE]
- **File:** `visualizer/src/hex_view.rs:232-241`
- **Problem:** `base_color` and `base_rgb` compute the same thing independently -- one as `Color32` and one as `[u8; 3]`. The `base_color` is only used in the `tier == 0` fallback. This means we compute the color twice for every byte, and `base_color` is unused when the tier is 1 or 2.
- **Fix:** Compute `base_rgb` once, derive `base_color` from it only in the tier-0 branch.

## MEDIUM Severity

### M1. `let...else` pattern not used in looks_like_proto_message [DONE]
- **File:** `visualizer/src/proto_walker.rs:148-151, 171-174`
- **Problem:** Two `match read_varint(...) { Ok(v) => v, Err(_) => return false }` patterns should use idiomatic `let...else`.
- **Fix:** Replace with `let Ok((tag, tag_len)) = read_varint(data, offset) else { return false };`

### M2. Use `u64::from()` instead of `as u64` for widening cast [DONE]
- **File:** `visualizer/src/proto_walker.rs:103`
- **Problem:** `(b & 0x7F) as u64` is a widening cast from `u8` to `u64`. While safe, `u64::from(b & 0x7F)` is more idiomatic and makes intent explicit.
- **Fix:** Change to `u64::from(b & 0x7F)`.

### M3. Redundant closure in effect logging [DONE]
- **File:** `visualizer/src/state.rs:377`
- **Problem:** `.map(|e| e.to_string())` is a redundant closure that can be replaced with a method reference.
- **Fix:** Use `.map(ToString::to_string)`.

### M4. Use `clone_from` instead of `clone()` assignment [DONE]
- **File:** `visualizer/src/state.rs:367`
- **Problem:** `self.data_text = hex_data.clone()` allocates a new String. `clone_from` can reuse the existing buffer.
- **Fix:** Change to `self.data_text.clone_from(&hex_data)`.

### M5. Unnecessary boolean negation in SwitchDataFormat [DONE]
- **File:** `visualizer/src/state.rs:239-243`
- **Problem:** `if !self.decoded_json.is_empty() { ... } else { ... }` is a negated condition with an else branch. The positive case should come first.
- **Fix:** Flip the condition: `if self.decoded_json.is_empty() { "{}".to_string() } else { clone }`.

### M6. Missing backticks in doc comments [DONE]
- **File:** `visualizer/src/permalink.rs:6-8`, `visualizer/src/proto_walker.rs:60,67,93`
- **Problem:** Several doc comments reference code identifiers (`schema_text`, `data_text`, `field_path`, `AnnotatedRegion`, `bytes_consumed`) without backticks, which breaks rustdoc rendering.
- **Fix:** Add backticks around identifiers in doc comments.

### M7. Inclusive range readability in syntax.rs [DONE]
- **File:** `visualizer/src/syntax.rs:109,114`
- **Problem:** `text[i..i + 1]` is less readable than `text[i..=i]` for single-character slices.
- **Fix:** Change to `text[i..=i]`.

### M8. Use `map_or` instead of `map().unwrap_or()` in byte_style [DONE]
- **File:** `visualizer/src/hex_view.rs:232-241`
- **Problem:** Two `map(...).unwrap_or(...)` chains should use `map_or` for clarity.
- **Fix:** Use `map_or`.

### M9. Capture variables directly in format strings [DONE]
- **File:** `visualizer/src/hex_view.rs:136`
- **Problem:** `format!("{:04X}: ", row_start)` should use `format!("{row_start:04X}: ")`.
- **Fix:** Inline the variable into the format string.

## LOW Severity (cosmetic / pedantic -- no action)

### L1. Similar variable names as_u64/as_f64 and as_u32/as_f32
- **File:** `visualizer/src/proto_walker.rs:267-268, 309-310`
- **Status:** SKIPPED -- the names are intentionally similar to show the same bytes interpreted as different types.

### L2. Function too many lines: walk_message (237 lines) and dispatch (174 lines)
- **File:** `visualizer/src/proto_walker.rs:195`, `visualizer/src/state.rs:196`
- **Status:** SKIPPED -- these are complex state machines. Splitting them would hurt readability more than help.

### L3. Various `cast_possible_truncation` warnings on u64->u32 and u64->usize
- **File:** Multiple locations in `proto_walker.rs` and `permalink.rs`
- **Status:** SKIPPED -- protobuf field numbers and lengths are inherently bounded. Adding `try_from` with error handling everywhere would add noise without safety benefit since protobuf wire format guarantees these fit.

### L4. `unused_self` on poll_platform_events for non-wasm target
- **File:** `visualizer/src/app.rs:168`
- **Status:** SKIPPED -- the function uses `self` on wasm target. The cfg gating makes this correct.

---

## Round 1 Findings (archived, all resolved)

Previous findings from the initial code quality audit have been addressed.
Key items completed: push_region/field_path helpers (1.1), dead FBS code removal (2.2),
byte_at_position extraction (1.3), bytes_to_hex/GenConfig dedup (1.4, 1.5),
AnnotatedRegion::new constructor (5.2), brighten helper (5.1), various #[allow] cleanups.
