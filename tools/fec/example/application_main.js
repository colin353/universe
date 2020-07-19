alert("hello world");
console.log("booted up baby");

console.log("document: ", document.getRootNode());

let x = "updated para" + Math.random();

setInterval(() => {
  x += "newly updated para" + Math.random();
  this.$$invalidate(0);
}, 3000)
