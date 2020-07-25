this.stateMappers = {
  "message": (x) => `you clicked it ${x} times`
}

this.setState({
  x: 0
});

function callback() {
  this.setState({
    x: this.state.x + 1
  });
}
