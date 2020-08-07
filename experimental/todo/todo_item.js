const attributes = [ 
  'key', 'message', 'done'
];

this.state = {
  style: "done"
}

this.stateMappers = {
  completed: (done) => done == "true",
  style: (completed) => completed ? "done" : "not-done",
  _syncChecked: (completed) => {
    if (this.refs.checkbox) {
      this.refs.checkbox.checked = completed;
    }
  }
}


const onClick = (e) => {
  this.dispatchEvent(new CustomEvent('toggle', {
    detail: {
      key: this.state.key,
      done: !this.state.completed
    }
  }))
}
