# Code Quality Findings

## 1. Duplication

### 1.1 Duplicated AnnotatedRegion construction pattern in proto_walker.rs [DONE]
- **Location:** `visualizer/src/proto_walker.rs:199-489` (walk_message)
- **Problem:** Every wire-type branch (varint at 199-242, fixed64 at 243-296, fixed32 at 297-350, length-delimited at 351-489) repeats the same boilerplate for constructing `AnnotatedRegion` structs. Each branch builds: (a) a value-level region with nearly identical field_path construction (`let mut p = ctx.path.clone(); p.push(field_name.clone()); p`), (b) a group-level region wrapping tag + value children. The field_path construction alone is copy-pasted 12 times.
- **Fix:** Extract a helper method on `WalkContext` such as `fn push_region(&mut self, byte_range, region_type, label, field_name, value_display, children, depth) -> usize` that encapsulates the repetitive `AnnotatedRegion` construction and `ctx.regions.push()`. Also extract `fn field_path(&self, field_name: &str) -> Vec<String>` to eliminate the path-building boilerplate.
- **Resolution:** Added `WalkContext::push_region()` and `WalkContext::field_path()` helper methods. All 13 construction sites now use `AnnotatedRegion::new` via `push_region()` and `field_path()` eliminates the path-building boilerplate.

### 1.2 Duplicated syntax highlighting logic between highlight_fbs and highlight_proto [DONE - MOOT]
- **Location:** `visualizer/src/syntax.rs:56-170` (highlight_fbs) and `visualizer/src/syntax.rs:326-437` (highlight_proto)
- **Problem:** These two functions share nearly identical structure: line comment parsing, block comment parsing, string literal parsing, number literal parsing (including hex and scientific notation), identifier/keyword classification, and fallback character handling. The only differences are the keyword/type lists and that proto supports single-quoted strings. This is ~170 lines of duplicated parsing logic.
- **Fix:** Extract a generic `highlight_with_config(text, font, wrap_width, config: &HighlightConfig)` function where `HighlightConfig` holds the keyword list, type list, constants list, and whether single-quoted strings are supported. Both `highlight_fbs` and `highlight_proto` become thin wrappers calling this shared function.
- **Resolution:** Moot -- `highlight_fbs` was removed entirely as dead code (see 2.2). Only `highlight_proto` remains, so there is no duplication to extract.

### 1.3 Duplicated hex cursor detection logic in hex_view.rs [DONE]
- **Location:** `visualizer/src/hex_view.rs:42-62` (hover detection) and `visualizer/src/hex_view.rs:66-85` (click detection)
- **Problem:** The hover and click branches contain identical logic for computing which byte is under the cursor: get position, compute char_width, subtract address prefix width, divide by byte column width, bounds-check, and look up `byte_to_region`. The only difference is one reads `response.hover_pos()` and the other reads `ui.input(|i| i.pointer.interact_pos())`.
- **Fix:** Extract a `fn byte_at_position(pos: egui::Pos2, rect: egui::Rect, ui: &Ui, row_start: usize, data_len: usize, byte_to_region: &[Option<usize>]) -> Option<usize>` helper, called from both branches.
- **Resolution:** Extracted `byte_at_position()` helper; both hover and click paths now call it.

### 1.4 Duplicated bytes_to_hex formatting [DONE]
- **Location:** `visualizer/src/state.rs:430-435` (bytes_to_hex function)
- **Also at:** `visualizer/src/app.rs:129-134` (inline in GenerateRandomSchemaAndData effect)
- **Also at:** `visualizer/src/state.rs:470-488` (test helper execute_effect_sync)
- **Problem:** The `format!("{b:02x}")` + `collect::<Vec<_>>().join(" ")` pattern for converting bytes to hex is written inline in the effect handler and test helper instead of reusing the existing `bytes_to_hex` function.
- **Fix:** Replace the inline formatting in `app.rs:129-134` and `state.rs:484-488` with calls to `crate::state::bytes_to_hex(&generated.binary_data)`.
- **Resolution:** Both `app.rs` and the test helper in `state.rs` now call `bytes_to_hex()`.

### 1.5 Duplicated GenConfig construction [DONE]
- **Location:** `visualizer/src/app.rs:117-127` (execute_effect in VisualizerApp)
- **Also at:** `visualizer/src/state.rs:471-481` (execute_effect_sync in tests)
- **Problem:** The `GenConfig` struct with its 8 fields is identically constructed in two places. If default values need to change, both must be updated.
- **Fix:** Define a `fn default_gen_config() -> protoc_rs_proto_gen::GenConfig` function in a shared location (e.g., `state.rs`) and call it from both sites.
- **Resolution:** Added `default_gen_config()` in `state.rs`; both `app.rs` and the test helper now call it.

