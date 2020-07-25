this.state = {
  title: "click here!",
};

let x = 0;

function callback() {
  x += 1;
  this.setState({
    title: `you clicked it ${x} times`
  });
}
