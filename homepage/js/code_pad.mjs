import getLanguageModel from './syntax_highlighter.mjs';
import { base64Decode } from './utils.mjs';
import Store from '../../util/js/store.mjs';
import debounce from '../../util/js/debounce.mjs';

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

function updateCodeSelection(event) {
  const selection = window.getSelection();
   
  if(!selection || selection.toString().trim() === "") {
    return
  }

  const matchingSpan = this.state.symbolSpans.getSymbolForTerm(selection.toString())

  if (matchingSpan) {
    this.setState({
      selectedSymbol: matchingSpan
    })
  } else {
    this.setState({
      selectedSymbol: {
        symbol: selection.toString()
      }
    })
  }
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

  getSymbolForTerm(term) {
    for (const symbol of this.sortedSymbols) {
      if (symbol.symbol === term) {
        return symbol;
      }
    }

    return null;
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

    // Check for overlaps. Don't want to render two overlapping sidebars since
    // that looks weird.
    let position = 0;
    for (var i=0; i<symbols.length; i++) {
      if(i < symbols.length-1) {
        if(symbols[i+1].start < symbols[i].end) {
          symbols[i].overlapping = true;
        } else {
          symbols[i].overlapping = false;
        }
      }

      if(!symbols[i].overlapping) {
        symbols[i].position = position;
        position += 1;
      }

    }

    this.functions = new SymbolSpans(symbols.filter(x => x.type == 'FUNCTION'))
    this.structures = new SymbolSpans(symbols.filter(x => x.type == 'STRUCTURE' || x.type == 'TRAIT'))
  }

  reset() {
    this.functions.reset();
    this.structures.reset();
  }

  getSymbolForTerm(term) {
    const match = this.functions.getSymbolForTerm(term) || this.structures.getSymbolForTerm(term)
    if (!match) return null;

    return this.getSymbolForLine(match.start + 1);
  }

  join(fn, st, skipOverlapping=true) {
    if (fn && !st || !fn && st && (!skipOverlapping || !st.overlapping)) return fn || st;
    else if (fn && st) {
      return { 
        ...fn,
        structure: st,
        symbol: fn.symbol,
      }
    }

    return fn
  }

  next() {
    return this.join(this.functions.next(), this.structures.next());
  }

  getSymbolForLine(line) {
    return this.join(this.functions.getSymbolForLine(line), this.structures.getSymbolForLine(line), false)
  }
}

this.stateMappers = {
  parsedLines: (code, language) => {
    if (!code) return [];
    let parsedLines = [];
    let model = getLanguageModel(language);
    for(const line of base64Decode(code).split("\n")) {
      parsedLines.push(model.extractSyntax(line));
    }

    // The reason for the parsedLines --> renderedLines distinction is because
    // parsedLines is the raw HTML after syntax extraction, whereas the
    // renderedLines also contains extra markup from highlighting selected text.
    this.setState({renderedLines: parsedLines})

    return parsedLines;
  },
  symbolSpans: (symbols) => {
    return new NestedSymbolSpans(symbols)
  },
  lines: (renderedLines, language, line, symbolSpans) => {
    if(!symbolSpans || !renderedLines) return {};

    const output = {};
    let lineNumber = 1;
    const selectedLine = parseInt(line)
    const topLine = Math.max(1, selectedLine - 5);

    symbolSpans.reset();
    for(const line of renderedLines) {
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
  matchingLines: (selectedSymbol) => {
    if (Object.keys(selectedSymbol).length == 0) {
      return '{}'
    }
    const sanitizedSymbol = selectedSymbol.symbol.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
    const regex = new RegExp(`(^|[^\\w])(${sanitizedSymbol})([^\\w]|$)`)
    let matches = [];
    let lineNumber = 0;
    let renderedLines = [];
    for (const line of this.state.parsedLines) {
      let match = regex.exec(line);
      if (match !== null) {
        renderedLines.push(line.replace(regex, (match, p1, p2, p3, offset, string) => {
          return `${p1}<span class="highlight">${p2}</span>${p3}`
        }))
        matches.push(lineNumber);
      } else {
        renderedLines.push(line)
      }
      lineNumber += 1;
    }

    this.setState({
      renderedLines
    })

    return {
      totalLines: renderedLines.length,
      matches
    };
  },
  showInfoBox: (selectedSymbol) => {
    return Object.keys(selectedSymbol).length > 0 && selectedSymbol != '{}'
  },
  selectedSymbol: (symbolSpans, line) => {
    if(!line) return {};

    return symbolSpans.getSymbolForLine(parseInt(line)) || {}
  },
  _ensureLineVisible: (line) => {
    setTimeout(() => {
      focusSelectedLine();
      this.dispatchEvent(new CustomEvent('lineSelected', {
        detail: { line }
      }));
    }, 0);
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

  const updateScrollPositionDebounced = debounce((scrollInfo) => {
    store.setState("codePadScrollInfo", scrollInfo)
  }, 16)

  this.parentElement.addEventListener("scroll", (e) => {
    updateScrollPositionDebounced({
      top: e.srcElement.scrollTop / e.srcElement.scrollHeight,
      height: e.srcElement.clientHeight / e.srcElement.scrollHeight
    })
  })

  updateScrollPositionDebounced({
    top: this.parentElement.scrollTop / this.parentElement.scrollHeight,
    height: this.parentElement.clientHeight / this.parentElement.scrollHeight
  })

  store.addWatcher("codePadScrollOffsetWriteOnly", (offset) => {
    this.parentElement.scrollTo(0, offset * this.parentElement.scrollHeight - this.parentElement.clientHeight / 2);
  })
}

this.state = {
  symbolSpans: new NestedSymbolSpans([]),
  selectedSymbol: {},
  renderedLines: [],
  lines: [],
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

function clearSelectedLine() {
  this.setState({ selectedSymbol: {} })
}

window.addEventListener('hashchange', () => {
  if (window.location.hash.startsWith("#L")) {
    this.setState({
      line: parseInt(window.location.hash.slice(2))
    })
  }
})

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
