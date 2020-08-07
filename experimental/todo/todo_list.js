let todoIndex = 2;

this.state = {
  todo: {
    0: { message: "buy groceries", done: false },
    1: { message: "do homework", done: true },
  }
}

const addItem = () => {
  this.state.todo[todoIndex++] = {
    message: this.refs.todoInput.value,
    done: false,
  };

  this.setState({
    todo: this.state.todo
  })
  this.refs.todoInput.value = '';
}

const keyDown = (e) => {
  if(e.key === 'Enter') {
    addItem()
  }
}

const onToggle = (e) => {
  this.state.todo[e.detail.key].done = e.detail.done;
  this.setState({todo: this.state.todo});

  if (e.detail.done) {
    setTimeout(() => {
      if (this.state.todo[e.detail.key].done) {
        delete this.state.todo[e.detail.key];
        this.setState({todo: this.state.todo});
      }
    }, 2000)
  }
}
