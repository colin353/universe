export default function truncate(input, length) {
  if(input.length <= length) return input;

  const startOffset = (length - 3) / 2;
  const endOffset = startOffset + (length - 3) % 2;

  let output = input.slice(0, startOffset) + "...";
  if (endOffset) output += input.slice(-endOffset);
  return output
}
