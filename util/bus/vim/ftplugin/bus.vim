setlocal commentstring=//\ %s
setlocal shiftwidth=4 softtabstop=2 expandtab
filetype plugin indent on

function! bus#DeleteLines(start, end) abort
    silent! execute a:start . ',' . a:end . 'delete _'
endfunction

function! bus#PreWrite()
  if !filereadable(expand("%@"))
    return
  endif

  let l:view = winsaveview()
  let l:stderr_tmpname = tempname()
  call writefile([], l:stderr_tmpname)

  let l:command = "busfmt 2> " .l:stderr_tmpname
  let l:buffer = getline(1, '$')
  silent let out = systemlist(l:command, l:buffer)

  let l:stderr = readfile(l:stderr_tmpname)
  call delete(l:stderr_tmpname)

  try | silent undojoin | catch | endtry

  if len(l:stderr) == 0 
    call setline(1, l:out)
    call bus#DeleteLines(len(l:out), line('$'))
  endif

  call winrestview(l:view)
endfunction

autocmd BufWritePre *.bus silent! call bus#PreWrite()
