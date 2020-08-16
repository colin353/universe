const attributes = [ "code", "language", "line" ];

// Disables chrome's automatic scroll restoration logic, necessary to
// avoid conflicts w/ the automatic "jump-to-line" behaviour.
window.history.scrollRestoration = 'manual';

function base64Decode(str) {
    return decodeURIComponent(atob(str).split('').map(function(c) {
        return '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2);
    }).join(''));
}

function escapeHtml(unsafe) {
    return unsafe
         .replace(/&/g, "&amp;")
         .replace(/</g, "&lt;")
         .replace(/>/g, "&gt;")
         .replace(/"/g, "&quot;")
         .replace(/'/g, "&#039;");
}

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
    menuX: event.screenX,
    menuY: event.screenY,
    selection,
  })
  console.log("position: ", event.pageX, event.pageY);
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
      output[lineNumber].lineNumber = lineNumber;
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
};

function selectLine(event) {
  this.setState({
    line: parseInt(event.srcElement.innerText)
  })
}

// Syntax highlighting

class LanguageModel {
  constructor() {
    this.line = "";
    this.index = 0;
    this.keywords = new Set([]);
  }

  next() {
    const ch = this.peek();
    this.index++;
    return ch;
  }

  peek() {
    if(this.index >= this.line.length) {
      return '';
    } else {
      return this.line[this.index];
    }
  }

  isStringDelimiter(ch) {
    return ch == '\'' || ch == '"' || ch == '`'
  }

  isCommentCharacter(ch) {
    return false;
  }

  isInterminableComment(chs) {
    return false;
  }

  isAlphanumeric(ch) {
    const code = ch.charCodeAt(0);
    if(code >= 48 && code <= 57) return true;
    if(code >= 97 && code <= 122) return true;
    if(code >= 65 && code <= 90) return true;

    return false;
  }

  takeUntil(delimiter) {
    let acc = "";
    let ch = this.next();
    let escaped = false;
    let loops = 0;
    while(ch != '' && loops < 100) {
      loops++;
      if(ch == delimiter && !escaped) {
        break;
      }

      escaped = ch == '\'';
      acc += ch;

      ch = this.next();
    }

    return acc;
  }

  extractSyntax(line) {
    this.line = line;
    this.index = 0;

    let output = "";
    let ch = this.next();
    let loops = 0;
    let acc = "";
    let commentAcc = "";
    while(ch != '' && loops < 100) {
      loops++;
      if(this.isStringDelimiter(ch)) {
        const str = this.takeUntil(ch);
        output += `<span class='str'>${escapeHtml(ch + str + ch)}</span>`;
        ch = this.next();
        continue;
      }

      if(this.isCommentCharacter(ch)) {
        commentAcc += ch;
        if(this.isInterminableComment(commentAcc)) {
          let remainder = "";
          ch = this.next();
          while(ch != '') {
            ch = this.next();
            remainder += ch;
          }
          
          output += `<span class='comment'>${escapeHtml(commentAcc + remainder)}</span>`;
          return output;
        }
        continue;
      } else if(commentAcc.length > 0) {
        output += commentAcc;
        commentAcc = "";
      }

      if(this.isAlphanumeric(ch)) {
        acc += ch;
      } else if(acc.length > 0) {
        if (this.keywords.has(acc)) {
          output += `<span class='keyword'>${escapeHtml(acc)}</span>${escapeHtml(ch)}`;
        } else {
          output += escapeHtml(acc + ch);
        }
        acc = "";
      } else {
        output += escapeHtml(ch);
      }

      ch = this.next();
    }

    return output + escapeHtml(acc);
  }
}

class RustLanguageModel extends LanguageModel {
  constructor() {
    super();

    this.keywords = new Set([
      'as', 'break', 'const', 'continue', 'crate', 'else', 'enum',
      'extern', 'false', 'fn', 'for', 'if', 'impl', 'in', 'let',
      'loop', 'match', 'mod', 'move', 'mut', 'pub', 'ref', 'return',
      'self', 'Self', 'static', 'struct', 'super', 'trait', 'true',
      'type', 'unsafe', 'use', 'where', 'while', 'async', 'await',
      'dyn', 'u8', 'u16', 'u32', 'u64', 'u128', 'i8', 'i16', 'i32',
      'i64', 'i128', 'f16', 'f32', 'f64', 'f128'
    ]);
  }

  isCommentCharacter(ch) {
    return ch == '/' || ch == '*'
  }

  isInterminableComment(chs) {
    return chs == '//'
  }
}

class JavascriptLanguageModel extends LanguageModel {
  constructor() {
    super();

    this.keywords = new Set([
      'abstract','arguments','await','boolean',
      'break','byte','case','catch',
      'char', 'class', 'const', 'continue',
      'debugger','default', 'delete','do',
      'double','else','enum','eval',
      'export','extends','false', 'final',
      'finally', 'float', 'for', 'function', 
      'goto', 'if', 'implements', 'import', 
      'in', 'instanceof', 'int', 'interface', 
      'let', 'long', 'native', 'new', 
      'null', 'package', 'private', 'protected', 
      'public', 'return', 'short', 'static', 
      'super', 'switch', 'synchronized', 'this', 
      'throw', 'throws', 'transient', 'true', 
      'try', 'typeof', 'var', 'void', 
      'volatile', 'while', 'with', 'yield'
    ])
  }

  isCommentCharacter(ch) {
    return ch == '/' || ch == '*'
  }

  isInterminableComment(chs) {
    return chs == '//'
  }
}

class BazelLanguageModel extends LanguageModel {
  isCommentCharacter(ch) {
    return ch == '#'
  }

  isInterminableComment(chs) {
    return chs == '#'
  }
}

function getLanguageModel(lang) {
  const models = {
     'bazel': BazelLanguageModel,
     'rust': RustLanguageModel,
     'javascript': JavascriptLanguageModel,
  };

  if(models[lang]) {
    return new models[lang]();
  }
  return new LanguageModel();
};
