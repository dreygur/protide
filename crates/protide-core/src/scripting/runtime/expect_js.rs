//! JS `expect()` assertion API binding

use rquickjs::{Ctx, Value};

use crate::scripting::results::ScriptError;

/// Set up `expect()` function for test assertions.
pub(super) fn setup_expect_js(ctx: &Ctx<'_>) -> Result<(), ScriptError> {
    let expect_js = r#"
// Recursive deep-equality for typical JSON-shaped values (objects, arrays,
// primitives, null/undefined/NaN). Uses Object.is semantics so NaN equals
// NaN but not null/undefined, and compares object keys order-independently.
// A property explicitly set to `undefined` counts as present (and differs
// from an absent property) so `{a:1,b:undefined}` !== `{a:1}`.
function __deepEqual(a, b) {
    if (Object.is(a, b)) return true;
    if (typeof a !== typeof b) return false;
    if (a === null || b === null || a === undefined || b === undefined) return false;
    if (typeof a !== 'object') return false;
    if (Array.isArray(a) !== Array.isArray(b)) return false;
    if (Array.isArray(a)) {
        if (a.length !== b.length) return false;
        for (let i = 0; i < a.length; i++) {
            if (!__deepEqual(a[i], b[i])) return false;
        }
        return true;
    }
    const aKeys = Object.keys(a);
    const bKeys = Object.keys(b);
    if (aKeys.length !== bKeys.length) return false;
    for (const key of aKeys) {
        if (!Object.hasOwn(b, key)) return false;
        if (!__deepEqual(a[key], b[key])) return false;
    }
    return true;
}

// Display helper for failure messages: JSON.stringify gives structured
// content instead of the useless "[object Object]" from String(obj).
function __display(x) {
    if (x === undefined) return 'undefined';
    try {
        const s = JSON.stringify(x);
        return s === undefined ? String(x) : s;
    } catch (e) {
        return String(x);
    }
}

function expect(actual) {
    return {
        _actual: actual,
        _negated: false,
        get not() {
            const copy = Object.create(this);
            copy._negated = !this._negated;
            return copy;
        },
        _check(passed, name, expected) {
            const finalPassed = this._negated ? !passed : passed;
            const prefix = this._negated ? "not " : "";
            globalThis.__storage.testResults.push({
                passed: finalPassed,
                name: prefix + name,
                expected: __display(expected),
                actual: __display(this._actual)
            });
            return finalPassed;
        },
        toBe(expected) {
            return this._check(this._actual === expected, "toBe", expected);
        },
        toEqual(expected) {
            const eq = __deepEqual(this._actual, expected);
            return this._check(eq, "toEqual", expected);
        },
        toBeTruthy() {
            return this._check(!!this._actual, "toBeTruthy", "truthy");
        },
        toBeFalsy() {
            return this._check(!this._actual, "toBeFalsy", "falsy");
        },
        toBeNull() {
            return this._check(this._actual === null, "toBeNull", "null");
        },
        toBeUndefined() {
            return this._check(this._actual === undefined, "toBeUndefined", "undefined");
        },
        toBeDefined() {
            return this._check(this._actual !== undefined, "toBeDefined", "defined");
        },
        toBeGreaterThan(n) {
            return this._check(this._actual > n, "toBeGreaterThan", n);
        },
        toBeGreaterThanOrEqual(n) {
            return this._check(this._actual >= n, "toBeGreaterThanOrEqual", n);
        },
        toBeLessThan(n) {
            return this._check(this._actual < n, "toBeLessThan", n);
        },
        toBeLessThanOrEqual(n) {
            return this._check(this._actual <= n, "toBeLessThanOrEqual", n);
        },
        toContain(item) {
            let contains = false;
            if (typeof this._actual === 'string') {
                contains = this._actual.includes(item);
            } else if (Array.isArray(this._actual)) {
                contains = this._actual.includes(item);
            }
            return this._check(contains, "toContain", item);
        },
        toHaveLength(n) {
            const len = this._actual?.length;
            return this._check(len === n, "toHaveLength", n);
        },
        toHaveProperty(path, value) {
            const parts = path.split('.');
            let obj = this._actual;
            for (const part of parts) {
                if (obj === null || obj === undefined || !Object.hasOwn(obj, part)) {
                    return this._check(false, "toHaveProperty", path);
                }
                obj = obj[part];
            }
            if (arguments.length === 2) {
                return this._check(obj === value, "toHaveProperty", path + " = " + value);
            }
            return this._check(true, "toHaveProperty", path);
        },
        toMatch(pattern) {
            const re = pattern instanceof RegExp ? pattern : new RegExp(pattern);
            return this._check(re.test(this._actual), "toMatch", pattern);
        }
    };
}
globalThis.expect = expect;
"#;

    ctx.eval::<Value, _>(expect_js)
        .map_err(|e| ScriptError::new(format!("Failed to setup expect: {}", e)))?;

    Ok(())
}
