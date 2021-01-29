const attributes = [ "id", "done", "text" ];

function markDone() {
  this.dispatchEvent(new CustomEvent('toggle', {
    detail: { id: this.state.id },
  }))
}
