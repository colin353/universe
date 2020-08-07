const attributes = [ "code" ];

this.stateMappers = {
  lines: (code) => {
    if (!code) return {}

    const output = {};
    let lineNumber = 0;
    let model = new LanguageModel();
    for(const line of atob(code).split("\n")) {
      output[lineNumber] = {};
      output[lineNumber].lineNumber = lineNumber;

      output[lineNumber].code = model.extractSyntax(line);

      lineNumber += 1;
    }
    return output;
  }
};

this.state = {
  lines: this.stateMappers.lines(this.state.code),
};

// Syntax highlighting

class LanguageModel {
  constructor() {
    this.line = "";
    this.index = 0;
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
    while(ch != '' && loops < 100) {
      loops++;
      if(this.isStringDelimiter(ch)) {
        const str = this.takeUntil(ch);
        output += `<span class='str'>${ch}${str}${ch}</span>`;
        ch = this.next();
        continue;
      }

      output += ch;
      ch = this.next();
    }

    return output;
  }
}
