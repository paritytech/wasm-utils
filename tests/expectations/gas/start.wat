(module
  (type (;0;) (func (param i32 i32)))
  (type (;1;) (func))
  (type (;2;) (func (param i32)))
  (import "env" "ext_return" (func (;0;) (type 0)))
  (import "env" "memory" (memory (;0;) 1 1))
  (import "env" "gas" (func (;1;) (type 2)))
  (func (;2;) (type 1)
    i32.const 4
    call 1
    i32.const 8
    i32.const 4
    call 0
    unreachable)
  (func (;3;) (type 1))
  (export "call" (func 3))
  (start 2)
  (data (;0;) (i32.const 8) "\01\02\03\04"))
