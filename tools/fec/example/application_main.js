const attributes = [ 'test', 'west' ];

this.state = {
  x: false,
  message: "Hello, world",
};

this.stateMappers = {
  "message": (x) => x ? "Hello, world" : "Goodbye, earth"
};

setInterval(() => {
  this.setState({
    x: !this.state.x
  });
}, 3000)
