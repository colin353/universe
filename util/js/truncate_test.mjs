import truncate from './truncate.mjs'
import { test, assert_eq } from '../../tools/fec/test.mjs'

test("always truncates correct length", () => {
  for(var i=3;i<255;i++) {
    let t = truncate("always truncates correct length", i);
    assert_eq(t.length, Math.min(i, 31));
  }
})

test("truncates correctly", () => {
  let t = truncate("asdf hello world", 10);
  assert_eq(t, "asd...orld");
})

test("truncates filenames correctly", () => {
  let t = truncate("third_party/vendor/aho-corasick-0.7.10/src/nfa.rs#L80", 25);
  assert_eq(t, "third_party.../nfa.rs#L80");
})
