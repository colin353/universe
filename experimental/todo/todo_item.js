const attributes = [ 
  'message', 'done'
];

this.state = {
  style: "done"
}

this.stateMappers = {
  style: (done) => done ? "done" : "not-done",
  _setRef: (done) => {
    if (this.refs.checkbox) {
      this.refs.checkbox.checked = done;
    }
  }
}

const onClick = () => {
  this.setState({
    done: !this.state.done
  })
}
