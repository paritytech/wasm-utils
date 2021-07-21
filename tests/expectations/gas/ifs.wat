(module
  (type (;0;) (func (param i32) (result i32)))
  (func (;0;) (type 0) (param i32) (result i32)
    i32.const 2
    set_global 0
    i32.const 1
    if (result i32)  ;; label = @1
      i32.const 3
      set_global 0
      get_local 0
      i32.const 1
      i32.add
    else
      i32.const 2
      set_global 0
      get_local 0
      i32.popcnt
    end)
  (global (;0;) (mut i32) (i32.const 0)))
