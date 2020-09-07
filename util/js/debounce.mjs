export default function debounce(fn, amount) {
  let wake = null;
  let lastArgs = null;
  return (...args) => {
    lastArgs = args;
    if (wake) {
      window.clearTimeout(wake);
    } 

    wake = window.setTimeout(() => fn(...lastArgs), amount)
  }
}
