(module
  (type (;0;) (func (param i32 i32) (result i32)))
  (type (;1;) (func (param i32)))
  (import "env" "gas" (func (;0;) (type 1)))
  (func (;1;) (type 0) (param i32 i32) (result i32)
    (local i32)
    i32.const 5
    call 0
    get_local 0
    get_local 1
    call 2
    set_local 2
    get_local 2)
  (func (;2;) (type 0) (param i32 i32) (result i32)
    i32.const 3
    call 0
    get_local 0
    get_local 1
    i32.add))
