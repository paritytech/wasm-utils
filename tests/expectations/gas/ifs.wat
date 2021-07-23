(module
  (type (;0;) (func (param i32) (result i32)))
  (type (;1;) (func))
  (import "env" "out_of_gas_callback" (func (;0;) (type 1)))
  (func (;1;) (type 0) (param i32) (result i32)
    get_global 0
    i32.const 2
    i32.lt_u
    if  ;; label = @1
      call 0
    end
    get_global 0
    i32.const 2
    i32.sub
    set_global 0
    i32.const 1
    if (result i32)  ;; label = @1
      get_global 0
      i32.const 3
      i32.lt_u
      if  ;; label = @2
        call 0
      end
      get_global 0
      i32.const 3
      i32.sub
      set_global 0
      get_local 0
      i32.const 1
      i32.add
    else
      get_global 0
      i32.const 2
      i32.lt_u
      if  ;; label = @2
        call 0
      end
      get_global 0
      i32.const 2
      i32.sub
      set_global 0
      get_local 0
      i32.popcnt
    end)
  (global (;0;) (mut i32) (i32.const 0)))
