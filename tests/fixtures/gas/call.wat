(module
	(func $add_locals (param $x i32) (param $y i32) (result i32)
		(local $t i32)

		get_local $x
		get_local $y
		call $add
		set_local $t

		get_local $t
	)

	(func $add (param $x i32) (param $y i32) (result i32)
		(i32.add
			(get_local $x)
			(get_local $y)
		)
	)
)
