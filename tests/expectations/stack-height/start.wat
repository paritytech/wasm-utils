(module
  (type (;0;) (func (param i32 i32)))
  (type (;1;) (func))
  (import "env" "ext_return" (func (;0;) (type 0)))
  (import "env" "memory" (memory (;0;) 1 1))
  (func (;1;) (type 1)
    (local i32))
  (func (;2;) (type 1))
  (func (;3;) (type 1)
    global.get 0
    i32.const 1
    i32.add
    global.set 0
    global.get 0
    i32.const 1024
    i32.gt_u
    if  ;; label = @1
      unreachable
    end
    call 1
    global.get 0
    i32.const 1
    i32.sub
    global.set 0)
  (global (;0;) (mut i32) (i32.const 0))
  (export "exported_start" (func 3))
  (export "call" (func 2))
  (start 3))
