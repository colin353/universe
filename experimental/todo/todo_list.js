let todoIndex = 2;

this.state = {
  todo: {
    0: "buy groceries",
    1: "do homework",
  }
}

const addItem = () => {
  this.state.todo[todoIndex++] = this.refs.todoInput.value;
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
