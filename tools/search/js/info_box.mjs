import truncate from '../../../util/js/truncate.mjs';
import Store from '../../../util/js/store.mjs';
const attributes = [ "symbol", "filename", "matches" ]

const store = new Store();

const updateInfoBox = async () => {
  const result = await fetch("/info?q=" + encodeURIComponent(this.state.name));
  const usages = await result.json();

  this.setState({usages: usages.filter(x => !x.startsWith(this.state.filename)).slice(0, 8)});
}

this.stateMappers = {
  _extractSymbolInfo: (symbol) => {
    if (!symbol) return;

    this.setState({
      name: symbol.symbol,
      className: symbol?.structure?.symbol,
      rawType: symbol.type,
      symbolLine: symbol.start + 1,
      classLine: symbol?.structure?.start + 1,
      start: symbol.start,
      end: symbol.end,
    })
  },
  _extractMatchInfo: (matches, start, end) => {
    if (!matches.matches) return;

    this.setState({
      totalLines: matches.totalLines,
      matchingLines: matches.matches.filter((x) => {
        return !start || !end || x > end || x < start
      }).map((x) => {
        return {
          lineNumber: x,
          percentage: 100 * x/matches.totalLines,
        }
      })
    })
  },
  showMatches: (matchingLines) => matchingLines.length,
  type: (rawType) => {
    if (!rawType) return '?';

    if (rawType == 'STRUCTURE') return 'cls';
    else if(rawType == 'FUNCTION') return 'fn';
    else if(rawType == 'TRAIT') return 'tr';

    return '?';
  },
  rangeExtent: (start, end, totalLines) => {
    return 100*(end-start)/totalLines
  },
  rangeStart: (start, totalLines) => {
    return 100*(start)/totalLines
  },
  usages: (name) => {
    if(name) updateInfoBox();
    return [];
  },
  showUsages: (usages) => usages.length > 0,
}

this.state = {
    showMatches: false,
    matches: [],
    matchingLines: [],
    totalLines: 0,
    usages: [],
    scrollTop: 0,
    scrollHeight: 0,
    type: '?',
}

store.addWatcher("codePadScrollInfo", (scrollInfo) => {
  if (!scrollInfo) return;

  this.setState({
    scrollTop: 100*scrollInfo.top,
    scrollHeight: 100*scrollInfo.height,
  })
})

const findMatchInZone = (start, end, reverse) => {
  for(var i=0; i<this.state.matchingLines.length; i++) {
    let index = i;
    if (reverse) index = this.state.matchingLines.length - 1 - i;

    const item = this.state.matchingLines[index];

    if (item.percentage < start || item.percentage > end) {
      continue;
    }

    return item;
  }
}

const jumpToPrevReference = () => {
  let match = findMatchInZone(0, (this.state.scrollTop + this.state.scrollHeight/2) - 2, true);
  if (match) {
    store.setState("codePadScrollOffsetWriteOnly", match.percentage/100);
    return;
  }

  match = findMatchInZone(0, 100, true);
  if (match) {
    store.setState("codePadScrollOffsetWriteOnly", match.percentage/100);
    return;
  }
}

function jumpToNextReference() {
  debugger
  let match = findMatchInZone((this.state.scrollTop + this.state.scrollHeight/2) + 2, 100, false);
  if (match) {
    store.setState("codePadScrollOffsetWriteOnly", match.percentage/100);
    return;
  }

  match = findMatchInZone(0, 100, false);
  if (match) {
    store.setState("codePadScrollOffsetWriteOnly", match.percentage/100);
    return;
  }
}

// When clicked, jump the scroll position to that place
function onClickScope(e) {
  const rect = this.refs.scope.getBoundingClientRect();
  store.setState("codePadScrollOffsetWriteOnly", (e.clientX - rect.x) / rect.width);
}
