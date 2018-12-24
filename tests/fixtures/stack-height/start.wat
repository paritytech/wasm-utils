(module
	(import "env" "ext_return" (func $ext_return (param i32 i32)))
	(import "env" "memory" (memory 1 1))

	(start $start)
	(func $start (export "exported_start")
		(local i32)
	)
	(func (export "call")
	)
)
