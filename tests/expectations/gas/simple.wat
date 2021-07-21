(module
  (type (;0;) (func))
  (func (;0;) (type 0)
    i32.const 2
    set_global 0
    i32.const 1
    if  ;; label = @1
      i32.const 1
      set_global 0
      loop  ;; label = @2
        i32.const 2
        set_global 0
        i32.const 123
        drop
      end
    end)
  (func (;1;) (type 0)
    i32.const 1
    set_global 0
    block  ;; label = @1
    end)
  (global (;0;) (mut i32) (i32.const 0))
  (export "simple" (func 0)))
