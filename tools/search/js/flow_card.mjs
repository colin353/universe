const attributes = [ "title", "code", "filename", "comment", "temporary" ];

this.state = {
  code: "default code",
  filename: "default filename",
  comment: "default comment",
  isTemporary: false,
}

this.stateMappers = {
  isTemporary: (temporary) => {
    const isTemporary = temporary == "true";

    if (this.shadow.host) {
      if (isTemporary){
        this.shadow.host.style.border = "1px dashed #aaa";
      } else {
        this.shadow.host.style.border = "1px solid #aaa";
      }
    }

    return isTemporary
  }
}

function clickAdd() {
  this.dispatchEvent(new CustomEvent('add-card', {
    detail: {
      code: this.state.code,
      filename: this.state.filename,
      comment: this.refs.comment.innerHTML,
    }
  }))
  this.refs.comment.innerHTML = "";
}
