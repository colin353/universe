const attributes = [ 'title' ];

this.stateMappers = {
  "message": (title) => `title is: ${title}`
}

this.state = {
  x: 0
};

function callback() {
  this.setState({
    x: this.state.x + 1
  });
}
