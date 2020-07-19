class {{class_name}} extends HTMLElement {
    constructor() {
          super();
          this.shadow = this.attachShadow({mode:'open'});
          this.shadow.innerHTML = `{{html}}`;

          this.state = this.initialize();

          this.paragraph = document.createElement("p");
          this.paragraph.innerHTML = "my paragraph";
          this.shadow.appendChild(this.paragraph);
    }

    initialize() {
      {{javascript}}

      this.$$invalidate = (idx) => {
        switch(idx) {
          {{invalidations[]}}
          case {{idx}}:
            {{code}}
            break;
          {{/invalidations}}
          default:
            break;
        }
      }
    }
    
    $$invalidate() {}

    static get observedAttributes() {
      return [];
    }
}
customElements.define('{{component_name}}', {{class_name}});
