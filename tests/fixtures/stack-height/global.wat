(module
  (import "env" "foo" (func $foo))

  ;; Declare a global.
  (global $counter (mut i32) (i32.const 1))

  (func $i32.add (export "i32.add") (param i32 i32) (result i32)
    get_local 0
	get_local 1
	i32.add
  )
  (func (param $arg i32)
     (local $tmp i32)

     get_global 0
     i32.const 1
     i32.add
	 tee_local $tmp
     set_global $counter

     get_local $tmp
     get_local $arg
     call $i32.add
     drop
  )
)
