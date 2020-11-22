import getLanguageModel from "./syntax_highlighter.mjs";
import { base64Decode, } from "./utils.mjs";
import Store from "../../util/js/store.mjs";
const attributes = [ "left", "right", "language", "line", "startline", ];
const store = new Store();
// Disables chrome's automatic scroll restoration logic, necessary to
// avoid conflicts w/ the automatic "jump-to-line" behaviour.
window.history.scrollRestoration = "manual";
this.shadow.addEventListener("click", () => {
    this.setState({
        showMenu: false,
    });
});
const contextMenu = (event) => {
    const selection = window.getSelection().toString();
    if(selection == "") return;
    this.setState({
        showMenu: true,
        menuX: event.clientX,
        menuY: event.clientY,
        selection,
    });
    event.preventDefault();
};
const copy = (event) => {
    navigator.clipboard.writeText(this.state.selection);
};
const search = () => {
    this.dispatchEvent(new CustomEvent("search", {
        detail: { token: this.state.selection, },
    }));
};
const definition = () => {
    this.dispatchEvent(new CustomEvent("define", {
        detail: { token: this.state.selection, },
    }));
};
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
        const selectedLine = parseInt(line);
        const topLine = Math.max(1, selectedLine - 5);
        for(const line of parsedLines) {
            output[lineNumber] = {};
            output[lineNumber].lineNumber = this.state.startingLine + lineNumber;
            output[lineNumber].class = lineNumber == selectedLine ? "selected-line" : "";
            output[lineNumber].class += lineNumber == topLine ? " top-line" : "";
            output[lineNumber].code = line;
            lineNumber += 1;
        }
        return output;
    },
    _ensureLineVisible: (line) => {
        focusSelectedLine();
        this.dispatchEvent(new CustomEvent("lineSelected", {
            detail: { line, },
        }));
    },
    _updateLineInStore: (line, parsedLines) => {
        store.setState("currentLineNumber", line);
        store.setState("currentLine", parsedLines[line-1]);
    },
    startingLine: (startline) => {
        return parseInt(startline) || 0;
    },
};
const focusSelectedLine = () => {
    const elements = this.shadowRoot.querySelectorAll(".top-line");
    if (elements.length) {
        elements[0].scrollIntoView({block: "start",});
    }
};
this.componentDidMount = () => {
    setTimeout(focusSelectedLine, 10);
};
this.state = {
    parsedLines: this.stateMappers.parsedLines(this.state.code),
    lines: this.stateMappers.lines(this.state.code),
    startingLine: 0,
};
function selectLine(event) {
    this.setState({
        line: parseInt(event.srcElement.innerText),
    });
}
let jumpToLine = "";
let command = "";
// Keyboard shortcuts for jumping to lines/to top
window.addEventListener("keydown", (e) => {
    // Ignore keypresses within inputs
    if (e.srcElement != window.document.body) {
        return;
    }
    if (e.key == "g") {
        if (command.length > 0) {
            // Jump to top
            this.setState({
                line: 1,
            });
        } else {
            command = "g";
        }
    } else {
        command = "";
    }
    if (e.key == "d") {
        this.parentElement.scrollBy(0, 350);
    } else if (e.key == "u") {
        this.parentElement.scrollBy(0, -350);
    } else if (e.key == "j") {
        this.parentElement.scrollBy(0, 100);
    } else if (e.key == "k") {
        this.parentElement.scrollBy(0, -100);
    }
    if (e.key.length == 1 && e.key >= "0" && e.key <= "9") {
        jumpToLine += e.key;
    } else if (e.keyCode == 27) {
    // Detect escape + clear the jump-to-line
        jumpToLine = "";
    } else if (e.key == "G") {
        if (jumpToLine.length > 0) {
            const lineToJumpTo = Math.min(parseInt(jumpToLine), Object.keys(this.state.lines).length);
            this.setState({
                line: lineToJumpTo,
            });
        } else {
            this.setState({
                line: Object.keys(this.state.lines).length,
            });
        }
        jumpToLine = "";
    }
});
