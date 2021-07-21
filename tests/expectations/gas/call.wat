(module
  (type (;0;) (func (param i32 i32) (result i32)))
  (func (;0;) (type 0) (param i32 i32) (result i32)
    (local i32)
    i32.const 5
    set_global 0
    get_local 0
    get_local 1
    call 1
    set_local 2
    get_local 2)
  (func (;1;) (type 0) (param i32 i32) (result i32)
    i32.const 3
    set_global 0
    get_local 0
    get_local 1
    i32.add)
  (global (;0;) (mut i32) (i32.const 0)))
