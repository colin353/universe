import debounce from '../../../util/js/debounce.mjs';

const attributes = ["query"];

this.state = {
  baseQuery: '',
  entitySuggestions: [],
  suggestions: [],
  suggestionMap: {},
  selectedIndex: -1,
}

this.stateMappers = {
  showSuggestions: (suggestions) => suggestions.length > 0,
  suggestionMap: (suggestions, selectedIndex) => {
    const output = {};
    let key = 0;
    for(const suggestion of suggestions) {
      output[key] = {
        suggestion: suggestion.name,
        left: suggestion.file_type ? `${suggestion.file_type} ${suggestion.kind}` : '',
        filename: suggestion.file || '',
        line_number: suggestion.line_number,
        className: key == selectedIndex ? 'selected' : '',
      }
      key += 1;
    }
    return output;
  },
  _updateSuggestions: (selectedIndex) => {
    if(this.state.selectedIndex == -1) {
      this.refs.search_input.value = this.state.baseQuery;
    } else {
      this.refs.search_input.value = this.state.suggestions[this.state.selectedIndex].name;
    }

    setTimeout(() => {
      this.refs.search_input.focus();
      const caretPos = this.refs.search_input.value.length;
      this.refs.search_input.setSelectionRange(caretPos, caretPos);
    });
  }
};

async function getSuggestions() {
  const query = this.refs.search_input.value;
  if (query.length < 4) {
    return this.setState({ suggestions: [] });
  }
  const result = await fetch("/suggest?q=" + encodeURIComponent(query));
  const suggestions = await result.json();
  this.setState({ suggestions });
}

const getSuggestionsDebounced = debounce(getSuggestions.bind(this), 250);

function handleKeyPress(e) {
  if(e.key === 'Enter') {
      let usedEntityInfo = false;

      if (this.state.selectedIndex !== -1) {
        let filename = this.state.suggestionMap[this.state.selectedIndex].filename
        if (filename) {
          let destination = `/${filename}`
          let line_number = this.state.suggestionMap[this.state.selectedIndex].line_number
          if (line_number) {
            destination += `#L${line_number + 1}`
          }
          window.location.href = destination;
          usedEntityInfo = true;

          this.setState({suggestedIndex: -1, suggestions: []});
        }
      } 

      if (!usedEntityInfo) {
        window.location.href = '/?q=' + encodeURIComponent(this.refs.search_input.value);
      }
      return e.preventDefault();
  }

  if(e.keyCode == 40) {
    let baseQuery = this.state.baseQuery;
    if (this.state.selectedIndex == -1) {
      baseQuery = this.refs.search_input.value;
    }

    this.setState({
      selectedIndex: ((this.state.selectedIndex + 2) % (this.state.suggestions.length + 1)) - 1,
      baseQuery,
    })
  } else if(e.keyCode == 38) {
    if(this.state.selectedIndex == -1) {
      this.setState({selectedIndex: this.state.suggestions.length-1 });
    } else {
      this.setState({
        selectedIndex: this.state.selectedIndex - 1,
      })
    }

  } else {
    getSuggestionsDebounced();
  }
}

function handleClick(key) {
  this.setState({
    selectedIndex: key
  })
}

function handleBlur() {
  setTimeout(() => {
    this.setState({
      suggestions: [],
    })
  }, 200);
}

this.componentDidMount = () => {
  if (this.state.query) {
    this.refs.search_input.value = this.state.query;
  }

  // Only enable autofocus when not on a detail page
  if (window.location.pathname.length <= 1) {
    this.refs.search_input.focus();
  }
}
