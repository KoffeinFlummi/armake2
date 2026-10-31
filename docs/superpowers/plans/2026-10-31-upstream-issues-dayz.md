# Upstream armake2 Issues Affecting DayZ: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the open KoffeinFlummi/armake2 issues that still reproduce in this fork and affect DayZ mods. "Correct" means matching what BI's own DayZ Tools do.

**Architecture:** Small, independent fixes in `preprocess.rs`, `config.rs`, `pbo.rs`, `sign.rs`, `io.rs`. Each fix gets a regression test, and where possible the test is a golden test against output from BI's tools, stored in `tests/fixtures/bi/` (see its README). CLI behaviour (warnings, exit codes, panics) is tested through the built binary in a new `tests/cli.rs`.

**Tech Stack:** Rust 2018, `peg` grammars, `docopt`, `tempfile`, plus `encoding_rs` (new, Task 6 only).

**Spec:** this document. Sources: https://github.com/KoffeinFlummi/armake2/issues, re-run against fork `782396e` on 2026-10-31 and compared with DayZ Tools (CfgConvert, DSSignFile, DSCheckSignatures) under Proton Hotfix.

---

## Triage

"BI" = what DayZ Tools do with the same input (run under Proton). All armake2 results are from a release build of `782396e`.

| Prio | Issue | armake2 today | BI | Task |
|---|---|---|---|---|
| **P1** | **(new) Unquoted string values** (`a = foo;`) | `config.rs:42` `![';'\|'}'] [_]*` checks only the first character and then swallows **the rest of the file**: rapify fails, or the config is wrong | `"foo"`. The value ends at `;`, `}` or a newline (`g = foo⏎h = 1;` gives two entries) | 1 |
| **P1** | (found via #23) Warning on line 1 | Panic `index out of bounds` at `config.rs:630`. Every warning is printed twice. | — | 2 |
| **P1** | #15 Already-rapified files | `build` rapifies every `.rvmat`/`.cpp`, so a vanilla binarized rvmat breaks the build with a garbled error | CfgConvert accepts rapified input and passes it through | 3 |
| **P1** | #51 `#include "x.hpp"  // c` | Hard error | Works | 4 |
| **P2** | #28 Macro argument-count mismatch (`QUOTE(a,b)`, `QUOTE([1,2])`) | Left **unexpanded**, with no message | Expands to `""`, also with no message | 5 |
| **P2** | #34 Non-UTF-8 entry names | `inspect`/`unpack`/`cat`/`verify`/`sign` panic (`io.rs:66`) | DSSignFile signs and verifies them; reference bisign in fixtures | 6 |
| **P2** | #42 `$PBOPREFIX$` with a trailing slash | Written to the header unchanged; the game can't find files | (Addon Builder not run under Proton) | 7 |
| **P2** | #17/#14 Include resolution | Case mismatches fail on Linux. A missing `-i` folder gives an error without its name, and `read_dir().unwrap()` can panic. | Case-insensitive (Windows) | 8 |
| **P3** | #23 `__EVAL`/`__EXEC` | Misleading "not quoted" warning (+ line-1 panic) | **Evaluated**: `__EVAL(1+2*3)`→`7`, `"x"+"y"`→`"xy"`; `__EXEC(zz=5)` sets variables for later `__EVAL` | 9 |
| **P3** | #39 Lone `\r` newlines | Parse error | Works | 10 |
| **P2** | #21 Opaque textures after binarize | `--proton-binarize` copies only the `.p3d` into the sandbox and runs binarize.exe with the current directory `<sandbox>/Binarize`. binarize resolves face textures relative to the current directory (strace: it looks for `z:/Binarize/t21/data/glass_ca.paa`), finds none, and leaves the alpha flags at **0**: glass, foliage and alpha clothing render opaque | With the current directory at the project root (`P:\`) it finds the `.paa`, and the model and section flags become **`0x802020`** (IsAlpha\|IsTransparent\|IsAlphaFog). The ODOL is otherwise byte-identical. | 11 |
| P4 | #54 Whole PBO in RAM | Applies | — | Deferred |
| — | #55 sign with `<signature>` path | **Works.** armake2's signature is byte-identical to DSSignFile's, and `DSCheckSignatures -deep` accepts armake2-signed PBOs with `.c` scripts | — | Already fixed |
| — | #61 img2paa, #60 releases | Work / exist | — | Already fixed |
| — | #56 prefix when packing | Folder-name default is right for DayZ mods | — | Won't fix |
| — | #13, #18, #44, #47, #52, #58 | Diagnostics/API wishes | — | Out of scope |

Rapify output is not byte-identical to CfgConvert: BI writes an extra 4-byte offset after every class body. Decoded, the configs are identical, and the game loads armake2's output. This is not a bug, so tests compare decoded configs.

## Global Constraints

- Build/test with `CARGO_TARGET_DIR=/tmp/armake2-target cargo test` (never build under `$HOME`). Test scratch files go in `tempfile::tempdir()`.
- No new dependencies except `encoding_rs` (Task 6).
- BI behaviour in `tests/fixtures/bi/` is the reference. Never edit an `*.expected.cpp` or a `.bisign` to make a test pass. If BI's behaviour is deliberately not copied, the test says so in a comment.
- New warnings go through `error::warning(msg, Some(name), location)`. Names: `prerapified-file`, `macro-arg-count`, `include-case`, `prefix-slash`, `non-utf8-filename`.
- Signing must stay byte-identical to DSSignFile (`tests/fixtures/bi/sign/testsig.pbo.armk.bisign`).
- One commit per task. Messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. **Signature bytes for non-UTF-8 names:** armake2 must hash the raw name bytes. The test is `sign_matches_bi_non_utf8` (Task 6), which compares byte-for-byte against BI.
2. **Unquoted string inside an array or before `}` without `;`:** `{foo, bar baz, 3}` and `class A { x = foo };` must still parse. Covered by golden `us` and test `unquoted_before_brace` (Task 1).
3. **A pre-rapified `config.cpp`:** must be stored as `config.bin` unchanged and not rapified twice. Test `build_keeps_prerapified_config_cpp` (Task 3).
4. **Case-insensitive include ambiguity:** an exact-case match must win. Test `include_exact_case_wins` (Task 8).
5. **Object-like macro followed by parentheses** (`#define FOO bar` then `FOO(1)`): must give `bar(1)` with no arity warning. Covered by golden `ma` and test `object_like_macro_no_arity_warning` (Task 5).

---

### Task 0: Test helpers

**Files:**
- Create: `tests/cli.rs`, `tests/bi_golden.rs`
- Add (already generated, untracked): `tests/fixtures/bi/**`

**Interfaces:**
- Produces in `tests/cli.rs`: `fn armake2(args: &[&str], cwd: &Path) -> (i32, String, String)` returning (exit code, stdout, stderr), using `env!("CARGO_BIN_EXE_armake2")`. A panic shows up as 101.
- Produces in `tests/bi_golden.rs`: `fn golden(name: &str)`. It rapifies `tests/fixtures/bi/cfgconvert/<name>.cpp` with the fixture dir as include root, derapifies the result (library API: `Config::read` + `to_cursor`, then `Config::read_rapified` + `write`), and `assert_eq!`s it against `<name>.expected.cpp`. Then one `#[test] fn golden_<name>()` per fixture: `g, w1, us, c51, rap, ma, m28, case, ev, ex, e23, mac`. Mark each one `#[ignore]` until its task removes the attribute. Only `golden_g` (the baseline) starts un-ignored.

- [ ] **Step 1:** Write the helpers, the smoke test `cli_version_runs`, and the golden tests.
- [ ] **Step 2:** Run `cargo test`. Expected: PASS (`golden_g` passes now; the others are ignored). `cargo test -- --ignored` shows the failing set.
- [ ] **Step 3:** Commit `tests: CLI helper and BI CfgConvert golden fixtures` (including `tests/fixtures/bi`).

### Task 1: Unquoted string values end at `;`, `}` or a newline [P1]

**Files:** Modify `src/config.rs:42-52, 81, 98` (`unquoted_string`, `unquoted_string_array`, `unquoted_string_entry` terminator). Test: `tests/bi_golden.rs`, `tests/cli.rs`.

- [ ] **Step 1:** Remove `#[ignore]` from `golden_us` and `golden_w1`. Add the CLI test `unquoted_before_brace`: `class A { x = foo };` rapifies with exit 0, and `derapify` gives `x = "foo";`.
- [ ] **Step 2:** Run them. Expected: FAIL (the string swallows the rest of the file).
- [ ] **Step 3:** Implement. The value is `(![';' | '}' | '\r' | '\n'] [_])*`, trimmed at the end. The entry terminator becomes `";" / &"}" / &newline`. Array elements also stop at a newline. `1.5x` must be the string `"1.5x"`, not a number followed by garbage.
- [ ] **Step 4:** Run `cargo test`. Expected: all PASS.
- [ ] **Step 5:** Commit `config: end unquoted string values at ; } or newline like CfgConvert`.

### Task 2: No panic for warnings on line 1; print each warning once [P1]

**Files:** Modify `src/config.rs:626-636`. Test: `tests/cli.rs`.

- [ ] **Step 1:** Write tests. `warning_on_line_one_no_panic`: `class A { x = foo; };` gives exit 0, and stderr contains `:1:` and `unquoted-string`. `warning_on_line_two_location`: the same after a blank line, stderr contains `:2:`. `warning_printed_once`: `stderr.matches("unquoted-string").count() == 1`.
- [ ] **Step 2:** Run them. Expected: FAIL (panic; the count is 2).
- [ ] **Step 3:** Implement. Fix the `line_origins` index so it cannot underflow (clamp to `0..len`), keeping the existing line numbers. Deduplicate `warnings` by `(offset, message)` before printing: PEG backtracking runs the rule actions twice.
- [ ] **Step 4:** Run `cargo test`. Expected: all PASS.
- [ ] **Step 5:** Commit `config: fix panic for warnings on line 1, dedupe warnings`.

### Task 3: Copy already-rapified files as-is (#15) [P1]

**Files:** Modify `src/pbo.rs:227-231` and `src/config.rs` (`cmd_rapify`). Test: `tests/cli.rs`, `tests/bi_golden.rs`.

**Interfaces:** Produces `pub fn is_rapified(data: &[u8]) -> bool` in `src/config.rs`, which checks for the prefix `b"\0raP"`.

- [ ] **Step 1:** Write tests. `build_keeps_prerapified_rvmat`: in a mod dir with `config.cpp` and `data/x.rvmat` (rapified with `armake2 rapify`), `build` exits 0, and `cat <pbo> data\x.rvmat` is byte-identical to the input. `build_keeps_prerapified_config_cpp`: the same with rapified bytes saved as `config.cpp`; the PBO's `config.bin` is byte-identical. Remove `#[ignore]` from `golden_rap` (the `rapify` command passes rapified input through unchanged, like CfgConvert).
- [ ] **Step 2:** Run them. Expected: FAIL.
- [ ] **Step 3:** Implement. Read the file into a buffer. If `is_rapified`, store it as-is and warn `prerapified-file`. Otherwise run `Config::read(&mut Cursor::new(buf), …)`. Do the same in `cmd_rapify`.
- [ ] **Step 4:** Run `cargo test`. Expected: all PASS.
- [ ] **Step 5:** Commit `build: pass already-rapified rvmat/config through (upstream #15)`.

### Task 4: Comments after directives (#51) [P1]

**Files:** Modify `src/preprocess.rs:111-115` (`line()` rule; drop the `@todo`). Test: `tests/bi_golden.rs`, `tests/preprocess.rs`.

- [ ] **Step 1:** Remove `#[ignore]` from `golden_c51`. Add `test_preprocess_directive_trailing_comments`, which covers `#include "inc.hpp" /* c */`, `#define Y 1   // c` and `#ifdef Y\t// c` and asserts the expected expansion.
- [ ] **Step 2:** Run them. Expected: FAIL `Unexpected token "/"`.
- [ ] **Step 3:** Implement. Allow `[' ' | '\t']*` before each trailing `comment_token()`.
- [ ] **Step 4:** Run `cargo test`. Expected: all PASS.
- [ ] **Step 5:** Commit `preprocess: allow comments after directives (upstream #51)`.

### Task 5: Macro argument-count mismatch (#28) [P2]

**Decision:** match CfgConvert's output (the invocation expands to an empty string), but also warn `macro-arg-count`. BI is silent here, and that silence is how ACE ended up shipping broken state machines. BI's other quirk, dropping commas inside nested parentheses (`QUOTE(f(1,2))` → `"f(12)"`), is deliberately **not** copied. armake2 keeps `"f(1,2)"`, which is what modders mean.

**Files:** Modify `src/preprocess.rs` (macro invocation/expansion, ~lines 240-300). Test: `tests/bi_golden.rs`, `tests/cli.rs`.

- [ ] **Step 1:** Remove `#[ignore]` from `golden_ma` and `golden_m28`. Add the CLI tests `macro_arity_warns` (stderr contains `macro-arg-count` and `QUOTE`) and `object_like_macro_no_arity_warning`.
- [ ] **Step 2:** Run them. Expected: FAIL (`QUOTE(a,b)` is left unexpanded, and the unquoted-string warning appears instead).
- [ ] **Step 3:** Implement. When a function-like macro is called with N≠M arguments, replace the invocation with nothing (inside `#x` stringification this gives `""`) and warn `Macro "NAME" expects M argument(s), got N`. `NONE()` with zero parameters accepts the empty argument list.
- [ ] **Step 4:** Run `cargo test`. Expected: all PASS.
- [ ] **Step 5:** Commit `preprocess: handle macro argument count mismatch like CfgConvert, with warning (upstream #28)`.

### Task 6: Non-UTF-8 entry names (#34) [P2]

**Files:** Modify `Cargo.toml` (`encoding_rs`), `src/io.rs:55-67`, `src/pbo.rs` (header read/write, `PBO` struct), `src/sign.rs:66-90`. Create `tests/pbo.rs`.

**Interfaces:**
- `ReadExt::read_cstring_bytes(&mut self) -> io::Result<Vec<u8>>`. `read_cstring` wraps it and returns `ErrorKind::InvalidData` instead of panicking.
- `PBO.raw_names: HashMap<String, Vec<u8>>`. It holds entries only for names that were not valid UTF-8, keyed by the windows-1252 decode (lossless for all 256 byte values).
- `PBO::name_bytes(&self, name: &str) -> Vec<u8>`. `namehash` (ASCII-lowercased bytes), the file-hash ordering and `PBO::write` all use it.

- [ ] **Step 1:** Write tests using `tests/fixtures/bi/sign/`.
  - `sign_matches_bi_utf8`: sign `testsig.pbo` with `armk.biprivatekey`; the bytes equal `testsig.pbo.armk.bisign`. This is a guard and passes today.
  - `sign_matches_bi_non_utf8`: the same for `nonutf8.pbo`.
  - `verify_bi_signature_non_utf8`: armake2 `verify` accepts BI's bisign.
  - `read_non_utf8_name`: the files contain the key `data\a—b.txt`.
  - `roundtrip_non_utf8_name`: `read` then `write` gives a byte-identical file.
  - `cli_unpack_non_utf8_name`: exit 0, stderr contains `non-utf8-filename`, and `data/a—b.txt` exists.
- [ ] **Step 2:** Run `cargo test --test pbo`. Expected: `sign_matches_bi_utf8` PASS, the others FAIL (panic `FromUtf8Error`).
- [ ] **Step 3:** Implement as in Interfaces. Header-extension strings use the same lossless fallback.
- [ ] **Step 4:** Run `cargo test` and `./testharness/run.sh`. Expected: all PASS.
- [ ] **Step 5:** Commit `pbo: handle non-UTF-8 entry names losslessly, signatures match DSSignFile (upstream #34)`.

### Task 7: Normalize `$PBOPREFIX$` (#42) [P2]

**Files:** Modify `src/pbo.rs:216-225`. Test: `tests/pbo.rs`.

- [ ] **Step 1:** Write tests with `PBO::from_directory(dir, false, &[], &[])`. `prefix_trailing_slash_stripped`: `my\mod\` gives `my\mod`. `prefix_normalized_crlf_and_leading_slash`: `\my\mod\\\r\n` gives `my\mod`. `prefix_key_value_form`: `prefix=my\mod\` followed by `version=a=b` gives the prefix `my\mod` and `version == "a=b"`.
- [ ] **Step 2:** Run them. Expected: FAIL.
- [ ] **Step 3:** Implement. Trim, strip leading and trailing `\`/`/`, and convert `/` to `\`. Warn `prefix-slash` if a separator was stripped. Use `splitn(2, '=')`.
- [ ] **Step 4:** Run `cargo test`. Expected: all PASS.
- [ ] **Step 5:** Commit `pbo: normalize slashes in $PBOPREFIX$ (upstream #42)`.

### Task 8: Include resolution on Linux (#17, #14) [P2]

**Files:** Modify `src/preprocess.rs:383-490` (`matches_include_path`, `search_directory`, `find_include_file`). Test: `tests/bi_golden.rs`, `tests/preprocess.rs`, `tests/cli.rs`.

- [ ] **Step 1:** Remove `#[ignore]` from `golden_case`. Add tests:
  - `include_case_insensitive_absolute`: `\z\mod\Foo.hpp` resolves via `-i` to `z/mod/foo.hpp`.
  - `include_exact_case_wins`: `Inc.hpp`(`a=1;`) and `inc.hpp`(`a=2;`) both exist; `#include "inc.hpp"` gives `a=2;`.
  - `cli_include_case_warns`: stderr contains `include-case`.
  - `cli_missing_include_folder`: `-i nonexist` gives an exit code other than 0 and 101, and stderr contains `nonexist`.
  - `include_not_found_has_line`: the error contains `config.cpp:2`.
- [ ] **Step 2:** Run them. Expected: FAIL.
- [ ] **Step 3:** Implement. Do an exact pass first, then a pass comparing each component ASCII-lowercased, with the `include-case` warning. Validate `-i` folders up front. Replace the `unwrap()`s in directory walking with `?`/skip. Put the directive's line in the not-found error.
- [ ] **Step 4:** Run `cargo test`. Expected: all PASS.
- [ ] **Step 5:** Commit `preprocess: case-insensitive include fallback, clear include errors (upstream #17, #14)`.

### Task 9: `__EVAL` / `__EXEC` (#23) [P3]

**Scope:** a small evaluator matching CfgConvert on number and string literals, `+ - * / % ^`, parentheses, unary minus, string `+`, and variables. `__EXEC(a = expr; b = expr)` assigns variables for later `__EVAL` calls in the same file. Anything else (SQF commands such as `floor`, comparisons) is a hard error: `__EVAL expression not supported by armake2: <expr>`. BI does evaluate those (`floor 2.7`→`2`, `1 > 0`→`"true"`), but implementing SQF is out of scope.

**Files:** Create `src/eval.rs` (evaluator). Modify `src/config.rs` (grammar: `__EVAL(...)` as value and array element, `__EXEC(...)` as entry) and `src/lib.rs` (module). Test: `tests/bi_golden.rs`, `tests/config.rs`.

**Interfaces:** `pub enum EvalValue { Number(f32), Str(String) }`. `pub fn eval(expr: &str, vars: &mut HashMap<String, EvalValue>) -> Result<EvalValue, Error>`. `pub fn exec(stmts: &str, vars: &mut HashMap<String, EvalValue>) -> Result<(), Error>`. A `Number` with an integral value is stored as a rapified int, otherwise as a float. This matches BI: `7`, `4.5`, `2`.

- [ ] **Step 1:** Remove `#[ignore]` from `golden_ev`, `golden_ex` and `golden_e23`. Add `eval_unsupported_errors` (`__EVAL(floor 2.7)` gives `Err` containing `not supported`).
- [ ] **Step 2:** Run them. Expected: FAIL.
- [ ] **Step 3:** Implement the precedence-climbing evaluator in `src/eval.rs` and hook it into the grammar. `^` is right-associative and binds tighter than `*`.
- [ ] **Step 4:** Run `cargo test`. Expected: all PASS.
- [ ] **Step 5:** Commit `config: evaluate simple __EVAL/__EXEC like CfgConvert (upstream #23)`.

### Task 10: Lone `\r` newlines (#39) [P3]

**Files:** Modify `src/preprocess.rs` (`pub fn preprocess`, before parsing ~493). Test: `tests/bi_golden.rs`.

- [ ] **Step 1:** Remove `#[ignore]` from `golden_mac`.
- [ ] **Step 2:** Run it. Expected: FAIL.
- [ ] **Step 3:** Implement. At entry, replace `"\r\n"` → `"\n"`, then `'\r'` → `'\n'`.
- [ ] **Step 4:** Run `cargo test`. Expected: all PASS, and `test_preprocess_lineorigins` is unchanged.
- [ ] **Step 5:** Commit `preprocess: accept classic Mac newlines (upstream #39)`.

---

### Task 11: Binarize with textures reachable (#21) [P2]

**Files:** Modify `src/binarize.rs` (proton sandbox: `create_sandbox`, `binarize`, lines ~290-390), `src/pbo.rs:232-233` (pass the addon root and prefix) and the Windows `binarize()` (~line 94, same current-directory rule). Test: `tests/binarize_proton.rs` (create).

**Interfaces:** `pub fn binarize(input: &PathBuf, addon_root: &Path, prefix: &str) -> Result<Cursor<Box<[u8]>>, Error>` (both the Windows and proton variants). `from_directory` passes the source directory and the normalized prefix from Task 7. For a `pack`/`binarize` CLI call with no prefix, it uses the folder name, as the PBO header does.

- [ ] **Step 1: Write the failing test** `proton_binarize_sets_alpha_flags`. It is `#[ignore]`d unless `ARMAKE2_PROTON_TESTS=1`, because it needs DayZ Tools and Proton. It runs `build --proton-binarize tests/fixtures/bi/binarize/t21 out.pbo`, then `cat out.pbo model.p3d`, and asserts the result is byte-equal to `tests/fixtures/bi/binarize/model.expected.p3d`. Add the unit test `sandbox_layout`, which runs without Proton: for the prefix `my\mod` and a source dir, the staging function creates `<sandbox>/P/my/mod` → source (symlink), and the computed input dir is `P:\my\mod\<subdir>`.
- [ ] **Step 2:** Run `ARMAKE2_PROTON_TESTS=1 cargo test --test binarize_proton -- --ignored`. Expected: FAIL (the flags at byte offset 52 are `ff000000`, but `ff802020` is expected).
- [ ] **Step 3: Implement.**
  - When the sandbox is created, also create `<sandbox>/P` and map `prefix/pfx/dosdevices/p:` → `<sandbox>/P`.
  - For each addon, symlink `<sandbox>/P/<prefix with \ → />` → the addon source dir, once per addon. Source models stay untouched because the output goes to `Z:\out\N`.
  - Run binarize.exe as `Z:\Binarize\binarize.exe` with the **current directory `<sandbox>/P`**, and the source as `P:\<prefix>\<relative dir of the model>`.
  - Textures from outside the addon (e.g. vanilla `dz\…`) stay unresolved. Scan binarize's log for texture-load failures and pass them on as warning `binarize-missing-texture`.
- [ ] **Step 4:** Run `ARMAKE2_PROTON_TESTS=1 cargo test -- --include-ignored` and `./testharness/run.sh`. Expected: all PASS.
- [ ] **Step 5:** Commit `binarize: run binarize.exe from a P: root with the addon mapped, so texture alpha is detected (upstream #21)`.

## Regenerating BI fixtures

The DayZ Tools are run in a Proton sandbox whose `Z:` drive is a small directory: `prefix/pfx/dosdevices/z:` → sandbox root. This is the same trick as `--proton-binarize`. Use `STEAM_COMPAT_DATA_PATH=<sandbox>/prefix SteamAppId=830640 proton run …` from the copied tool folder. Commands:
- `CfgConvert.exe -bin -dst Z:\out\x.bin Z:\in\x.cpp`
- `cmd /c "DSSignFile.exe key.biprivatekey x.pbo > Z:\out\log.txt 2>&1"`
- `cmd /c "DSCheckSignatures.exe -deep <pbodir> <keydir> > Z:\out\log.txt 2>&1"`

The DS tools print nothing on Proton's console unless the output is redirected through `cmd`. A PBO edited by hand needs its SHA-1 trailer recomputed (`sha1(body)`, after a `\0`), or DSSignFile rejects it with "Stored and calculated hash don't match".

## Deferred

- **#54 (RAM):** needs `PBO` to hold file sources instead of in-memory `Cursor`s, and signing to hash while streaming.
- **Byte-identical rapify:** CfgConvert writes a 4-byte offset after every class body. Not needed for the game; only worth doing for reproducible builds that match BI.
