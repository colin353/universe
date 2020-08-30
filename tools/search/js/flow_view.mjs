import Store from '../../../util/js/store.mjs';

import("./flow_card.mjs");

this.state = {
  stack: extractURLStack() || JSON.parse(localStorage.getItem("flow-stack") || "[]"),
  currentLineNumber: 43,
  currentFilename: window.location.pathname.substr(1),
  currentLine: '',
}

function extractURLStack() {
  if(!window.location.hash.startsWith("#F")) {
    return false;
  }

  return JSON.parse(atob(window.location.hash.substr(2)));
}

this.stateMappers = {
  currentJoinedFilename: (currentFilename, currentLineNumber) => {
    let output = currentFilename;
    if(currentLineNumber) {
      output += "#L" + currentLineNumber
    }
    return output;
  },
  _updateLocalStorage: (stack) => {
    const json = JSON.stringify(stack);
    localStorage.setItem("flow-stack", json);
    window.location.hash = '#F' + btoa(json);
  }
}

const store = new Store();
store.addWatcher('currentLineNumber', (currentLineNumber) => {
  this.setState({
    currentLineNumber: currentLineNumber || 0,
  });
})

store.addWatcher('currentLine', (currentLine) => {
  this.setState({
    currentLine: currentLine || ''
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

function removeCard(event) {
   this.state.stack.splice(event.detail.key, 1);
   this.setState({stack: this.state.stack});
}
