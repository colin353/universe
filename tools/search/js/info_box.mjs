import truncate from '../../../util/js/truncate.mjs';

const attributes = [ "symbol", "filename" ]

const updateInfoBox = async () => {
  const result = await fetch("/info?q=" + encodeURIComponent(this.state.name));
  const usages = await result.json();

  this.setState({usages: usages.filter(x => !x.startsWith(this.state.filename)).slice(0, 8)});
}

this.stateMappers = {
  _extractSymbolInfo: (symbol) => {
    const s = JSON.parse(symbol)
    this.setState({
      name: s.symbol,
      className: s?.structure?.symbol,
      rawType: s.type,
      symbolLine: s.start + 1,
      classLine: s?.structure?.start + 1,
    })
  },
  type: (rawType) => {
    if (!rawType) return '?';

    if (rawType == 'STRUCTURE') return 'cls';
    else if(rawType == 'FUNCTION') return 'fn';
    else if(rawType == 'TRAIT') return 'tr';

    return '?';
  },
  usages: (name) => {
    if(name) updateInfoBox();
    return [];
  },
  showUsages: (usages) => usages.length > 0,
}

this.state = {
    usages: [],
}
