(module
  (import "env" "foo" (func $foo))
  (func (param i32)
     get_local 0
     i32.const 0
     call $i32.add
     drop
  )
  (func $i32.add (export "i32.add") (param i32 i32) (result i32)
    get_local 0
	get_local 1
	i32.add
  )
  (table 10 anyfunc)

  ;; Refer all types of functions: imported, defined not exported and defined exported.
  (elem (i32.const 0) 0 1 2)
)
