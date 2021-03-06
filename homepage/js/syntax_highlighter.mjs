import { escapeHtml, } from "./utils.mjs";

const PreviousLineStates = {
  NONE: 0,
  COMMENT: 1,
  STRING: 2,
}

class LanguageModel {
    constructor() {
        this.line = "";
        this.index = 0;
        this.keywords = new Set([]);
        this.multiCharacterSingleQuoteStrings = true;  
        this.multiLineCommentTerminator = "*/";
        this.multiLineStringDelimiter = '"""';
        
        this.previousLineState = PreviousLineStates.NONE;
    }

    next() {
        const ch = this.peek();
        this.index++;
        return ch;
    }

    peek() {
        if(this.index >= this.line.length) {
            return "";
        } else {
            return this.line[this.index];
        }
    }

    isStringDelimiter(ch) {
        return ch == "'" || ch == "\"" || ch == "`";
    }

    isCommentCharacter() {
        return false;
    }

    isInterminableComment() {
        return false;
    }

    isMultiLineComment(chs) { 
        return chs == "/*";
    }

    isAlphanumericUnderscore(ch) {
        const code = ch.charCodeAt(0);
        if(code >= 48 && code <= 57) return true;
        if(code >= 97 && code <= 122) return true;
        if(code >= 65 && code <= 90) return true;
        if(ch == "_") return true;

        return false;
    }

    takeUntilCh(delimiter) {
        let acc = "";
        let ch = this.next();
        let escaped = false;
        let loops = 0;
        while(ch != "" && loops < 250) {
            loops++;
            acc += ch;
            if(ch == delimiter && !escaped) {
                break;
            }

            escaped = ch == "'";
            ch = this.next();
        }

        return acc;
    }

    takeUntil(delimeter) {
      let acc = "";
      let done = false;
      while(!done) {
        let segment = this.takeUntilCh(delimeter[0]);
        if (segment == "") break;
        acc += segment;

        done = true;
        for(const ch of delimeter.substr(1)) {
          const next = this.next();
          acc += next;
          if (next == "") {
            return acc;
          } else if(next != ch) {
            done = false;
            break;
          }
        }
      }

      return acc;
    }

    extractSyntax(line) {
        this.line = line;
        this.index = 0;
        let output = "";

        if(this.previousLineState === PreviousLineStates.COMMENT) {
          const comment = this.takeUntil(this.multiLineCommentTerminator);
          output += `<span class='comment'>${comment}</span>`;
          if(comment.endsWith(this.multiLineCommentTerminator)) {
            this.previousLineState = PreviousLineStates.NONE;
          }
        }

        let ch = this.next();
        let loops = 0;
        let acc = "";
        let commentAcc = "";
        while(ch != "" && loops < 512) {
            loops++;

            if(this.isMultiLineComment(commentAcc)) {
              const comment = this.takeUntil(this.multiLineCommentTerminator);
              output += `<span class='comment'>${commentAcc + comment}</span>`;
              commentAcc = "";
              if(comment.endsWith(this.multiLineCommentTerminator)) {
                this.previousLineState = PreviousLineStates.NONE;
              } else {
                break;
              }
            }

            if (this.previousLineState === PreviousLineStates.STRING) {
                this.index = 0;
                const strAcc = this.takeUntil(this.multiLineStringDelimiter);
                output += `<span class='str'>${strAcc}</span>`;
                if (strAcc.endsWith(this.multiLineStringDelimiter)) {
                    this.previousLineState = PreviousLineStates.NONE;
                } else {
                    break;
                }
            }

            if (this.isStringDelimiter(ch)) {
                if (this.line.substr(this.index-1).startsWith(this.multiLineStringDelimiter)) {
                    const strAcc = this.takeUntil(this.multiLineStringDelimiter)
                    if (strAcc.endsWith(this.multiLineStringDelimiter)) {
                        this.previousLineState = PreviousLineStates.NONE;
                    } else {
                        this.previousLineState = PreviousLineStates.STRING
                    }
                    output += `<span class='str'>${ch+strAcc}</span>`;
                    ch = this.next();
                    continue
                }

                if(ch == "'" && !this.multiCharacterSingleQuoteStrings) {
                    const str = this.next();
                    ch = this.next();
                    if (ch != "'") {
                        output += "'" + str;
                    } else {
                        output += `<span class='str'>${escapeHtml(ch + str)}</span>`;
                        ch = this.next();
                    }

                    continue;
                }
                const str = this.takeUntil(ch);
                output += `<span class='str'>${escapeHtml(ch + str)}</span>`;
                ch = this.next();
                continue;
            }

            if(this.isAlphanumericUnderscore(ch)) {
                acc += ch;
                ch = this.next();
                continue;
            } else if(acc.length > 0) {
                if (this.keywords.has(acc)) {
                    output += `<span class='keyword'>${escapeHtml(acc)}</span>${escapeHtml(ch)}`;
                } else if (!isNaN(acc)) {
                    output += `<span class='literal'>${escapeHtml(acc)}</span>${escapeHtml(ch)}`;
                } else {
                    output += escapeHtml(acc + ch);
                }
                acc = "";
                ch = this.next();
                continue;
            }

            if(this.isCommentCharacter(ch)) {
                commentAcc += ch;
                if(this.isInterminableComment(commentAcc)) {
                    let remainder = "";
                    while(ch != "") {
                        ch = this.next();
                        remainder += ch;
                    }
                    output += `<span class='comment'>${escapeHtml(commentAcc + remainder)}</span>`;
                    return output;
                } else if(this.isMultiLineComment(commentAcc)) {
                  this.previousLineState = PreviousLineStates.COMMENT;
                  ch = this.next();
                } else {
                  ch = this.next();
                }
                continue;
            } else if(commentAcc.length > 0) {
                output += commentAcc;
                commentAcc = "";
                ch = this.next();
                continue;
            }

            output += escapeHtml(ch);
            ch = this.next();
        }

        if (commentAcc.length > 0) {
          output += `<span class='comment'>${commentAcc}</span>`;
        }

        return output + escapeHtml(acc);
    }
}

