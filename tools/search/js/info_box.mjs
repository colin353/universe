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
    if(!symbol) return;

    const s = JSON.parse(symbol)
    this.setState({
      name: s.symbol,
      className: s?.structure?.symbol,
      rawType: s.type,
      symbolLine: s.start + 1,
      classLine: s?.structure?.start + 1,
      start: s.start,
      end: s.end,
    })
  },
  _extractMatchInfo: (matches, start, end) => {
    const m = JSON.parse(matches)
    this.setState({
      totalLines: m.totalLines,
      matchingLines: m.matches.filter((x) => {
        return !start || !end || x > end || x < start
      }).map((x) => {
        return {
          lineNumber: x,
          percentage: 100 * x/m.totalLines,
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
}

store.addWatcher("codePadScrollInfo", (scrollInfo) => {
  if (!scrollInfo) return;

  this.setState({
    scrollTop: 100*scrollInfo.top,
    scrollHeight: 100*scrollInfo.height,
  })
})

// When clicked, jump the scroll position to that place
function onClickScope(e) {
  const rect = this.refs.scope.getBoundingClientRect();
  store.setState("codePadScrollOffsetWriteOnly", (e.clientX - rect.x) / rect.width);
}