## 2. Dead Code / Suppressed Warnings

### 2.1 File-level #[allow(dead_code)] on syntax.rs [DONE]
- **Location:** `visualizer/src/syntax.rs:1`
- **Problem:** The file-level `#![allow(dead_code)]` suppresses all dead code warnings for the entire module. This hides the fact that `highlight_fbs` is never called anywhere in the codebase (see next finding). Blanket `#[allow]` attributes mask real issues.
- **Fix:** Remove the file-level `#![allow(dead_code)]` and add targeted `#[allow(dead_code)]` only on individual items if truly needed, or delete unused functions.
- **Resolution:** Removed `#![allow(dead_code)]` from syntax.rs. No targeted allows needed after removing dead FBS code.

### 2.2 highlight_fbs function is never called [DONE]
- **Location:** `visualizer/src/syntax.rs:56-170` (highlight_fbs)
- **Problem:** `highlight_fbs` is a public function for FlatBuffers syntax highlighting but is never called anywhere in the codebase. The project is a Protobuf visualizer, not a FlatBuffers one. This is ~115 lines of dead code, along with the associated `FBS_KEYWORDS` and `FBS_TYPES` constants (~18 lines).
- **Fix:** Remove `highlight_fbs`, `FBS_KEYWORDS`, and `FBS_TYPES` entirely. If FlatBuffers support is planned for the future, track it as a task rather than shipping dead code.
- **Resolution:** Removed `highlight_fbs`, `FBS_KEYWORDS`, and `FBS_TYPES` entirely.

### 2.3 #[allow(dead_code)] on cjk_font_loaded field [DONE]
- **Location:** `visualizer/src/app.rs:17-18`
- **Problem:** `#[allow(dead_code)]` suppresses the warning that `cjk_font_loaded` is unused on non-wasm targets. On native, the field is written in the constructor but never read from any non-wasm code path. This is a real dead-code issue being silenced rather than fixed.
- **Fix:** Gate the field with `#[cfg(target_arch = "wasm32")]` on both the field declaration and its usage in `poll_platform_events`, or use a single `#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]` with a comment explaining why.
- **Resolution:** Replaced with `#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]` and added a comment explaining the field is only read on wasm32.

### 2.4 #[allow(dead_code)] on Command enum [DONE]
- **Location:** `visualizer/src/state.rs:28-29`
- **Problem:** The `#[allow(dead_code)]` on the `Command` enum suppresses warnings for any variant that might not be constructed. This could hide genuinely unused variants over time.
- **Fix:** Remove the blanket `#[allow(dead_code)]` and verify all variants are used. `RandomGenerateError` at line 60 appears to never be constructed anywhere in the codebase -- it should either be removed or have a targeted `#[allow]` with a TODO.
- **Resolution:** Removed `#[allow(dead_code)]` from `Command`. Also removed the unused `RandomGenerateError` variant (see 2.6). No warnings remain.

### 2.5 #[allow(dead_code)] on EventLogEntry [DONE]
- **Location:** `visualizer/src/state.rs:119`
- **Problem:** `#[allow(dead_code)]` on `EventLogEntry` suppresses the warning that its fields are written but never read. The event log is populated in `dispatch()` but there is no UI or export that reads it. The struct fields `command` and `effects` are truly never consumed.
- **Fix:** Either implement the event log viewer UI (the data is there, just never displayed), or remove the event logging code entirely if it is not planned. Currently it allocates strings and a VecDeque on every dispatch with no consumer. **Low priority** -- this is useful for debugging.
- **Resolution:** Kept `#[allow(dead_code)]` but added a TODO comment explaining the fields are populated but not yet consumed, kept for future event log viewer.

### 2.6 RandomGenerateError variant is never constructed [DONE]
- **Location:** `visualizer/src/state.rs:60`
- **Problem:** The `Command::RandomGenerateError(String)` variant is declared but never constructed anywhere in the codebase. Both the app effect handler (`app.rs`) and test helper (`state.rs`) always produce `RandomGenerated` on success, and random generation never produces an error path that would construct `RandomGenerateError`.
- **Fix:** Remove the variant, or add the missing error path in the random generation effect handler in `app.rs`.
- **Resolution:** Removed the variant and its Display impl and dispatch handler.

## 3. Unnecessary Clones

