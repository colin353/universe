const attributes = [ "code" ];

this.stateMappers = {
  lines: (code) => {
    if (!code) return {}

    const output = {};
    let lineNumber = 0;
    for(const line of atob(code).split("\n")) {
      output[lineNumber] = {};
      output[lineNumber].lineNumber = lineNumber;
      output[lineNumber].code = line;

      lineNumber += 1;
    }
    return output;
  }
};

this.state = {
  lines: this.stateMappers.lines(this.state.code),
  x: 0,
};

setInterval(() => {
  this.setState({
    x: (this.state.x + 1) % 4 
  })
}, 500)
