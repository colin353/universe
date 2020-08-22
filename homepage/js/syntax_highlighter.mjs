import { escapeHtml, } from "./utils.mjs";

class LanguageModel {
    constructor() {
        this.line = "";
        this.index = 0;
        this.keywords = new Set([]);
        this.multiCharacterSingleQuoteStrings = true;
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

    isAlphanumericUnderscore(ch) {
        const code = ch.charCodeAt(0);
        if(code >= 48 && code <= 57) return true;
        if(code >= 97 && code <= 122) return true;
        if(code >= 65 && code <= 90) return true;
        if(ch == "_") return true;

        return false;
    }

    takeUntil(delimiter) {
        let acc = "";
        let ch = this.next();
        let escaped = false;
        let loops = 0;
        while(ch != "" && loops < 250) {
            loops++;
            if(ch == delimiter && !escaped) {
                break;
            }

            escaped = ch == "'";
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
        while(ch != "" && loops < 100) {
            loops++;
            if(this.isStringDelimiter(ch)) {
                if(ch == "'" && !this.multiCharacterSingleQuoteStrings) {
                    const str = this.next();
                    ch = this.next();
                    if (ch != "'") {
                        output += "'" + str;
                    } else {
                        output += `<span class='str'>${escapeHtml(ch + str + ch)}</span>`;
                        ch = this.next();
                    }

                    continue;
                }
                const str = this.takeUntil(ch);
                output += `<span class='str'>${escapeHtml(ch + str + ch)}</span>`;
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
                }
                ch = this.next();
                continue;
            } else if(commentAcc.length > 0) {
                output += commentAcc;
                commentAcc = "";
            }

            if(this.isAlphanumericUnderscore(ch)) {
                acc += ch;
            } else if(acc.length > 0) {
                if (this.keywords.has(acc)) {
                    output += `<span class='keyword'>${escapeHtml(acc)}</span>${escapeHtml(ch)}`;
                } else if (!isNaN(acc)) {
                    output += `<span class='literal'>${escapeHtml(acc)}</span>${escapeHtml(ch)}`;
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

        this.multiCharacterSingleQuoteStrings = false;
        this.keywords = new Set([
            "as", "break", "const", "continue", "crate", "else", "enum",
            "extern", "false", "fn", "for", "if", "impl", "in", "let",
            "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
            "self", "Self", "static", "struct", "super", "trait", "true",
            "type", "unsafe", "use", "where", "while", "async", "await",
            "dyn", "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32",
            "i64", "i128", "f16", "f32", "f64", "f128",
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

    isInterminableComment(chs) {
        return chs == "//";
    }
}

class BazelLanguageModel extends LanguageModel {
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
    };

    if(models[lang]) {
        return new models[lang]();
    }
    return new LanguageModel();
}