### 3.1 Excessive .clone() on field_name in proto_walker.rs [SKIPPED]
- **Location:** `visualizer/src/proto_walker.rs:209-234` (varint), `262-289` (fixed64), `316-343` (fixed32), `389-484` (length-delimited)
- **Problem:** `field_name` is a `String` that gets `.clone()`'d 4-6 times per field within each wire-type branch (for `RegionType` construction, `field_path` construction, label formatting). Since `field_name` is constructed at the top of the loop and not used after the branch, the final usage could take ownership instead of cloning.
- **Fix:** Use `field_name.clone()` only where intermediate uses require it, and pass the final usage by move. Alternatively, change `field_name` to `&str` and only allocate `String` at the point of insertion into the region struct.
- **Resolution:** Skipped -- the `push_region` refactor (1.1) already reduced the number of clone sites. The remaining clones are needed because `field_name` is used in multiple regions within the same branch. Changing to `&str` would require restructuring the RegionType enum variants to hold owned Strings differently. The performance impact is negligible for this UI application.

### 3.2 Unnecessary clone of decoded_json in ProtoWalked handler [DONE]
- **Location:** `visualizer/src/state.rs:347`
- **Problem:** `self.decoded_json = decoded_json.clone();` clones the string, then on line 352, `self.data_text = decoded_json;` moves the original. The clone is unnecessary -- the order can be swapped to avoid it.
- **Fix:** Swap the assignment order: first `self.data_text = decoded_json;` (conditional), then `self.decoded_json = self.data_text.clone();` (if needed), or restructure to clone only when the JSON format branch is taken.
- **Resolution:** Restructured to clone only in the JSON format branch: `if Json { clone then move } else { move }`.

## 4. Unsafe Patterns

### 4.1 Multiple .unwrap() calls in wasm file upload handler [SKIPPED]
- **Location:** `visualizer/src/app.rs:238-278` (trigger_file_upload)
- **Problem:** There are 9 `.unwrap()` calls in `trigger_file_upload` on DOM operations (`window.document().unwrap()`, `create_element().unwrap()`, `.dyn_into().unwrap()`, etc.). While these are unlikely to fail in a browser, any failure produces a panic with no actionable error message. One `.lock().unwrap()` on line 265 could panic if the mutex is poisoned.
- **Fix:** For DOM operations that truly cannot fail (e.g., `window()` in a browser context), `.expect("descriptive message")` is acceptable. For the mutex lock on line 265, use `.lock().ok()` with a fallback, or use `try_lock()` as is done elsewhere in the file (line 187). **Low priority** -- this is WASM-only code where panics become console errors.
- **Resolution:** Skipped -- low priority, wasm-only code. The DOM operations truly cannot fail in a browser context and panics become console errors.

### 4.2 .unwrap() on serde_json serialization [NO ACTION NEEDED]
- **Location:** `visualizer/src/app.rs:392`
- **Problem:** `serde_json::to_string_pretty(&json_value).unwrap_or_default()` -- while this uses `unwrap_or_default` which is safe, the JSON value is constructed manually and could theoretically produce a serialization error. The current handling silently returns an empty string.
- **Fix:** This is actually handled correctly via `unwrap_or_default`. No change needed. **Cosmetic only.**
- **Resolution:** No action needed -- already correctly handled.

## 5. Missing Abstractions

### 5.1 Color brightening logic duplicated between hex_view and structure_view [DONE]
- **Location:** `visualizer/src/hex_view.rs:230-254` (byte_style)
- **Also at:** `visualizer/src/structure_view.rs:74-96` (render_tree_node)
- **Problem:** Both modules independently compute "brightened" colors by calling `saturating_add(60)` for locked and `saturating_add(30)` for hovered states. The exact same color manipulation logic is used in both places.
- **Fix:** Add a `fn brighten(color: [u8; 3], amount: u8) -> Color32` helper to `region.rs` (or a new `colors.rs` module) and call it from both modules. The highlight tier concept (locked=bright, hovered=medium) should be a shared abstraction.
- **Resolution:** Added `brighten()` helper to `region.rs` and updated both `hex_view.rs` and `structure_view.rs` to use it.

### 5.2 No builder or constructor for AnnotatedRegion [DONE]
- **Location:** `visualizer/src/region.rs:4-12` (AnnotatedRegion struct)
- **Problem:** `AnnotatedRegion` has 7 fields, all constructed inline at every call site in `proto_walker.rs` (13 construction sites). This leads to verbose, error-prone construction where forgetting a field causes a compile error but field ordering is easy to get wrong.
- **Fix:** Add a constructor `AnnotatedRegion::new(byte_range, region_type, label, field_path, value_display, children, depth)` or use a builder pattern. Given the number of construction sites, even a simple constructor would reduce noise.
- **Resolution:** Added `AnnotatedRegion::new()` constructor and updated all construction sites in `proto_walker.rs` via `WalkContext::push_region()`.

