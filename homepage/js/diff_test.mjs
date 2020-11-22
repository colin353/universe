import calculateDiff from "./diff.mjs";
import {assert_eq, test, } from "../../tools/fec/test.mjs";

test("test_diff", () => {
    assert_eq(calculateDiff(), []);
});

test("test_obj_comparison", () => {
    assert_eq({a: []}, {a: []});
});
