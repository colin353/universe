import { humanizeInterval } from './human_date.mjs'
import { test, assert_eq } from '../../tools/fec/test.mjs'

test("renders date correctly", () => {
  assert_eq(humanizeInterval(4000), "just now");
  assert_eq(humanizeInterval(40000), "a minute ago");
  assert_eq(humanizeInterval(-40000), "a minute from now");
  assert_eq(humanizeInterval(-17 * 60 * 1000), "15 minutes from now");

  assert_eq(humanizeInterval(60 * 60 * 1000), "an hour ago");
  assert_eq(humanizeInterval(24 * 60 * 60 * 1000), "yesterday");
  assert_eq(humanizeInterval(-24 * 60 * 60 * 1000), "tomorrow");

  assert_eq(humanizeInterval(17 * 60 * 60 * 1000), "17 hours ago");
})

