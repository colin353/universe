//const attributes = [ "code" ];

this.state = {
  code: {
    1: "abcdef",
    2: "ghijkl",
    3: "mnopqr",
  },
  codeStyle: 'normal',
};

let x = 4;

setInterval(() => {
  this.state.code[x] = `hello ${x}`;
  x++;

  if (x % 2 == 0) {
    delete this.state.code[x-2];
  }

  if (x % 4 == 0) {
    this.state.code = {}
  }

  this.setState({
    code: this.state.code,
  })
}, 800)
