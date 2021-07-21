(module
  (type (;0;) (func (result i32)))
  (func (;0;) (type 0) (result i32)
    (local i32 i32)
    i32.const 13
    set_global 0
    block  ;; label = @1
      i32.const 0
      set_local 0
      i32.const 1
      set_local 1
      get_local 0
      get_local 1
      tee_local 0
      i32.add
      set_local 1
      i32.const 1
      br_if 0 (;@1;)
      i32.const 5
      set_global 0
      get_local 0
      get_local 1
      tee_local 0
      i32.add
      set_local 1
    end
    get_local 1)
  (global (;0;) (mut i32) (i32.const 0)))
