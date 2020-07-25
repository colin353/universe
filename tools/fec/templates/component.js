class {{class_name}} extends HTMLElement {
    constructor() {
          super();
          this.shadow = this.attachShadow({mode:'open'});
          this.shadow.innerHTML = `<style>{{css}}</style>`

          this.props = [];
          this.state = [];
          this.initialize();
    }

    initialize() {
      this.setState = (newState) => {
        for (const k of Object.keys(newState)) {
          this.state[k] = newState[k];
          this.trigger_rerenders("this.state." + k);
        }
      }

      {{javascript}}

      {{html}}

      this.render = (keys) => {
        for (const k of keys) {
          switch(k) {
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
    }

    render(keys) {}

    trigger_rerenders(key) {
      switch(key) {
        {{symbols[]}}
        case '{{name}}':
           this.render([{{mutations}}]);
           break;
        {{/symbols}}
        default:
          break;
      }
    }

    
    static get observedAttributes() {
      return [];
    }
}
customElements.define('{{component_name}}', {{class_name}});