## 6. Noise / Comments

### 6.1 Excessive section dividers [PARTIALLY DONE]
- **Location:** `visualizer/src/app.rs` (lines 9-11, 282-284, 302-304, 395-397, 409-411)
- **Also at:** `visualizer/src/state.rs` (lines 13-15, 87-89, 112-114, 125-127, 426-428, 437-439)
- **Also at:** `visualizer/src/view.rs` (lines 9-11, 86-88, 203-205, 319-321)
- **Also at:** `visualizer/src/syntax.rs` (lines 10-12, 27-29, 51-53, 172-174, 283-285, 321-323, 439-441)
- **Problem:** The codebase uses `// -------...-------` section dividers extensively (20+ occurrences across files). In files under 500 lines, these add visual noise without aiding navigation -- modern editors and `mod` organization serve the same purpose.
- **Fix:** Remove section dividers. Use doc comments on functions/structs instead. **Low priority / cosmetic.**
- **Resolution:** Removed section dividers from `syntax.rs` as part of the dead code cleanup. Remaining dividers in other files left as-is (low priority cosmetic change).

## 7. Over-Architecture

### 7.1 Unused DataFormat::Json path in CompileAndEncode [SKIPPED]
- **Location:** `visualizer/src/state.rs:219-229` (CompileAndEncode handler)
- **Problem:** The `CompileAndEncode` command has a JSON format branch (lines 223-224) that falls through to reading `self.binary_data` if the format is JSON. But the JSON data is never compiled -- it is always parsed from hex bytes. The "Compile & Encode" button name and the JSON branch suggest planned schema compilation that was never implemented. The current logic for JSON format is confusing: it reads `self.binary_data` (which might be stale from a previous operation) rather than doing anything with the JSON text.
- **Fix:** Clarify the intent. If JSON-to-binary compilation is not planned, simplify this to always use the hex data text, or convert the JSON format branch to an error/no-op with a clear message. **Medium priority** -- this is a source of user confusion.
- **Resolution:** Skipped -- this requires understanding the product intent (whether JSON-to-binary compilation is planned). Changing the behavior could break the current "re-walk existing binary when in JSON mode" workflow. Needs product decision.

## 8. Stringly-Typed APIs

### 8.1 Error messages as strings throughout state.rs [SKIPPED]
- **Location:** `visualizer/src/state.rs:55` (ProtoWalkError), `visualizer/src/state.rs:60` (RandomGenerateError)
- **Problem:** `Command::ProtoWalkError(String)` and `Command::RandomGenerateError(String)` use raw strings for errors. The `AppState` error field (`pub error: Option<String>` at line 141) is also a raw string. While this works, it prevents programmatic error handling (e.g., distinguishing parse errors from walk errors).
- **Fix:** Consider using an enum for error categories if different error types need different UI treatment in the future. **Low priority** -- for a UI application, string errors shown to users are acceptable.
- **Resolution:** Skipped -- low priority for a UI application. String errors displayed to users are acceptable. `RandomGenerateError` variant was removed (see 2.6).

## 9. Minor Issues

### 9.1 font.clone() on every TextFormat construction in hex_view.rs [NO ACTION NEEDED]
- **Location:** `visualizer/src/hex_view.rs:130, 144, 159, 175, 203` (render_hex_row)
- **Problem:** `FontId::monospace(13.0)` is created at line 123, then `.clone()`'d 5 times in the same function for each `TextFormat`. `FontId` is small (contains a `f32` and an enum), so this is not a performance issue, but it is unnecessary since `FontId` implements `Clone` cheaply and could be constructed inline or the `TextFormat` could reference it differently.
- **Fix:** No action needed -- `FontId` is cheap to clone. **Cosmetic only.**
- **Resolution:** No action needed.

### 9.2 proto_templates::all() allocates nothing but is called repeatedly [NO ACTION NEEDED]
- **Location:** `visualizer/src/proto_templates.rs:8-19` (all function)
- **Called from:** `visualizer/src/state.rs:171, 203, 226-227` and `visualizer/src/view.rs:214`
- **Problem:** `all()` returns `&'static [ProtoTemplate]`, which is zero-cost. No issue here -- this is well-designed.
- **Fix:** No action needed. This is noted for completeness.
- **Resolution:** No action needed.
