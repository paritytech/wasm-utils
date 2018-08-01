(module
  (type (;0;) (func))
  (func (;0;) (type 0)
    i32.const 123
    drop)
  (func (;1;) (type 0)
    get_global 0
    i32.const 1
    i32.add
    set_global 0
    get_global 0
    i32.const 1024
    i32.gt_u
    if  ;; label = @1
      unreachable
    end
    call 0
    get_global 0
    i32.const 1
    i32.sub
    set_global 0)
  (global (;0;) (mut i32) (i32.const 0))
  (export "simple" (func 1)))
