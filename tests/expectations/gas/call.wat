(module
  (type (;0;) (func (param i32 i32) (result i32)))
  (type (;1;) (func))
  (import "env" "out_of_gas_callback" (func (;0;) (type 1)))
  (func (;1;) (type 0) (param i32 i32) (result i32)
    (local i32)
    get_global 0
    i32.const 5
    i32.lt_u
    if  ;; label = @1
      call 0
    end
    get_global 0
    i32.const 5
    i32.sub
    set_global 0
    get_local 0
    get_local 1
    call 2
    set_local 2
    get_local 2)
  (func (;2;) (type 0) (param i32 i32) (result i32)
    get_global 0
    i32.const 3
    i32.lt_u
    if  ;; label = @1
      call 0
    end
    get_global 0
    i32.const 3
    i32.sub
    set_global 0
    get_local 0
    get_local 1
    i32.add)
  (global (;0;) (mut i32) (i32.const 0)))
