setlocal indentexpr=GetCCLIndent()
setlocal indentkeys=0{,0},0),0[,0],!^F,o,O,e

" Check if line 'lnum' has more opening brackets than closing ones.
function s:LineHasOpeningBrackets(lnum)
  let open_0 = 0
  let open_2 = 0
  let open_4 = 0
  let line = getline(a:lnum)
  let pos = match(line, '[][(){}]', 0)
  while pos != -1
    let idx = stridx('(){}[]', line[pos])
    if idx % 2 == 0
      let open_{idx} = open_{idx} + 1
    else
      let open_{idx - 1} = open_{idx - 1} - 1
    endif
    let pos = match(line, '[][(){}]', pos + 1)
  endwhile
  return (open_0 > 0) . (open_2 > 0) . (open_4 > 0)
endfunction

function GetCCLIndent()
  " 3.1. Setup {{{2
  " ----------

  " Set up variables for restoring position in file.  Could use v:lnum here.
  let vcol = col('.')

  " 3.2. Work on the current line {{{2
  " -----------------------------

  " Get the current line.
  let line = getline(v:lnum)
  let ind = -1

  " If we got a closing bracket on an empty line, find its match and indent
  " according to it.
  let col = matchend(line, '^\s*[]}]')

  if col > 0
    call cursor(v:lnum, col)
    let bs = strpart('{}[]', stridx('}]', line[col - 1]) * 2, 2)

    let pairstart = escape(bs[0], '[')
    let pairend = escape(bs[1], ']')
    let pairline = searchpair(pairstart, '', pairend, 'bW')

    if pairline > 0
      let ind = indent(pairline)
    else
      let ind = virtcol('.') - 1
    endif

    return ind
  endif

  " 3.3. Work on the previous line. {{{2
  " -------------------------------

  let lnum = prevnonblank(v:lnum - 1)

  if lnum == 0
    return 0
  endif

  " Set up variables for current line.
  let line = getline(lnum)
  let ind = indent(lnum)

  " If the previous line contained an opening bracket, and we are still in it,
  " add indent depending on the bracket type.
  if line =~ '[[({]'
    let counts = s:LineHasOpeningBrackets(lnum)
    if counts[0] == '1' || counts[1] == '1' || counts[2] == '1'
      if exists('*shiftwidth')
        return ind + shiftwidth()
      else
        return ind + &sw
      endif
    else
      call cursor(v:lnum, vcol)
    end
  endif

  " }}}2

  return ind
endfunction
