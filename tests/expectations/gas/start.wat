(module
  (type (;0;) (func (param i32 i32)))
  (type (;1;) (func))
  (import "env" "ext_return" (func (;0;) (type 0)))
  (import "env" "memory" (memory (;0;) 1 1))
  (import "env" "out_of_gas_callback" (func (;1;) (type 1)))
  (func (;2;) (type 1)
    get_global 0
    i32.const 4
    i32.lt_u
    if  ;; label = @1
      call 1
    end
    get_global 0
    i32.const 4
    i32.sub
    set_global 0
    i32.const 8
    i32.const 4
    call 0
    unreachable)
  (func (;3;) (type 1))
  (global (;0;) (mut i32) (i32.const 0))
  (export "call" (func 3))
  (start 2)
  (data (i32.const 8) "\01\02\03\04"))
