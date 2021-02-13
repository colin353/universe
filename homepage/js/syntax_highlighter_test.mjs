import { test, assert_eq } from '../../tools/fec/test.mjs'

import getLanguageModel from './syntax_highlighter.mjs'

test("extract weird js syntax", () => {
  const model = getLanguageModel("javascript")

  const output = model.extractSyntax("export function* myTestFunction() {}")

  assert_eq(output, "<span class='keyword'>export</span> <span class='keyword'>function</span>* myTestFunction() {}")
})