class RustLanguageModel extends LanguageModel {
    constructor() {
        super();

        this.multiCharacterSingleQuoteStrings = false;
        this.keywords = new Set([
            "as", "break", "const", "continue", "crate", "else", "enum",
            "extern", "false", "fn", "for", "if", "impl", "in", "let",
            "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
            "self", "Self", "static", "struct", "super", "trait", "true",
            "type", "unsafe", "use", "where", "while", "async", "await",
            "dyn", "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32",
            "i64", "i128", "f16", "f32", "f64", "f128", "str", "String",
            "Result", "Option", "Some", "Ok", "None", "Err",
        ]);
    }

    isCommentCharacter(ch) {
        return ch == "/" || ch == "*" || ch == "#";
    }

    isInterminableComment(chs) {
        return chs == "//" || chs == "#";
    }
}

class CLanguageModel extends LanguageModel {
    constructor() {
        super();

        this.keywords = new Set([
            "alignas", "alignof", "and", "and_eq", "asm", "atomic_cancel",
            "atomic_commit", "atomic_noexcept", "aut", "bitand", "bitor",
            "bool", "break", "case", "catch", "char", "char8_t", "char16_t",
            "char32_t", "clas", "compl", "concept", "const", "consteval", "constexpr",
            "constinit", "const_cast", "continue", "co_await", "co_return", "co_yield", "decltype",
            "defaul", "delet", "do", "double", "dynamic_cast", "else", "enum",
            "explicit", "expor", "exter", "false", "float", "for", "friend",
            "goto", "if", "inlin", "int", "long", "mutabl", "namespace",
            "new", "noexcept", "not", "not_eq", "nullptr", "operator", "or",
            "or_eq", "private", "protected", "public", "reflexpr", "registe", "reinterpret_cast",
            "requires", "return", "short", "signed", "sizeo", "static", "static_assert",
            "static_cast", "struc", "switch", "synchronized", "template", "this", "thread_local",
            "throw", "true", "try", "typedef", "typeid", "typename", "union",
            "unsigned", "using", "virtual", "void", "volatile", "wchar_t", "while",
            "xor", "xor_eq",
        ]);
    }

    isCommentCharacter(ch) {
        return ch == "/" || ch == "*" || ch == "#";
    }

    isMultiLineComment(chs) { 
        return chs == "/*";
    }

    isInterminableComment(chs) {
        return chs == "//";
    }
}

class JavascriptLanguageModel extends LanguageModel {
    constructor() {
        super();

        this.keywords = new Set([
            "abstract","arguments","await","boolean",
            "break","byte","case","catch",
            "char", "class", "const", "continue",
            "debugger","default", "delete","do",
            "double","else","enum","eval",
            "export","extends","false", "final",
            "finally", "float", "for", "function", 
            "goto", "if", "implements", "import", 
            "in", "instanceof", "int", "interface", 
            "let", "long", "native", "new", 
            "null", "package", "private", "protected", 
            "public", "return", "short", "static", 
            "super", "switch", "synchronized", "this", 
            "throw", "throws", "transient", "true", 
            "try", "typeof", "var", "void", 
            "volatile", "while", "with", "yield",
        ]);
    }

    isCommentCharacter(ch) {
        return ch == "/" || ch == "*";
    }

    isMultiLineComment(chs) { 
        return chs == "/*";
    }

    isInterminableComment(chs) {
        return chs == "//";
    }
}

class BazelLanguageModel extends LanguageModel {
    isCommentCharacter(ch) {
        return ch == "#";
    }

    isMultiLineComment(chs) { 
        return chs == "/*";
    }

    isInterminableComment(chs) {
        return chs == "#";
    }
}

class ProtobufLanguageModel extends LanguageModel {
    constructor() {
        super();

        this.keywords = new Set([
            "optional", "required", "message", "import", "string", "repeated",
            "option", "true", "false", "enum", "int64", "int32", "int16",
            "uint64", "uint32", "uint16", "bool", "float", "service", "rpc",
            "returns",
        ]);
    }

    isCommentCharacter(ch) {
        return ch == "/" || ch == "*";
    }

    isMultiLineComment(chs) { 
        return chs == "/*";
    }

    isInterminableComment(chs) {
        return chs == "//";
    }
}

class PythonLanguageModel extends LanguageModel {
    constructor() {
        super();
        this.keywords = new Set([
            "and", "except", "lambda", "with", "as",
            "finally", "nonlocal", "while", "assert", "false",
            "None", "yield", "break", "for", "not",
            "class", "from", "or", "continue", "global",
            "pass", "def", "if", "raise", "del",
            "import", "return", "elif", "in", "True",
            "else", "is", "try",
        ]);
    }

    isCommentCharacter(ch) {
        return ch == "#";
    }

    isInterminableComment(chs) {
        return chs == "#";
    }

}

export default function getLanguageModel(lang) {
    const models = {
        "bazel": BazelLanguageModel,
        "rust": RustLanguageModel,
        "javascript": JavascriptLanguageModel,
        "c": CLanguageModel,
        "cpp": CLanguageModel,
        "proto": ProtobufLanguageModel,
        "python": PythonLanguageModel,
    };

    if(models[lang]) {
        return new models[lang]();
    }
    return new LanguageModel();
}
