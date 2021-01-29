import './todo_item.mjs'

this.state = {
  hasCheckedItems: false,
  items: [],
};

this.stateMappers = {
  hasCheckedItems: (items) => items.filter(x => x.done).length > 0,
}

function markItemDone(event) {
  this.state.items[event.detail.id].done = !this.state.items[event.detail.id].done
  this.setState({items: this.state.items})
}

function addItem() {
  this.state.items.push({
    text: this.refs.textbox.value,
    done: false,
  })
  this.refs.textbox.value = "";
  this.setState({items: this.state.items})
}

function clearChecked() {
  this.setState({
    items: this.state.items.filter((x) => !x.done),
  })
}
