import debounce from '../../../util/js/debounce.mjs';

this.state = {
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
        suggestion,
        className: key == selectedIndex ? 'selected' : '',
      }
      key += 1;
    }
    console.log("saved: ", output);
    return output;
  }
};

async function getSuggestions() {
  const query = this.refs.search_input.value;
  const result = await fetch("/suggest?q=" + encodeURIComponent(query));
  const suggestions = await result.json();
  this.setState({ suggestions });
}

const getSuggestionsDebounced = debounce(getSuggestions.bind(this), 250);

function handleKeyPress(e) {
  if(e.key === 'Enter') {
      window.location.href = '/?q=' + encodeURIComponent(this.refs.search_input.value);
      return event.preventDefault();
  }

  if(e.keyCode == 40) {
    this.setState({
      selectedIndex: this.state.selectedIndex + 1,
    })
  } else if(e.keyCode == 38) {
    this.setState({
      selectedIndex: this.state.selectedIndex - 1,
    })
  }

  getSuggestionsDebounced();
}
