syntax match CCLNumber "\v<\d+>"
syntax match CCLNumber "\v<\d+.\d+>"
syn keyword  CCLValue null true false


syntax region CCLString start=/"/ skip=/\\"/ end=/"/ oneline contains=CCLInterpolatedWrapper
syntax region CCLInterpolatedWrapper start="\v\\\(\s*" end="\v\s*\)" contained containedin=CCLString contains=CCLInterpolatedString
syntax match CCLInterpolatedString "\v\w+(\(\))?" contained containedin=CCLInterpolatedWrapper

syntax region CCLComment start="//" end="$"

syntax match CCLIdentifier contains=CCLIdentifierPrime "\%([^[:cntrl:][:space:][:punct:][:digit:]]\|_\)\%([^[:cntrl:][:punct:][:space:]]\|_\)*" display contained

highlight default link CCLNumber Number
highlight default link CCLString String
highlight default link CCLComment Comment
highlight default link CCLInterpolatedWrapper Delimiter
highlight default link CCLIdentifierPrime CCLIdentifier
highlight default link CCLValue Boolean

