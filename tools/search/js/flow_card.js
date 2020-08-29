const attributes = [ "title", "code", "filename", "comment", "temporary" ];

this.state = {
  title: "default title",
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
