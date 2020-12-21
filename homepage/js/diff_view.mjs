import getLanguageModel from "./syntax_highlighter.mjs";
import { base64Decode, } from "./utils.mjs";
import Store from "../../util/js/store.mjs";
import { diff } from './diff.mjs'
const attributes = [ "left", "right", "language", "line", "startline", ];
const store = new Store();

const renderLines = (parsed) => {
    if(!parsed) return [];
    let output = [];
    for(const line of parsed) {
        const renderedLine = {};
        const lineNumber = line.lineNumber;
        
        if (line.type == BlockType.PLACEHOLDER) {
          renderedLine.lineNumber = "-";
          renderedLine.code = "";
        } else {
          renderedLine.lineNumber = line.lineNumber;
          renderedLine.code = line.line;
        }
        renderedLine.class = "diff-" + line.type;
        output.push(renderedLine);
    }
    return output;
}

const BlockType = {
  SAME: 'same',
  DELETED: 'deleted',
  ADDED: 'added',
  PLACEHOLDER: 'placeholder',
}

this.stateMappers = {
    diffs: (left, right) => {
      if (!left) {
        left = [];
      } else {
        left = base64Decode(left).split("\n")
      }

      if (!right) {
        right = [];
      } else {
        right = base64Decode(right).split("\n")
      }
      return diff(left, right)
    },
    parsed: (diffs, language) => {
        let parsedLines = { left: [], right: [] };
        if (!diffs) return parsedLines;

        let model = getLanguageModel(language);
        let leftLineNumber = 1;
        let rightLineNumber = 1;
        let leftAllPlaceholder = true;
        let rightAllPlaceholder = true;

        for(const chunk of diffs) {
            if (chunk.common) {
              for (const ch of chunk.common) {
                const line = model.extractSyntax(ch);
                parsedLines.left.push({type: BlockType.SAME, line, lineNumber: leftLineNumber});
                parsedLines.right.push({type: BlockType.SAME, line, lineNumber: rightLineNumber});
                leftLineNumber += 1;
                rightLineNumber += 1;
                rightAllPlaceholder = false;
                leftAllPlaceholder = false;
              }
            } else {
              for (const ch of chunk.file1) {
                const line = model.extractSyntax(ch);
                parsedLines.left.push({type: BlockType.DELETED, line, lineNumber: leftLineNumber})
                leftLineNumber += 1;
                leftAllPlaceholder = false;
              }

              for (const ch of chunk.file2) {
                const line = model.extractSyntax(ch);
                parsedLines.right.push({type: BlockType.ADDED, line, lineNumber: rightLineNumber})
                rightLineNumber += 1;
                rightAllPlaceholder = false;
              }


              for(var i = 0; i < (chunk.file1.length - chunk.file2.length); i++) {
                parsedLines.right.push({type: BlockType.PLACEHOLDER})
              }

              for(var i = 0; i < (chunk.file2.length - chunk.file1.length); i++) {
                parsedLines.left.push({type: BlockType.PLACEHOLDER})
              }
            }
        }

        if (leftAllPlaceholder) {
          parsedLines.left = [];
        }

        if (rightAllPlaceholder) {
          parsedLines.right = [];
        }

        return parsedLines;
    },
    leftLines: (parsed) => renderLines(parsed.left),
    rightLines: (parsed) => renderLines(parsed.right),
    hideLeft: (left) => left.length == 0 ? "hidden" : "",
    hideRight: (right) => right.length == 0 ? "hidden" : "",
};

this.state = {
  diffs: [],
  parsed: { left: [], right: [] },
  leftLines: [],
  rightLines: [],
  hideLeft: true,
  hideRight: true,
};

this.setState({
    diffs: this.stateMappers.diffs(this.state.left, this.state.right),
})

