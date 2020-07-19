class {{class_name}} extends HTMLElement {
    constructor() {
          super();
          this.shadow = this.attachShadow({mode:'open'});
          this.shadow.innerHTML = `<style>{{css}}</style>`
          this.state = this.initialize();
    }

    initialize() {
      {{javascript}}

      {{html}}

      this.$$invalidate = (idx) => {
        switch(idx) {
          {{mutations[]}}
          case {{idx}}:
            {{code}}
            break;
          {{/mutations}}
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
