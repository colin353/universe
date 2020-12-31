import getLanguageModel from './syntax_highlighter.mjs';
import { base64Decode } from './utils.mjs';
import Store from '../../util/js/store.mjs';
import truncate from '../../util/js/truncate.mjs';

const attributes = [ "code", "language", "line", "startline", "symbols", "filename" ];
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

class SymbolSpans {
  constructor(symbols) {
    this.lineNumber = 0;
    this.position = 0;
    this.sortedSymbols = symbols;
    let position = 0;
    for (const symbol of this.sortedSymbols) {
      symbol.position = position;
      position += 1;
    }
  }

  reset() {
    this.position = 0;
    this.lineNumber = 0;
  }

  next() {
    const symbol = this.getCurrentSymbol();
    this.lineNumber += 1;
    return symbol;
  }

  getSymbolForLine(line) {
    this.lineNumber = line - 1;
    for (let i=0; i<this.sortedSymbols.length; i++) {
      this.position = i;
      const symbol = this.getCurrentSymbol();
      if (symbol) return symbol;
    }
    return null;
  }

  getCurrentSymbol() {
    if(this.sortedSymbols.length <= this.position) {
      return null;
    }

    // The next symbol is still below us, return null
    if(this.sortedSymbols[this.position].start > this.lineNumber) {
      return null;
    }

    // We are at the last part of a symboldefinition, so increment position
    if(this.lineNumber == this.sortedSymbols[this.position].end) {
      this.position += 1;
      return this.sortedSymbols[this.position - 1];
    }
    // We are inside a symbol definition
    else if(this.lineNumber < this.sortedSymbols[this.position].end) {
      return this.sortedSymbols[this.position];
    }

    return null
  }
}

class NestedSymbolSpans {
  constructor(symbols) {
    if(!symbols) symbols = [];
    else symbols = eval(symbols);

    symbols.sort((a, b) => a.start - b.start);
    this.functions = new SymbolSpans(symbols.filter(x => x.type == 'FUNCTION'))
    this.structures = new SymbolSpans(symbols.filter(x => x.type == 'STRUCTURE'))
  }

  reset() {
    this.functions.reset();
    this.structures.reset();
  }

  join(fn, st) {
    if (fn && !st || !fn && st) return fn || st;
    else if (fn && st) {
      return { 
        ...fn,
        symbol: st.symbol + "::" + fn.symbol,
      }
    }

    return fn
  }

  next() {
    return this.join(this.functions.next(), this.structures.next());
  }

  getSymbolForLine(line) {
    return this.join(this.functions.getSymbolForLine(line), this.structures.getSymbolForLine(line))
  }
}

const updateInfoBox = async () => {
  const result = await fetch("/info?q=" + encodeURIComponent(this.state.selectedSymbolName));
  const usages = await result.json();

  this.setState({usages: usages.filter(x => !x.startsWith(this.state.filename)).slice(0, 8)});
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
  symbolSpans: (symbols) => {
    return new NestedSymbolSpans(symbols)
  },
  lines: (parsedLines, language, line, symbolSpans) => {
    if(!symbolSpans || !parsedLines) return {};

    const output = {};
    let lineNumber = 1;
    const selectedLine = parseInt(line)
    const topLine = Math.max(1, selectedLine - 5);

    symbolSpans.reset();
    for(const line of parsedLines) {
      output[lineNumber] = {};
      output[lineNumber].lineNumber = this.state.startingLine + lineNumber;

      output[lineNumber].class = lineNumber == selectedLine ? 'selected-line' : ''
      output[lineNumber].class += lineNumber == topLine ? ' top-line' : '';
      output[lineNumber].code = line;

      const symbol = symbolSpans.next();
      if (lineNumber !== selectedLine) {
        output[lineNumber].hasSymbol = symbol ? `has-symbol symbol-color-${symbol.position%2}` : '';
      }
 
      lineNumber += 1;
    }

    return output;
  },
  selectedSymbol: (symbolSpans, line) => {
    if(!line) return {};

    const x = symbolSpans.getSymbolForLine(parseInt(line))
    if(x) {
      return x;
    } 

    return {}
  },
  selectedSymbolName: (selectedSymbol) => {
    return selectedSymbol?.symbol
  },
  usages: (selectedSymbolName) => {
    if(this && selectedSymbolName) updateInfoBox();
    return [];
  },
  showUsages: (usages) => {
    return usages && usages.length > 0 
  },
  selectedSymbolType: (selectedSymbol) => {
    if (!selectedSymbol?.type) return '?';

    if (selectedSymbol?.type == 'STRUCTURE') return 'cls';
    else if(selectedSymbol?.type == 'FUNCTION') return 'fn';
    else if(selectedSymbol?.type == 'TRAIT') return 'tr';

    return '?';
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
  selectedSymbol: {},
  parsedLines: this.stateMappers.parsedLines(this.state.code),
  lines: this.stateMappers.lines(this.state.code),
  startingLine: 0,
  usages: [],
};

function selectLine(event) {
  this.setState({
    line: parseInt(event.srcElement.innerText)
  })
}

let jumpToLine = '';
let command = '';

// Keyboard shortcuts for jumping to lines/to top
window.addEventListener('keydown', (e) => {
  // Ignore keypresses within inputs
  if (e.srcElement != window.document.body) {
    return;
  }

  if (e.key == 'g') {
    if (command.length > 0) {
      // Jump to top
      this.setState({
        line: 1
      })
    } else {
      command = 'g';
    }
  } else {
    command = '';
  }

  if (e.key == 'd') {
    this.parentElement.scrollBy(0, 350);
  } else if (e.key == 'u') {
    this.parentElement.scrollBy(0, -350);
  } else if (e.key == 'j') {
    this.parentElement.scrollBy(0, 100);
  } else if (e.key == 'k') {
    this.parentElement.scrollBy(0, -100);
  }

  if (e.key.length == 1 && e.key >= '0' && e.key <= '9') {
    jumpToLine += e.key
  } else if (e.keyCode == 27) {
    // Detect escape + clear the jump-to-line
    jumpToLine = '';
  } else if (e.key == 'G') {
    if (jumpToLine.length > 0) {
      const lineToJumpTo = Math.min(parseInt(jumpToLine), Object.keys(this.state.lines).length)
      this.setState({
        line: lineToJumpTo
      })
    } else {
      this.setState({
        line: Object.keys(this.state.lines).length,
      })
    }
    jumpToLine = '';
  }
})
