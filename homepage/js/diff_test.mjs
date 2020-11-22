import calculateDiff, { Difference, } from "./diff.mjs";
import {assert_eq, test, } from "../../tools/fec/test.mjs";

const left = `hello
the
world`;

const right = `hello
new
world`;

test("test_diff", () => {
    const expected = [
        Difference.same("hello"),
        Difference.removed("the"),
        Difference.added("new"),
        Difference.same("world"),
    ];

    assert_eq(calculateDiff(left, right), expected);
});
