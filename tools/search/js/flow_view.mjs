import Store from '../../../util/js/store.mjs';

import("./flow_card.mjs");

this.state = {
  stack: JSON.parse(localStorage.getItem("flow-stack") || "[]"),
  currentLineNumber: 43,
  currentFilename: window.location.pathname.substr(1),
  currentLine: 'import x from y',
}

this.stateMappers = {
  _updateLocalStorage: (stack) => {
    localStorage.setItem("flow-stack", JSON.stringify(stack));
  }
}

const store = new Store();
store.addWatcher('currentLineNumber', (currentLineNumber) => {
  this.setState({
    currentLineNumber,
  });
})

store.addWatcher('currentLine', (currentLine) => {
  this.setState({
    currentLine
  });
})

function addCard(event) {
    this.state.stack.push({
      code: event.detail.code,
      filename: event.detail.filename,
      comment: event.detail.comment,
    })

   this.setState({stack: this.state.stack});
}
