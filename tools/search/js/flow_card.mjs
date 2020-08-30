const attributes = [ "key", "title", "code", "filename", "comment", "temporary", ];

this.state = {
    key: "",
    code: "",
    filename: "",
    comment: "",
    isTemporary: false,
};

this.stateMappers = {
    isTemporary: (temporary) => {
        const isTemporary = temporary == "true";

        if (this.shadow.host) {
            if (isTemporary){
                this.shadow.host.style.border = "1px dashed #aaa";
                this.shadow.host.style.opacity = "0.5";
            } else {
                this.shadow.host.style.border = "1px solid #aaa";
                this.shadow.host.style.opacity = "1";
            }
        }

        return isTemporary;
    },
    hasCode: (code) => {
        return code ? "has-code" : "";
    },
};

function clickAdd() {
    this.dispatchEvent(new CustomEvent("add-card", {
        detail: {
            code: this.state.code,
            filename: this.state.filename,
            comment: this.refs.comment.innerHTML,
        },
    }));
    this.refs.comment.innerHTML = "";
}

function clickRemove() {
    this.dispatchEvent(new CustomEvent("remove-card", {
        detail: {
            key: this.state.key,
        },
    }));
}
