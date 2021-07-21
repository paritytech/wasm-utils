(module
  (type (;0;) (func (param i32 i32)))
  (type (;1;) (func))
  (import "env" "ext_return" (func (;0;) (type 0)))
  (import "env" "memory" (memory (;0;) 1 1))
  (func (;1;) (type 1)
    i32.const 4
    set_global 0
    i32.const 8
    i32.const 4
    call 0
    unreachable)
  (func (;2;) (type 1))
  (global (;0;) (mut i32) (i32.const 0))
  (export "call" (func 2))
  (start 1)
  (data (i32.const 8) "\01\02\03\04"))
