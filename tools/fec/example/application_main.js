alert("hello world");
console.log("booted up baby");

console.log("document: ", document.getRootNode());

this.state = {
  x: "updated para" + Math.random(),
};

setInterval(() => {
  this.setState({
    x: this.state.x += "newly updated para" + Math.random()
  });
}, 3000)
