import getLanguageModel from './syntax_highlighter.mjs';
import { base64Decode } from './utils.mjs';
import Store from '../../util/js/store.mjs';

const attributes = [ "code", "language", "line", "startline" ];
const store = new Store();

// Disables chrome's automatic scroll restoration logic, necessary to
// avoid conflicts w/ the automatic "jump-to-line" behaviour.
window.history.scrollRestoration = 'manual';

this.shadow.addEventListener("click", () => {
  this.setState({
    showMenu: false
  })
})

const contextMenu = (event) => {
  const selection = window.getSelection().toString();
  if(selection == "") return;

  this.setState({
    showMenu: true,
    menuX: event.clientX,
    menuY: event.clientY,
    selection,
  })

  event.preventDefault();
}

const copy = (event) => {
  navigator.clipboard.writeText(this.state.selection)
}

const search = () => {
  this.dispatchEvent(new CustomEvent('search', {
    detail: { token: this.state.selection }
  }));
}

const definition = () => {
  this.dispatchEvent(new CustomEvent('define', {
    detail: { token: this.state.selection }
  }));
}

this.stateMappers = {
  parsedLines: (code, language) => {
    if (!code) return [];
    let parsedLines = [];
    let model = getLanguageModel(language);
    for(const line of base64Decode(code).split("\n")) {
      parsedLines.push(model.extractSyntax(line));
    }
    return parsedLines;
  },
  lines: (parsedLines, language, line) => {
    if(!parsedLines) return {};

    const output = {};
    let lineNumber = 1;
    const selectedLine = parseInt(line)
    for(const line of parsedLines) {
      output[lineNumber] = {};
      output[lineNumber].lineNumber = this.state.startingLine + lineNumber;
      output[lineNumber].class = lineNumber == selectedLine ? 'selected-line' : lineNumber == selectedLine - 5 ? 'top-line' : '';
      output[lineNumber].code = line;

      lineNumber += 1;
    }
    return output;
  },
  _ensureLineVisible: (line) => {
    focusSelectedLine();
    this.dispatchEvent(new CustomEvent('lineSelected', {
      detail: { line }
    }));
  },
  _updateLineInStore: (line, parsedLines) => {
    store.setState("currentLineNumber", line);
    store.setState("currentLine", parsedLines[line-1]);
  },
  startingLine: (startline) => {
    return parseInt(startline) || 0
  }
};

const focusSelectedLine = () => {
    const elements = this.shadowRoot.querySelectorAll(".top-line")
    if (elements.length) {
      elements[0].scrollIntoView({block: "start"});
    }
}

this.componentDidMount = () => {
  setTimeout(focusSelectedLine, 10);
}

this.state = {
  parsedLines: this.stateMappers.parsedLines(this.state.code),
  lines: this.stateMappers.lines(this.state.code),
  startingLine: 0,
};

function selectLine(event) {
  this.setState({
    line: parseInt(event.srcElement.innerText)
  })
}