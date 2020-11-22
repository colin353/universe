const Iterator = {
    EOF: -1,
};

const DiffType= {
    SAME : "SAME",
    ADDED : "ADDED",
    REMOVED: "REMOVED",
};

export class Difference {
    constructor(type, content) {
        this.type = type;
        this.content = content;
    }

    static added(content) {
        return new Difference(DiffType.ADDED, content);
    }

    static removed(content) {
        return new Difference(DiffType.REMOVED, content);
    }

    static same(content) {
        return new Difference(DiffType.SAME, content);
    }
}

class LinesIterator {

    constructor(data) {
        this.lines = data.split("\n");
        this.index = 0;
    }

    peek() {
        if (this.index >= this.lines.length) {
            return Iterator.EOF;
        }

        return this.lines[this.index];
    }

    next() {
        if(this.index >= this.lines.length) {
            return Iterator.EOF;
        }

        this.index += 1;
        return this.lines[this.index-1];
    }
}

export default function calculateDiff(leftInput, rightInput) {
    const leftLines = new LinesIterator(leftInput);
    const rightLines = new LinesIterator(rightInput);

    let output = [];

    let left = leftLines.next();
    let right = rightLines.next();

    while(right !== Iterator.EOF && left !== Iterator.EOF) {
        if (right === Iterator.EOF) {
            while(left !== Iterator.EOF) {
                output.push(Difference.removed(left));
                left = leftLines.next();
            }
            break;
        }

        if (left === Iterator.EOF) {
            while(right !== Iterator.EOF) {
                output.push(Difference.added(right));
                right = rightLines.next();
            }
            break;
        }

        if (left == right) {
            output.push(Difference.same(left));
        } else {
            output.push(Difference.removed(left));
            output.push(Difference.added(right));
        }

        left = leftLines.next();
        right = rightLines.next();
    }

    return output;
}
