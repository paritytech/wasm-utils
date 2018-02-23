(module
  (import "env" "foo" (func $foo))
  (global (mut i32) (i32.const 1))
  (func $i32.add (export "i32.add") (param i32 i32) (result i32)
    get_local 0
	get_local 1
	i32.add
  )
  (func (param i32)
     get_local 0
     i32.const 0
     call $i32.add
     drop
  )
)
