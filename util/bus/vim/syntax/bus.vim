syntax match BusNumber "\v<\d+>"
syn keyword BusType u64 u32 u16 u8 bool string float bytes
syn keyword BusKeyword repeated message service enum rpc

syntax region BusComment start="//" end="$"

highlight default link BusKeyword Keyword
highlight default link BusType Boolean
highlight default link BusNumber Number
highlight default link BusString String
highlight default link BusComment Comment
highlight default link BusValue Boolean

