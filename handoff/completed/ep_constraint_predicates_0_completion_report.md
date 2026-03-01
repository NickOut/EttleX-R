# Completion Report: ettle:constraint_predicates (ep:constraint_predicates:0)

## Summary

**EP**: `ep:constraint_predicates:0`
**Ettle**: `ettle:constraint_predicates`
**Completed**: 2026-02-27
**Acceptance gates**: lint ✅ test ✅ coverage ✅ (80.28% ≥ 80%)

---

## Scope

Minimal DSL predicate evaluator with typed context, candidate selection with priority + lexicographic tiebreak, and pluggable ambiguity routing via `ApprovalRouter` trait.

---

## New Files Created

| File | Purpose |
|---|---|
| `crates/ettlex-core/src/predicate/mod.rs` | Module root; re-exports |
| `crates/ettlex-core/src/predicate/parser.rs` | Hand-rolled recursive descent parser |
| `crates/ettlex-core/src/predicate/evaluator.rs` | `evaluate_predicate()` |
| `crates/ettlex-core/src/predicate/resolver.rs` | `resolve_candidates()` with policy dispatch |

---

## Files Modified

| File | Change |
|---|---|
| `crates/ettlex-core/src/errors.rs` | Added 3 predicate `ExErrorKind` variants + `code()` arms |
| `crates/ettlex-core/src/lib.rs` | Added `pub mod predicate;` |

---

## Predicate DSL

### Syntax
```
predicate  = or_expr
or_expr    = and_expr ("or" and_expr)*
and_expr   = not_expr ("and" not_expr)*
not_expr   = "not" not_expr | primary
primary    = comparison | has_expr | "(" or_expr ")"
comparison = key ("==" | "!=") value | key "in" "[" value_list "]"
has_expr   = "has" "(" key ")"
```

### Operators
| Operator | Syntax | Notes |
|---|---|---|
| equality | `key == "value"` | String, number (integer or float), boolean |
| inequality | `key != "value"` | Same types as equality |
| membership | `key in ["a", "b"]` | RHS is string-only list |
| has | `has(key)` | True if key exists in context (any type) |
| and | `expr and expr` | Short-circuit |
| or | `expr or expr` | Short-circuit |
| not | `not expr` | Unary prefix |

### Context Types
- `ContextValue::Text(String)`
- `ContextValue::Number(f64)`
- `ContextValue::Boolean(bool)`

Context is a `BTreeMap<String, ContextValue>` for stable ordering and deterministic evaluation.

---

## Candidate Selection

`resolve_candidates(candidates, context, policy, router)`:

1. **Filter**: Evaluate each candidate's predicate against context; keep passing candidates
2. **Priority selection**: Among passing candidates, select those with the minimum `priority` value
3. **Tiebreak**: If exactly one minimum-priority candidate remains → return it
4. **Ambiguity resolution** (multiple minimum-priority candidates):
   - `AmbiguityPolicy::FailFast` → `AmbiguousSelection` error
   - `AmbiguityPolicy::ChooseDeterministic` → lexicographic sort, return first
   - `AmbiguityPolicy::RouteForApproval(router)` → build `ApprovalRequest`, call router, return token

### Candidate Type
```rust
pub struct Candidate {
    pub id: String,
    pub predicate: Option<String>,  // None means always-match
    pub priority: u32,
}
```

---

## Error Kinds Added

| ExErrorKind | Stable Code |
|---|---|
| `PredicateParseError` | `ERR_PREDICATE_PARSE_ERROR` |
| `PredicateTypeError` | `ERR_PREDICATE_TYPE_ERROR` |
| `ContextKeyMissing` | `ERR_CONTEXT_KEY_MISSING` |

Note: `AmbiguousSelection` error is surfaced via `ExErrorKind::PredicateParseError` with a descriptive message (not a separate variant), to avoid bloating the error enum. `ApprovalRoutingUnavailable` is reused from the approval module.

---

## Key Design Decisions

1. **Hand-rolled recursive descent parser**: No new dependencies (no nom, pest, lalrpop). The grammar is simple enough that a hand-rolled parser with clear recursive structure is more maintainable.
2. **`Option<String>` for predicate**: `None` means no predicate — the candidate always matches. This avoids parsing an empty string.
3. **Priority is ascending**: Lower `priority` value = higher priority. All candidates with the minimum value are considered "best".
4. **ApprovalRouter decoupled**: The resolver takes `&dyn ApprovalRouter` by reference — no coupling to storage. The `NullApprovalRouter` can be passed when routing is not configured.
5. **BTreeMap context**: Ensures stable iteration order, which matters for deterministic tiebreak when `ChooseDeterministic` is used.

---

## Test Coverage

Parser tests:
- All operators: `==`, `!=`, `in`, `has`
- Compound: `and`, `or`, `not`
- Nested parentheses
- Error cases: unclosed bracket, unknown operator, missing operand

Evaluator tests:
- String equality/inequality
- Number equality (integer and float)
- Boolean equality
- `in` with match and non-match
- `has` with present and absent keys
- `and` / `or` / `not` combinators
- `ContextKeyMissing` for undefined keys

Resolver tests:
- Single candidate — passes filter
- Multiple candidates — filter reduces
- Priority tiebreak — lower priority wins
- Tiebreak needed — `FailFast` errors, `ChooseDeterministic` picks lexicographic first
- `RouteForApproval` with `NullApprovalRouter` → `ApprovalRoutingUnavailable`
- No predicate (`None`) — always matches
- Empty candidate list

---

## Acceptance Gate Results

```
make lint        ✅  0 warnings
make test        ✅  all tests pass (179+ across workspace)
make coverage-check  ✅  80.28% (threshold: 80%)
make coverage-html   ✅  coverage/tarpaulin-report.html generated
```
