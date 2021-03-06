{{js_imports}}

class {{class_name}} extends HTMLElement {
    constructor() {
          super();
          this.componentDidMount = () => {};

          this.shadow = this.attachShadow({mode:'open'});
          this.shadow.innerHTML = `<style>{{css}}</style>`

          this.props = [{{props}}];
          this.state = {};
          this.stateMappers = {};

          this.__mappings = {};
          this.__selectors = {};

          this.refs = {};
          this.currentlySettingState = false;
          this.pendingStateDifferences = new Set([]);

          for (const k of this.props) {
            this.state[k] = this.getAttribute(k);
          }
          for (const k of Object.keys(this.__rawAttributes || {})) {
            this.state[k] = this.__rawAttributes[k]
          }

          this.initialize();
    }

    connectedCallback() {
      this.componentDidMount();
    }

    setRawAttribute(name, value) {
      this.setState({
        [name]: value,
      })
    }

    setInitialState(state) {
      for (const key of Object.keys(state)) {
        if (!(key in this.state)) this.state[key] = state[key];
      }
    }

    attributeChangedCallback(name, oldValue, newValue) {
      this.setState({
        [name]: newValue,
      })
    }

    _hasStateChanged(oldState, newState) {
      if (typeof oldState !== typeof newState) return true;

      // Not smart enough to introspect into objects so just assume different
      if (typeof oldState === "object") return true;

      return oldState != newState
    }

    renderStateDifferences() {
      const unrenderedStateDifferences = new Set(this.pendingStateDifferences);
      let iterations = 0;
      while(this.pendingStateDifferences.size > 0) {
        iterations += 1;
        if (iterations > 1024) {
          console.error("setState trigger depth exceeded!");
          break;
        }

        const unmappedStateDifferences = this.pendingStateDifferences
        this.pendingStateDifferences = new Set()
        this.triggerAllMappings(unmappedStateDifferences);

        for (const k of unmappedStateDifferences) {
          unrenderedStateDifferences.add(k)
        }
      }

      for (const k of unrenderedStateDifferences) {
        this.triggerRenders("this.state." + k);
      }

      this.currentlySettingState = false;
    }

    initialize() {
      this.setState = (newState) => {
          let isOuter = !this.currentlySettingState;
          if (isOuter) {
            this.currentlySettingState = true;
          }

          for (const k of Object.keys(newState)) {
            if (this._hasStateChanged(this.state[k], newState[k])) {
              this.state[k] = newState[k];
              this.pendingStateDifferences.add(k);
            }
          }

          if (!isOuter) {
            return
          }

          setTimeout(this.renderStateDifferences.bind(this), 16);
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

      {{refs[]}}
      this.refs.{{refname}} = {{tagname}};
      {{/refs}}

      this.initializeStateMappers();
    }

    initializeStateMappers() {
      for (const k of Object.keys(this.stateMappers)) {
        const args = getArguments(this.stateMappers[k]);
        for(const arg of args) {
          if(!this.__mappings[arg]) {
            this.__mappings[arg] = [];
          }
          this.__mappings[arg].push(k);
        }

        this.__selectors[k] = (state) => {
          let output = [];
          for(const arg of args) {
            output.push(state[arg]);
          }
          return output;
        }
      }

      this.triggerAllMappings(Object.keys(this.__mappings))
    }

    render(keys) {}

    triggerAllMappings(keys) {
      let allMappings = new Set([])

      for(const key of keys) {
        if (!this.__mappings[key]) continue;

        for(const m of this.__mappings[key]) {
          allMappings.add(m);
        }
      }
      
      for (const m of allMappings) {
        this.setState({
          [m]: this.stateMappers[m](...this.__selectors[m](this.state))
        })
      }
    }

    triggerRenders(key) {
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

    setElementAttribute(element, key, value) {
      if (typeof value === "string") {
        element.setAttribute(key, value);
      } else if (typeof value === "boolean") {
        if (value) {
          element.setAttribute(key, true);
        } else {
          element.removeAttribute(key);
        }
      } else {
        element.__rawAttributes[key] = value;
        if(element.setRawAttribute) element.setRawAttribute(key, value);
      }
    }
    
    static get observedAttributes() {
      return [{{props}}]
    }
}

customElements.define('{{component_name}}', {{class_name}});

function getArguments(func) {
    const ARROW = true;
    const FUNC_ARGS = ARROW ? /^(function)?\s*[^\(]*\(\s*([^\)]*)\)/m : /^(function)\s*[^\(]*\(\s*([^\)]*)\)/m;
    const FUNC_ARG_SPLIT = /,/;
    const FUNC_ARG = /^\s*(_?)(.+?)\1\s*$/;
    const STRIP_COMMENTS = /((\/\/.*$)|(\/\*[\s\S]*?\*\/))/mg;

    return ((func || '').toString().replace(STRIP_COMMENTS, '').match(FUNC_ARGS) || ['', '', ''])[2]
        .split(FUNC_ARG_SPLIT)
        .map(function(arg) {
            return arg.replace(FUNC_ARG, function(all, underscore, name) {
                return name.split('=')[0].trim();
            });
        })
        .filter(String);
}

